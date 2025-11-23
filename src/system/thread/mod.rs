mod stack;
mod registry;
mod spawn;

use stack::ThreadStack;
pub use registry::{get_thread_stats, register_thread, cleanup_threads};
pub use spawn::spawn_thread;

use core::sync::atomic::{AtomicU16, AtomicI32, Ordering};
use crate::syscalls::{nanosleep, close};
use crate::io::print;

static HTTP_STACK: ThreadStack = ThreadStack::new_large();
static HTTP_PORT: AtomicU16 = AtomicU16::new(0);

const MAX_WS_THREADS: usize = 16;
static WS_STACKS: [ThreadStack; MAX_WS_THREADS] = [const { ThreadStack::new() }; MAX_WS_THREADS];
static WS_CLIENT_FDS: [AtomicI32; MAX_WS_THREADS] = [const { AtomicI32::new(0) }; MAX_WS_THREADS];

fn http_server_func() -> ! {
    use crate::network::start_http_server;
    use crate::syscalls::sys_exit;
    
    nanosleep(0, 100_000_000);
    
    let port = HTTP_PORT.load(Ordering::Acquire);
    start_http_server(port);
    
    print(b"[ERROR] HTTP server exited\n");
    sys_exit(1);
}

pub fn start_http_server_thread(port: u16) {
    HTTP_PORT.store(port, Ordering::Release);
    
    if !HTTP_STACK.allocate() {
        print(b"[ERROR] Failed to allocate HTTP stack\n");
        return;
    }
    
    match spawn_thread(&HTTP_STACK, http_server_func) {
        Ok(tid) => {
            register_thread(tid);
            print(b"[INFO] HTTP server thread started\n");
        }
        Err(err) => {
            print(b"[ERROR] Failed to start HTTP server thread: ");
            print(err.as_bytes());
            print(b"\n");
        }
    }
}

static WS_CLIENT_INDICES: [AtomicI32; MAX_WS_THREADS] = [const { AtomicI32::new(-1) }; MAX_WS_THREADS];

fn websocket_handler_func() -> ! {
    use crate::syscalls::sys_exit;
    
    let mut client_fd = 0;
    let mut client_idx = 0;
    
    for i in 0..MAX_WS_THREADS {
        let idx = WS_CLIENT_INDICES[i].load(Ordering::Acquire);
        if idx >= 0 {
            client_idx = idx as usize;
            client_fd = WS_CLIENT_FDS[i].load(Ordering::Acquire);
            WS_CLIENT_INDICES[i].store(-1, Ordering::Release); // Clear marker
            break;
        }
    }
    
    if client_fd <= 0 {
        print(b"[WS Thread] No client FD\n");
        sys_exit(1);
    }
    
    use crate::io::{print_number};
    print(b"[WS Thread] Starting frame loop (client_idx=");
    print_number(client_idx as i64);
    print(b", fd=");
    print_number(client_fd as i64);
    print(b")\n");
    
    use crate::network::websocket::websocket_frame_loop_with_index;
    websocket_frame_loop_with_index(client_fd, client_idx);
    
    print(b"[WS Thread] Closing connection\n");
    close(client_fd);
    
    WS_CLIENT_FDS[client_idx].store(0, Ordering::Release);
    
    sys_exit(0);
}

pub fn start_websocket_thread(client_fd: i32) -> bool {
    let mut slot = None;
    for i in 0..MAX_WS_THREADS {
        let current = WS_CLIENT_FDS[i].load(Ordering::Acquire);
        if current == 0 {
            slot = Some(i);
            break;
        }
    }
    
    let slot_idx = match slot {
        Some(i) => i,
        None => {
            print(b"[ERROR] No available WebSocket thread slots\n");
            return false;
        }
    };
    
    WS_CLIENT_FDS[slot_idx].store(client_fd, Ordering::Release);
    WS_CLIENT_INDICES[slot_idx].store(slot_idx as i32, Ordering::Release);
    
    if !WS_STACKS[slot_idx].allocate() {
        print(b"[ERROR] Failed to allocate WebSocket stack\n");
        WS_CLIENT_FDS[slot_idx].store(0, Ordering::Release);
        WS_CLIENT_INDICES[slot_idx].store(-1, Ordering::Release);
        return false;
    }
    
    let tid = match spawn_thread(&WS_STACKS[slot_idx], websocket_handler_func) {
        Ok(tid) => tid,
        Err(err) => {
            print(b"[ERROR] Failed to start WebSocket thread: ");
            print(err.as_bytes());
            print(b"\n");
            WS_CLIENT_FDS[slot_idx].store(0, Ordering::Release);
            WS_CLIENT_INDICES[slot_idx].store(-1, Ordering::Release);
            return false;
        }
    };
    
    register_thread(tid);
    
    use crate::io::print_number;
    print(b"[WS Thread] WebSocket handler thread started (slot=");
    print_number(slot_idx as i64);
    print(b")\n");
    true
}
