#![no_std]
#![no_main]

mod syscalls;
mod storage;
mod utils;
mod io;
mod parser;
mod builtins;
mod executor;
mod thread;
mod server;
mod assets;
mod crypto;
mod websocket;

use core::panic::PanicInfo;
use syscalls::*;
use io::{print, print_number, CStr, StaticBuffer, read_line_with_tab};
use storage::ENV_STORAGE;
use executor::execute_command;
use utils::bytes_equal;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    write(STDERR, b"Panic!\n");
    sys_exit(1);
}

static INPUT_BUF: StaticBuffer = StaticBuffer::new();

struct Args {
    count: i64,
    ptr: *const *const u8,
}

impl Args {
    fn new(argc: i64, argv: *const *const u8) -> Self {
        Args { count: argc, ptr: argv }
    }
    
    fn get(&self, index: i64) -> Option<CStr> {
        if index < 0 || index >= self.count || self.ptr.is_null() {
            None
        } else {
            unsafe {
                let ptr = *self.ptr.offset(index as isize);
                CStr::from_ptr(ptr)
            }
        }
    }
}

fn parse_port(bytes: &[u8]) -> Option<u16> {
    let mut port = 0u16;
    for &b in bytes {
        if b >= b'0' && b <= b'9' {
            port = port.saturating_mul(10).saturating_add((b - b'0') as u16);
        } else if b == 0 {
            break;
        } else {
            return None;
        }
    }
    if port > 0 { Some(port) } else { None }
}

#[unsafe(no_mangle)]
extern "C" fn main(argc: i64, argv: *const *const u8) -> i32 {
    let args = Args::new(argc, argv);
    
    // Parse command line arguments
    let mut serve_port = None;
    let mut servepty_port = None;
    
    // Simple argument parsing (no complex logic to avoid crashes)
    // Usage: reshell --serve 8000 --servepty 8080
    let mut i = 1;
    while i < argc {
        if let Some(arg) = args.get(i) {
            let arg_bytes = arg.as_bytes();
            
            if bytes_equal(arg_bytes, b"--serve") || bytes_equal(arg_bytes, b"-s") {
                if let Some(next) = args.get(i + 1) {
                    serve_port = parse_port(next.as_bytes());
                    i += 1;
                }
            } else if bytes_equal(arg_bytes, b"--servepty") || bytes_equal(arg_bytes, b"-p") {
                if let Some(next) = args.get(i + 1) {
                    servepty_port = parse_port(next.as_bytes());
                    i += 1;
                }
            }
        }
        i += 1;
    }
    
    print(b"Minimal Shell v0.2\n");
    print(b"Features: tab completion, env vars, aliases, history, HTTP server\n");
    print(b"Builtins: ls, cd, pwd, export, echo, env, alias, history, serve, servepty\n\n");
    
    ENV_STORAGE.set(b"HOME", b"/home");
    ENV_STORAGE.set(b"USER", b"user");
    ENV_STORAGE.set(b"PATH", b"/bin:/usr/bin");
    
    // Start server threads if requested
    use crate::thread::{start_http_server_thread, start_pty_server_thread};
    
    let mut server_mode = false;
    
    if let Some(port) = serve_port {
        print(b"[INFO] Starting HTTP server on port ");
        print_number(port as i64);
        print(b"\n");
        start_http_server_thread(port);
        server_mode = true;
    }
    
    if let Some(port) = servepty_port {
        print(b"[INFO] Starting PTY WebSocket server on port ");
        print_number(port as i64);
        print(b"\n");
        start_pty_server_thread(port);
        server_mode = true;
    }
    
    if server_mode {
        print(b"\nServers started. Press Ctrl+C to quit or use the shell below.\n\n");
    }
    
    loop {
        print(b"$ ");
        
        let should_break = INPUT_BUF.with_mut(|buf| {
            let n = read_line_with_tab(buf);
            if n == 0 {
                return true;
            }
            
            execute_command(&buf[..n]);
            false
        });
        
        if should_break {
            break;
        }
    }
    
    0
}

core::arch::global_asm!(
    ".global _start",
    "_start:",
    "xor rbp, rbp",
    "pop rdi",
    "mov rsi, rsp",
    "and rsp, ~0xF",
    "call main",
    "mov rdi, rax",
    "mov rax, 60",
    "syscall"
);
