use crate::syscalls::{fork, execve, waitpid, sys_exit, nanosleep};
use crate::utils::{trim_newline, bytes_equal, split_first_word};
use crate::shell::builtins::*;
use crate::shell::parser::find_in_path;
use crate::io::{print, print_number, StaticBuffer};

static CMD_BUF: StaticBuffer = StaticBuffer::new();

fn cleanup_and_exit(code: i32) -> ! {
    print(b"\n[INFO] Shutting down...\n");
    
    use crate::syscalls::request_shutdown;
    request_shutdown();
    
    use crate::network::close_server_socket;
    close_server_socket();
    
    print(b"[INFO] Waiting for threads to finish...\n");
    nanosleep(0, 500_000_000); // 500ms bekle
    
    use crate::system::thread::cleanup_threads;
    cleanup_threads();
    
    nanosleep(0, 100_000_000);
    
    print(b"[INFO] Goodbye!\n");
    sys_exit(code);
}

pub fn execute_command(cmd: &[u8]) {
    let cmd = trim_newline(cmd);
    
    if cmd.is_empty() {
        return;
    }

    if bytes_equal(cmd, b"exit") {
        cleanup_and_exit(0);
    }

    let (program, args) = split_first_word(cmd);
    
    if bytes_equal(program, b"cd") {
        builtin_cd(args);
        return;
    }
    
    if bytes_equal(program, b"ls") {
        builtin_ls(args);
        return;
    }
    
    if bytes_equal(program, b"pwd") {
        builtin_pwd();
        return;
    }
    
    if bytes_equal(program, b"export") {
        builtin_export(args);
        return;
    }
    
    if bytes_equal(program, b"echo") {
        builtin_echo(args);
        return;
    }
    
    if bytes_equal(program, b"env") {
        builtin_export(b"");
        return;
    }
    
    if bytes_equal(program, b"threads") {
        builtin_threads();
        return;
    }
    
    let pid = fork();
    
    if pid == 0 {
        CMD_BUF.with_mut(|cmd_buf| {
            find_in_path(program, cmd_buf);
            
            let argv: [*const u8; 2] = [cmd_buf.as_ptr(), core::ptr::null()];
            let ret = execve(cmd_buf, &argv);
            
            print(b"Command not found (errno: ");
            print_number(-ret as i64);
            print(b")\n");
            sys_exit(1);
        })
    } else if pid > 0 {
        let mut status: i32 = 0;
        waitpid(pid as i32, &mut status);
    } else {
        print(b"Fork failed\n");
    }
}
