mod stack;
mod registry;
mod spawn;

use stack::ThreadStack;
pub use registry::{get_thread_stats, register_thread, cleanup_threads};
pub use spawn::spawn_thread;

use core::sync::atomic::{AtomicU16, AtomicI32, AtomicBool, Ordering};
use crate::syscalls::{nanosleep, close};
use crate::io::print;

static HTTP_STACK: ThreadStack = ThreadStack::new_large();
static HTTP_PORT: AtomicU16 = AtomicU16::new(0);

const MAX_WS_THREADS: usize = 16;
static WS_STACKS: [ThreadStack; MAX_WS_THREADS] = [const { ThreadStack::new() }; MAX_WS_THREADS];
static WS_CLIENT_FDS: [AtomicI32; MAX_WS_THREADS] = [const { AtomicI32::new(0) }; MAX_WS_THREADS];
static WS_THREAD_ACTIVE: [AtomicBool; MAX_WS_THREADS] = [const { AtomicBool::new(false) }; MAX_WS_THREADS];

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

// Per-thread WebSocket handlers - each gets its own function to avoid race conditions
static WS_THREAD_FD: [AtomicI32; MAX_WS_THREADS] = [const { AtomicI32::new(0) }; MAX_WS_THREADS];

macro_rules! make_ws_handler {
    ($idx:expr) => {{
        fn handler() -> ! {
            use crate::syscalls::sys_exit;
            use crate::io::print_number;
            
            print(b"[WS Handler] Entry point reached\n");
            
            let client_fd = WS_THREAD_FD[$idx].load(Ordering::Acquire);
            
            print(b"[WS Handler] FD loaded: ");
            print_number(client_fd as i64);
            print(b"\n");
            
            if client_fd <= 0 {
                print(b"[WS Thread] No client FD\n");
                sys_exit(1);
            }
            
            print(b"[WS Thread ");
            print_number($idx as i64);
            print(b"] Starting (fd=");
            print_number(client_fd as i64);
            print(b")\n");
            
            use crate::network::websocket::websocket_frame_loop_with_index;
            websocket_frame_loop_with_index(client_fd, $idx);
            
            print(b"[WS Thread ");
            print_number($idx as i64);
            print(b"] Closing connection\n");
            
            close(client_fd);
            WS_CLIENT_FDS[$idx].store(0, Ordering::Release);
            WS_THREAD_ACTIVE[$idx].store(false, Ordering::Release);
            
            sys_exit(0);
        }
        handler
    }};
}

fn get_ws_handler(idx: usize) -> fn() -> ! {
    match idx {
        0 => make_ws_handler!(0),
        1 => make_ws_handler!(1),
        2 => make_ws_handler!(2),
        3 => make_ws_handler!(3),
        4 => make_ws_handler!(4),
        5 => make_ws_handler!(5),
        6 => make_ws_handler!(6),
        7 => make_ws_handler!(7),
        8 => make_ws_handler!(8),
        9 => make_ws_handler!(9),
        10 => make_ws_handler!(10),
        11 => make_ws_handler!(11),
        12 => make_ws_handler!(12),
        13 => make_ws_handler!(13),
        14 => make_ws_handler!(14),
        15 => make_ws_handler!(15),
        _ => {
            fn fallback() -> ! {
                use crate::syscalls::sys_exit;
                print(b"[WS Thread] Invalid slot\n");
                sys_exit(1);
            }
            fallback
        }
    }
}

pub fn start_websocket_thread(client_fd: i32) -> bool {
    // Find available slot
    let mut slot = None;
    for i in 0..MAX_WS_THREADS {
        let active = WS_THREAD_ACTIVE[i].load(Ordering::Acquire);
        if !active {
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
    
    // Mark slot as active before starting thread
    WS_THREAD_ACTIVE[slot_idx].store(true, Ordering::Release);
    WS_THREAD_FD[slot_idx].store(client_fd, Ordering::Release);
    WS_CLIENT_FDS[slot_idx].store(client_fd, Ordering::Release);
    
    // Allocate stack
    if !WS_STACKS[slot_idx].allocate() {
        print(b"[ERROR] Failed to allocate WebSocket stack\n");
        WS_CLIENT_FDS[slot_idx].store(0, Ordering::Release);
        WS_THREAD_ACTIVE[slot_idx].store(false, Ordering::Release);
        return false;
    }
    
    // Get the appropriate handler for this slot
    let handler = get_ws_handler(slot_idx);
    
    // Spawn thread
    let tid = match spawn_thread(&WS_STACKS[slot_idx], handler) {
        Ok(tid) => tid,
        Err(err) => {
            print(b"[ERROR] Failed to start WebSocket thread: ");
            print(err.as_bytes());
            print(b"\n");
            WS_CLIENT_FDS[slot_idx].store(0, Ordering::Release);
            WS_THREAD_ACTIVE[slot_idx].store(false, Ordering::Release);
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
