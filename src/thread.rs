use core::cell::UnsafeCell;
use crate::syscalls::{nanosleep, sys_clone_with_func};
use crate::syscalls::{CLONE_VM, CLONE_FS, CLONE_FILES, CLONE_SIGHAND, CLONE_THREAD};
use crate::io::print;

pub struct ThreadStack {
    data: UnsafeCell<[u8; 131072]>,  // 128 KB per thread
}

unsafe impl Sync for ThreadStack {}

impl ThreadStack {
    const fn new() -> Self {
        Self {
            data: UnsafeCell::new([0u8; 131072]),
        }
    }
    
    fn get_stack_top(&self) -> *mut u8 {
        unsafe {
            let buf = &mut *self.data.get();
            let stack_size = buf.len();
            let stack_ptr = buf.as_mut_ptr();
            let stack_top = stack_ptr.add(stack_size);
            (stack_top as usize & !15) as *mut u8
        }
    }
}

static HTTP_STACK: ThreadStack = ThreadStack::new();
static PTY_STACK: ThreadStack = ThreadStack::new();

static mut HTTP_PORT: u16 = 0;
static mut PTY_PORT: u16 = 0;

fn http_server_func() -> ! {
    use crate::server::start_http_server;
    use crate::syscalls::sys_exit;
    
    nanosleep(0, 100_000_000);
    
    let port = unsafe { HTTP_PORT };
    start_http_server(port);
    
    print(b"[ERROR] HTTP server exited\n");
    sys_exit(1);
}

fn pty_server_func() -> ! {
    use crate::server::start_pty_server;
    use crate::syscalls::sys_exit;
    
    nanosleep(0, 300_000_000); // Wait 300ms to let HTTP bind first
    
    let port = unsafe { PTY_PORT };
    start_pty_server(port);
    
    print(b"[ERROR] PTY server exited\n");
    sys_exit(1);
}

pub fn start_http_server_thread(port: u16) {
    unsafe { HTTP_PORT = port; }
    
    let flags = CLONE_VM | CLONE_FS | CLONE_FILES | CLONE_SIGHAND | CLONE_THREAD;
    let stack_top = HTTP_STACK.get_stack_top();
    
    let tid = unsafe {
        sys_clone_with_func(flags, stack_top, http_server_func)
    };
    
    if tid < 0 {
        print(b"[ERROR] Failed to start HTTP server thread\n");
    }
}

pub fn start_pty_server_thread(port: u16) {
    unsafe { PTY_PORT = port; }
    
    let flags = CLONE_VM | CLONE_FS | CLONE_FILES | CLONE_SIGHAND | CLONE_THREAD;
    let stack_top = PTY_STACK.get_stack_top();
    
    let tid = unsafe {
        sys_clone_with_func(flags, stack_top, pty_server_func)
    };
    
    if tid < 0 {
        print(b"[ERROR] Failed to start PTY server thread\n");
    }
}
