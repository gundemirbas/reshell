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

use core::panic::PanicInfo;
use syscalls::*;
use io::{print, print_number, print_cstr, CStr, StaticBuffer, read_line};
use storage::ENV_STORAGE;
use executor::execute_command;
use thread::start_ticker_thread;

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

#[unsafe(no_mangle)]
extern "C" fn main(argc: i64, argv: *const *const u8) -> i32 {
    let args = Args::new(argc, argv);
    
    print(b"Shell started with ");
    print_number(argc);
    print(b" argument(s)\n");
    
    if argc > 0 {
        print(b"Arguments:\n");
        for i in 0..argc {
            print(b"  [");
            print_number(i);
            print(b"]: ");
            if let Some(arg) = args.get(i) {
                print_cstr(&arg);
            }
            print(b"\n");
        }
        print(b"\n");
    }
    
    print(b"Minimal Shell v0.2 - Type 'exit' to quit\n");
    print(b"Features: tab completion, env vars, aliases, history\n");
    print(b"Builtins: ls, cd, pwd, export, echo, env, alias, history\n");
    print(b"[Ticker thread active - prints # every 10s]\n");
    
    ENV_STORAGE.set(b"HOME", b"/home");
    ENV_STORAGE.set(b"USER", b"user");
    ENV_STORAGE.set(b"PATH", b"/bin:/usr/bin");
    
    start_ticker_thread();
    
    loop {
        print(b"$ ");
        
        let should_break = INPUT_BUF.with_mut(|buf| {
            let n = read_line(buf);
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
