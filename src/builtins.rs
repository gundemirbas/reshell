use crate::syscalls::{write, chdir, open, close, getdents64, getcwd, STDOUT, O_RDONLY, O_DIRECTORY};
use crate::storage::{ENV_STORAGE, HISTORY, ALIASES};
use crate::utils::{trim_spaces, sort_entries};
use crate::parser::{expand_env_vars, DirentParser};
use crate::io::print;

pub fn builtin_export(args: &[u8]) {
    let args = trim_spaces(args);
    
    if args.is_empty() {
        ENV_STORAGE.iter(|var| {
            write(STDOUT, var);
            print(b"\n");
        });
        return;
    }
    
    for i in 0..args.len() {
        if args[i] == b'=' {
            let name = &args[..i];
            let value = if i + 1 < args.len() {
                &args[i + 1..]
            } else {
                &[]
            };
            if !ENV_STORAGE.set(name, value) {
                print(b"export: too many variables\n");
            }
            return;
        }
    }
    
    print(b"export: invalid format (use NAME=VALUE)\n");
}

pub fn builtin_echo(args: &[u8]) {
    let mut expanded = [0u8; 512];
    let len = expand_env_vars(args, &mut expanded);
    write(STDOUT, &expanded[..len]);
    print(b"\n");
}

pub fn builtin_pwd() {
    use crate::syscalls::getcwd;
    
    let mut buf = [0u8; 512];
    let ret = getcwd(&mut buf);
    
    if ret > 0 && ret < 512 {
        write(STDOUT, &buf[..ret as usize]);
        print(b"\n");
    } else {
        print(b"pwd: error getting current directory\n");
    }
}

pub fn builtin_history() {
    HISTORY.list();
}

pub fn builtin_serve(args: &[u8]) {
    use crate::server::start_http_server;
    
    let args = trim_spaces(args);
    
    // Parse port number, default to 8000
    let port = if args.is_empty() {
        8000
    } else {
        let mut port_num = 0u16;
        for &b in args {
            if b >= b'0' && b <= b'9' {
                port_num = port_num * 10 + (b - b'0') as u16;
            } else {
                break;
            }
        }
        if port_num == 0 {
            8000
        } else {
            port_num
        }
    };
    
    start_http_server(port);
}

pub fn builtin_alias(args: &[u8]) {
    let args = trim_spaces(args);
    
    if args.is_empty() {
        ALIASES.list();
        return;
    }
    
    for i in 0..args.len() {
        if args[i] == b'=' {
            let name = trim_spaces(&args[..i]);
            let value = trim_spaces(&args[i + 1..]);
            
            if !ALIASES.set(name, value) {
                print(b"alias: failed to set alias\n");
            }
            return;
        }
    }
    
    print(b"alias: invalid format (use NAME=VALUE)\n");
}

pub fn builtin_cd(path: &[u8]) {
    let path = trim_spaces(path);
    
    if path.is_empty() {
        print(b"cd: missing argument\n");
        return;
    }
    
    let mut expanded = [0u8; 512];
    let exp_len = expand_env_vars(path, &mut expanded);
    
    let mut path_buf = [0u8; 256];
    let mut idx = 0;
    for i in 0..exp_len {
        if idx >= path_buf.len() - 1 {
            print(b"cd: path too long\n");
            return;
        }
        path_buf[idx] = expanded[i];
        idx += 1;
    }
    path_buf[idx] = 0;
    
    let ret = chdir(&path_buf[..idx + 1]);
    if ret < 0 {
        print(b"cd: cannot change directory\n");
    }
}

pub fn builtin_ls(path: &[u8]) {
    let path = trim_spaces(path);
    let mut expanded = [0u8; 512];
    let exp_len = if path.is_empty() {
        0
    } else {
        expand_env_vars(path, &mut expanded)
    };
    
    let mut path_buf = [0u8; 256];
    let mut idx = 0;
    
    if exp_len == 0 {
        path_buf[0] = b'.';
        path_buf[1] = 0;
        idx = 1;
    } else {
        for i in 0..exp_len {
            if idx >= path_buf.len() - 1 {
                print(b"ls: path too long\n");
                return;
            }
            path_buf[idx] = expanded[i];
            idx += 1;
        }
        path_buf[idx] = 0;
    }
    
    let fd = open(&path_buf[..idx + 1], O_RDONLY | O_DIRECTORY);
    if fd < 0 {
        print(b"ls: cannot open directory\n");
        return;
    }
    
    // Collect entries first
    let mut entries = [[0u8; 256]; 128]; // Max 128 entries
    let mut count = 0;
    
    let mut buf = [0u8; 2048];
    loop {
        let nread = getdents64(fd as i32, &mut buf);
        if nread <= 0 {
            break;
        }
        
        let mut parser = DirentParser::new(&buf[..nread as usize]);
        while let Some(entry) = parser.next() {
            if count >= 128 {
                break;
            }
            
            let name = entry.name;
            if name.len() > 0 && name.len() < 256 {
                for i in 0..name.len() {
                    entries[count][i] = name[i];
                }
                entries[count][name.len()] = 0;
                count += 1;
            }
        }
        
        if count >= 128 {
            break;
        }
    }
    
    close(fd as i32);
    
    // Sort entries
    sort_entries(&mut entries, count);
    
    // Print sorted entries
    for i in 0..count {
        let mut len = 0;
        while len < 256 && entries[i][len] != 0 {
            len += 1;
        }
        if len > 0 {
            write(STDOUT, &entries[i][..len]);
            print(b"\n");
        }
    }
}
