use crate::syscalls::{write, chdir, open, close, getdents64, STDOUT, O_RDONLY, O_DIRECTORY};
use crate::utils::{trim_spaces, sort_entries};
use crate::shell::parser::{expand_env_vars, DirentParser};
use crate::io::print;

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
    
    let mut entries = [[0u8; 256]; 128];
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
    
    sort_entries(&mut entries, count);
    
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
