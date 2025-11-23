#![no_std]
#![no_main]

mod syscalls;
mod utils;
mod io;
mod assets;
mod shell;
mod network;
mod system;

use core::panic::PanicInfo;
use syscalls::*;
use io::{print, print_number, CStr, StaticBuffer, read_line_with_tab};
use shell::{ENV_STORAGE, execute_command};

fn initialize_path_from_envp(envp: *const *const u8) -> bool {
    if envp.is_null() {
        return false;
    }
    
    unsafe {
        let mut env_ptr = envp;
        let mut count = 0;
        
        while count < 1000 {
            let env_str_ptr = *env_ptr;
            if env_str_ptr.is_null() {
                break;
            }
            
            let mut len = 0;
            while len < 4096 && *env_str_ptr.add(len) != 0 {
                len += 1;
            }
            
            if is_path_variable(env_str_ptr, len) {
                return extract_and_set_path(env_str_ptr, len);
            }
            
            env_ptr = env_ptr.add(1);
            count += 1;
        }
    }
    
    false
}

fn is_path_variable(env_str_ptr: *const u8, len: usize) -> bool {
    len > 5 && unsafe {
        *env_str_ptr == b'P' && 
        *env_str_ptr.add(1) == b'A' && 
        *env_str_ptr.add(2) == b'T' && 
        *env_str_ptr.add(3) == b'H' && 
        *env_str_ptr.add(4) == b'='
    }
}

fn extract_and_set_path(env_str_ptr: *const u8, len: usize) -> bool {
    let mut path_value = [0u8; 2048];
    let path_len = (len - 5).min(2048);
    
    if path_len == 0 {
        return false;
    }
    
    unsafe {
        for i in 0..path_len {
            path_value[i] = *env_str_ptr.add(5 + i);
        }
    }
    
    ENV_STORAGE.set(b"PATH", &path_value[..path_len]);
    true
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    write(STDERR, b"Panic!\n");
    cleanup_and_exit(1);
}

fn cleanup_and_exit(code: i32) -> ! {
    print(b"\n[INFO] Shutting down...\n");
    
    use crate::syscalls::signal::request_shutdown;
    request_shutdown();
    
    use crate::network::close_server_socket;
    close_server_socket();
    
    print(b"[INFO] Waiting for threads to finish...\n");
    nanosleep(0, 500_000_000);
    
    print(b"[INFO] Goodbye!\n");
    sys_exit(code);
}

fn should_shutdown() -> bool {
    use crate::syscalls::signal::should_shutdown as sig_should_shutdown;
    sig_should_shutdown()
}

struct Args {
    count: i64,
    ptr: *const *const u8,
}

impl Args {
    fn new(argc: i64, argv: *const *const u8) -> Self {
        Args { count: argc, ptr: argv }
    }
    
    fn get(&self, index: usize) -> Option<CStr> {
        if index < self.count as usize {
            unsafe {
                let arg_ptr = *self.ptr.add(index);
                if !arg_ptr.is_null() {
                    CStr::from_ptr(arg_ptr)
                } else {
                    None
                }
            }
        } else {
            None
        }
    }
}

