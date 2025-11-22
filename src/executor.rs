use crate::syscalls::{fork, execve, waitpid, sys_exit};
use crate::storage::{HISTORY, ALIASES};
use crate::utils::{trim_newline, bytes_equal, split_first_word};
use crate::builtins::*;
use crate::parser::find_in_path;
use crate::io::{print, print_number, StaticBuffer};

static CMD_BUF: StaticBuffer = StaticBuffer::new();

pub fn execute_command(cmd: &[u8]) {
    let cmd = trim_newline(cmd);
    
    if cmd.is_empty() {
        return;
    }
    
    HISTORY.add(cmd);

    if bytes_equal(cmd, b"exit") {
        print(b"Goodbye!\n");
        sys_exit(0);
    }

    let (mut program, mut args) = split_first_word(cmd);
    
    let mut alias_buf = [0u8; 128];
    let alias_len = ALIASES.get(program, &mut alias_buf);
    if alias_len > 0 {
        let (alias_prog, alias_args) = split_first_word(&alias_buf[..alias_len]);
        program = alias_prog;
        
        if alias_args.is_empty() {
            args = args;
        } else if args.is_empty() {
            args = alias_args;
        }
    }
    
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
    
    if bytes_equal(program, b"history") {
        builtin_history();
        return;
    }
    
    if bytes_equal(program, b"alias") {
        builtin_alias(args);
        return;
    }
    
    if bytes_equal(program, b"serve") {
        builtin_serve(args);
        return;
    }
    
    if bytes_equal(program, b"servepty") {
        builtin_servepty(args);
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
