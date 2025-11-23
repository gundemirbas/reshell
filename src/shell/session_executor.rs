use crate::utils::{trim_newline, bytes_equal, split_first_word};
use crate::shell::session::ShellSession;

pub fn execute_command_in_session(session: &ShellSession, cmd: &[u8]) {
    let cmd = trim_newline(cmd);
    
    if cmd.is_empty() {
        return;
    }

    // Special commands that shouldn't run in session context
    if bytes_equal(cmd, b"exit") {
        session.write_output(b"Session closed\n");
        return;
    }

    let (program, args) = split_first_word(cmd);
    
    // Built-in commands - redirect output to session
    if bytes_equal(program, b"pwd") {
        execute_builtin_pwd(session);
        return;
    }
    
    if bytes_equal(program, b"cd") {
        execute_builtin_cd(session, args);
        return;
    }
    
    if bytes_equal(program, b"ls") {
        execute_builtin_ls(session, args);
        return;
    }
    
    if bytes_equal(program, b"echo") {
        execute_builtin_echo(session, args);
        return;
    }
    
    if bytes_equal(program, b"export") {
        execute_builtin_export(session, args);
        return;
    }
    
    if bytes_equal(program, b"env") {
        execute_builtin_export(session, b"");
        return;
    }
    
    if bytes_equal(program, b"threads") {
        execute_builtin_threads(session);
        return;
    }
    
    // External command - would need process isolation per session
    // For now, just report
    session.write_output(b"External commands not yet supported in session mode\n");
}

fn execute_builtin_pwd(session: &ShellSession) {
    use crate::syscalls::getcwd;
    let mut buf = [0u8; 512];
    let len = getcwd(&mut buf);
    if len > 0 {
        session.write_output(&buf[..len as usize]);
        session.write_output(b"\n");
    } else {
        session.write_output(b"pwd: error\n");
    }
}

fn execute_builtin_cd(session: &ShellSession, path: &[u8]) {
    use crate::syscalls::chdir;
    use crate::utils::trim_spaces;
    use crate::shell::parser::expand_env_vars;
    
    let path = trim_spaces(path);
    
    if path.is_empty() {
        session.write_output(b"cd: missing argument\n");
        return;
    }
    
    let mut expanded = [0u8; 512];
    let len = expand_env_vars(path, &mut expanded);
    let expanded_path = &expanded[..len];
    
    let mut path_with_null = [0u8; 512];
    let copy_len = expanded_path.len().min(511);
    path_with_null[..copy_len].copy_from_slice(&expanded_path[..copy_len]);
    path_with_null[copy_len] = 0;
    
    let ret = chdir(&path_with_null);
    if ret < 0 {
        session.write_output(b"cd: ");
        session.write_output(expanded_path);
        session.write_output(b": No such directory\n");
    }
}

fn execute_builtin_ls(session: &ShellSession, path: &[u8]) {
    use crate::syscalls::{open, close, getdents64, O_RDONLY, O_DIRECTORY};
    use crate::shell::parser::DirentParser;
    use crate::utils::{trim_spaces, sort_entries};
    
    let path = trim_spaces(path);
    let dir_path = if path.is_empty() { b".\0" } else { path };
    
    let fd = open(dir_path, O_RDONLY | O_DIRECTORY);
    if fd < 0 {
        session.write_output(b"ls: cannot open directory\n");
        return;
    }
    
    let mut buf = [0u8; 4096];
    let mut entries = [[0u8; 256]; 64];
    let mut count = 0;
    
    loop {
        let n = getdents64(fd as i32, &mut buf);
        if n <= 0 {
            break;
        }
        
        let mut parser = DirentParser::new(&buf[..n as usize]);
        while let Some(dirent) = parser.next() {
            // Skip . and ..
            if dirent.name.len() == 1 && dirent.name[0] == b'.' {
                continue;
            }
            if dirent.name.len() == 2 && dirent.name[0] == b'.' && dirent.name[1] == b'.' {
                continue;
            }
            
            if count < 64 {
                let name_len = dirent.name.len().min(256);
                entries[count][..name_len].copy_from_slice(&dirent.name[..name_len]);
                count += 1;
            }
        }
    }
    
    close(fd as i32);
    
    sort_entries(&mut entries, count);
    
    for i in 0..count {
        let name_len = entries[i].iter().position(|&c| c == 0).unwrap_or(256);
        session.write_output(&entries[i][..name_len]);
        session.write_output(b"  ");
    }
    if count > 0 {
        session.write_output(b"\n");
    }
}

fn execute_builtin_echo(session: &ShellSession, args: &[u8]) {
    use crate::shell::parser::expand_env_vars;
    use crate::utils::trim_spaces;
    
    let args = trim_spaces(args);
    
    let mut expanded = [0u8; 512];
    let len = expand_env_vars(args, &mut expanded);
    
    session.write_output(&expanded[..len]);
    session.write_output(b"\n");
}

fn execute_builtin_export(session: &ShellSession, args: &[u8]) {
    use crate::shell::ENV_STORAGE;
    use crate::utils::trim_spaces;
    
    let args = trim_spaces(args);
    
    if args.is_empty() {
        ENV_STORAGE.iter(|var| {
            session.write_output(var);
            session.write_output(b"\n");
        });
        return;
    }
    
    if let Some(eq_pos) = args.iter().position(|&c| c == b'=') {
        let key = &args[..eq_pos];
        let value = &args[eq_pos + 1..];
        ENV_STORAGE.set(key, value);
    }
}

fn execute_builtin_threads(session: &ShellSession) {
    use crate::system::thread::get_thread_stats;
    
    let (active, total) = get_thread_stats();
    
    session.write_output(b"Active threads: ");
    write_number_to_session(session, active as i64);
    session.write_output(b" / ");
    write_number_to_session(session, total as i64);
    session.write_output(b"\n");
}

fn write_number_to_session(session: &ShellSession, n: i64) {
    if n == 0 {
        session.write_output(b"0");
        return;
    }
    
    let mut num = n;
    let mut digits = [0u8; 20];
    let mut count = 0;
    
    while num > 0 {
        digits[count] = b'0' + (num % 10) as u8;
        num /= 10;
        count += 1;
    }
    
    for i in (0..count).rev() {
        session.write_output(&[digits[i]]);
    }
}