fn parse_port(bytes: &[u8]) -> Option<u16> {
    let mut port: u16 = 0;
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

core::arch::global_asm!(
    ".global _start",
    ".type _start, @function",
    "_start:",
    "   mov rdi, [rsp]",           // argc
    "   lea rsi, [rsp + 8]",       // argv
    "   mov rax, rdi",             // rax = argc
    "   add rax, 1",               // rax = argc + 1
    "   shl rax, 3",               // rax = (argc+1) * 8
    "   lea rdx, [rsi + rax]",     // envp = argv + (argc+1)*8
    "   call main",
    "   mov rdi, rax",             // exit code
    "   mov rax, 60",              // sys_exit
    "   syscall",
);

#[unsafe(no_mangle)]
extern "C" fn main(argc: i64, argv: *const *const u8, envp: *const *const u8) -> i32 {
    let args = Args::new(argc, argv);
    
    let port = if argc > 1 {
        if let Some(arg) = args.get(1) {
            parse_port(arg.as_bytes()).unwrap_or(8000)
        } else {
            8000
        }
    } else {
        8000
    };
    
    // Get PATH from parent environment
    if !initialize_path_from_envp(envp) {
        ENV_STORAGE.set(b"PATH", b"/bin:/usr/bin:/usr/local/bin");
    }
    
    if !setup_signal_handlers() {
        print(b"[WARN] Failed to setup signal handlers\n");
    }
    
    print(b"Minimal Shell v0.3\n");
    print(b"Features: tab completion, env vars, WebSocket, multi-threaded\n");
    print(b"Builtins: ls, cd, pwd, export, echo, env, threads, exit\n");
    print(b"Signal handlers: SIGINT, SIGTERM, SIGPIPE\n\n");
    
    ENV_STORAGE.set(b"HOME", b"/home");
    ENV_STORAGE.set(b"USER", b"user");
    // PATH already set from parent env
    
    use crate::system::thread::start_http_server_thread;
    
    print(b"[INFO] Starting HTTP server on port ");
    print_number(port as i64);
    print(b"\n[INFO] WebSocket endpoint: /ws\n");
    print(b"[INFO] Web files served from: html/\n");
    print(b"[INFO] Server running in multi-threaded mode\n");
    print(b"[INFO] Type 'exit' to quit, or use commands below\n\n");
    
    start_http_server_thread(port);
    
    use syscalls::nanosleep;
    nanosleep(0, 200_000_000);
    
    let has_websocket = port > 0;
    
    static INPUT_BUF: StaticBuffer = StaticBuffer::new();
    
    loop {
        if should_shutdown() {
            cleanup_and_exit(0);
        }
        
        // Check for WebSocket commands
        if has_websocket {
            use crate::network::websocket::{get_shell_command, broadcast_message};
            use crate::io::{enable_output_capture, disable_output_capture, get_captured_output};
            
            let mut ws_cmd = [0u8; 512];
            let ws_cmd_len = get_shell_command(&mut ws_cmd);
            if ws_cmd_len > 0 {
                // Enable output capture BEFORE executing
                enable_output_capture();
                execute_command(&ws_cmd[..ws_cmd_len]);
                disable_output_capture();
                
                let mut output = [0u8; 4096];
                let output_len = get_captured_output(&mut output);
                
                print(b"[Shell] Executed: ");
                write(STDOUT, &ws_cmd[..ws_cmd_len]);
                print(b" (");
                print_number(output_len as i64);
                print(b" bytes)\n");
                
                if output_len > 0 {
                    broadcast_message(&output[..output_len]);
                }
                
                nanosleep(0, 10_000_000);
                continue;
            }
        }
        
        // Local shell prompt
        print(b"reshell> ");
        
        INPUT_BUF.with_mut(|input| {
            let n = read_line_with_tab(input);
            
            if n > 0 {
                if has_websocket {
                    use crate::network::websocket::broadcast_message;
                    use crate::io::{enable_output_capture, disable_output_capture, get_captured_output};
                    
                    // Broadcast command
                    let mut cmd_msg = [0u8; 520];
                    let mut cmd_len = 0;
                    let prefix = b"[Local] > ";
                    for &b in prefix {
                        cmd_msg[cmd_len] = b;
                        cmd_len += 1;
                    }
                    for i in 0..n.min(510) {
                        cmd_msg[cmd_len] = input[i];
                        cmd_len += 1;
                    }
                    broadcast_message(&cmd_msg[..cmd_len]);
                    
                    // Execute with output capture
                    enable_output_capture();
                    execute_command(&input[..n]);
                    disable_output_capture();
                    
                    let mut output = [0u8; 4096];
                    let output_len = get_captured_output(&mut output);
                    if output_len > 0 {
                        broadcast_message(&output[..output_len]);
                    }
                } else {
                    execute_command(&input[..n]);
                }
            }
            
            if should_shutdown() {
                cleanup_and_exit(0);
            }
        });
        
        if has_websocket {
            nanosleep(0, 10_000_000);
        }
    }
}
