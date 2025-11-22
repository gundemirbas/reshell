use crate::syscalls::{read, write, open, close, getdents64, access, LinuxDirent64};
use crate::syscalls::{STDIN, STDOUT, O_RDONLY, O_DIRECTORY, X_OK};
use crate::storage::{ENV_STORAGE};


pub fn expand_env_vars(input: &[u8], output: &mut [u8]) -> usize {
    let mut out_idx = 0;
    let mut i = 0;
    
    while i < input.len() && input[i] != 0 {
        if input[i] == b'$' && i + 1 < input.len() {
            i += 1;
            let var_start = i;
            while i < input.len() && (input[i].is_ascii_alphanumeric() || input[i] == b'_') {
                i += 1;
            }
            
            if i > var_start {
                let var_name = &input[var_start..i];
                let remaining = &mut output[out_idx..];
                let len = ENV_STORAGE.get(var_name, remaining);
                out_idx += len;
            }
        } else {
            if out_idx >= output.len() { return out_idx; }
            output[out_idx] = input[i];
            out_idx += 1;
            i += 1;
        }
    }
    
    out_idx
}

pub struct DirentParser<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> DirentParser<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }
    
    pub fn next(&mut self) -> Option<DirentEntry<'a>> {
        if self.pos >= self.buf.len() {
            return None;
        }
        
        unsafe {
            let dirent_ptr = self.buf.as_ptr().add(self.pos) as *const LinuxDirent64;
            let dirent = &*dirent_ptr;
            let reclen = dirent.d_reclen as usize;
            
            if self.pos + reclen > self.buf.len() {
                return None;
            }
            
            let name_offset = self.pos + 19;
            if name_offset >= self.buf.len() {
                self.pos += reclen;
                return self.next();
            }
            
            let name_start = name_offset;
            let mut name_end = name_start;
            let max_end = self.pos + reclen;
            while name_end < self.buf.len() && name_end < max_end && self.buf[name_end] != 0 {
                name_end += 1;
            }
            
            let entry = DirentEntry {
                name: &self.buf[name_start..name_end],
            };
            
            self.pos += reclen;
            Some(entry)
        }
    }
}

pub struct DirentEntry<'a> {
    pub name: &'a [u8],
}

pub fn find_in_path(cmd: &[u8], out_buf: &mut [u8]) -> bool {
    const PATHS: &[&[u8]] = &[
        b"/bin/",
        b"/usr/bin/",
        b"/usr/local/bin/",
    ];
    
    if cmd.len() > 0 && cmd[0] == b'/' {
        let mut idx = 0;
        for &b in cmd {
            if idx >= out_buf.len() - 1 {
                break;
            }
            out_buf[idx] = b;
            idx += 1;
        }
        out_buf[idx] = 0;
        return true;
    }
    
    for path in PATHS {
        let mut idx = 0;
        for &b in *path {
            if idx >= out_buf.len() - cmd.len() - 1 {
                break;
            }
            out_buf[idx] = b;
            idx += 1;
        }
        
        for &b in cmd {
            if idx >= out_buf.len() - 1 {
                break;
            }
            out_buf[idx] = b;
            idx += 1;
        }
        out_buf[idx] = 0;
        
        return true;
    }
    false
}

pub fn find_completions(prefix: &[u8], matches: &mut [[u8; 256]; 16]) -> usize {
    let mut count = 0;
    const PATHS: &[&[u8]] = &[b"/bin/", b"/usr/bin/"];
    
    for path in PATHS {
        if count >= 16 { break; }
        
        let mut path_buf = [0u8; 256];
        let mut idx = 0;
        for &b in *path {
            path_buf[idx] = b;
            idx += 1;
        }
        path_buf[idx] = 0;
        
        let fd = open(&path_buf[..idx + 1], O_RDONLY | O_DIRECTORY);
        if fd < 0 { continue; }
        
        let mut buf = [0u8; 4096];
        let mut should_break = false;
        loop {
            if should_break || count >= 16 {
                break;
            }
            
            let nread = getdents64(fd as i32, &mut buf);
            if nread <= 0 { break; }
            
            let mut parser = DirentParser::new(&buf[..nread as usize]);
            
            while let Some(entry) = parser.next() {
                if count >= 16 {
                    should_break = true;
                    break;
                }
                
                let name = entry.name;
                if name.len() == 0 || name.len() > 255 {
                    continue;
                }
                
                if !name.starts_with(prefix) {
                    continue;
                }
                
                let mut full_path = [0u8; 256];
                let mut pi = 0;
                for &b in *path {
                    if pi >= 255 { break; }
                    full_path[pi] = b;
                    pi += 1;
                }
                for i in 0..name.len() {
                    if pi >= 255 { break; }
                    full_path[pi] = name[i];
                    pi += 1;
                }
                if pi < 256 {
                    full_path[pi] = 0;
                }
                
                if access(&full_path[..pi.min(255) + 1], X_OK) == 0 {
                    for i in 0..name.len().min(255) {
                        matches[count][i] = name[i];
                    }
                    if name.len() < 256 {
                        matches[count][name.len()] = 0;
                    }
                    count += 1;
                }
            }
        }
        
        close(fd as i32);
    }
    
    count
}

#[allow(dead_code)]
pub fn read_line_with_completion(buf: &mut [u8]) -> usize {
    use crate::utils::split_first_word;
    
    let mut pos = 0;
    let mut tmp = [0u8; 1];
    
    loop {
        let n = read(STDIN, &mut tmp);
        if n <= 0 {
            return pos;
        }
        
        let ch = tmp[0];
        
        if ch == b'\n' {
            buf[pos] = b'\n';
            pos += 1;
            return pos;
        } else if ch == 9 {
            if pos > 0 {
                buf[pos] = 0;
                let line = &buf[..pos];
                let (prefix, _) = split_first_word(line);
                
                if prefix.len() > 0 && prefix.len() < 256 {
                    let mut matches = [[0u8; 256]; 16];
                    let match_count = find_completions(prefix, &mut matches);
                    
                    if match_count == 1 {
                        write(STDOUT, b"\n");
                        pos = 0;
                        let mut i = 0;
                        while matches[0][i] != 0 && pos < buf.len() - 1 {
                            buf[pos] = matches[0][i];
                            write(STDOUT, &[matches[0][i]]);
                            pos += 1;
                            i += 1;
                        }
                    } else if match_count > 1 {
                        write(STDOUT, b"\n");
                        for i in 0..match_count {
                            let mut j = 0;
                            while matches[i][j] != 0 {
                                write(STDOUT, &[matches[i][j]]);
                                j += 1;
                            }
                            write(STDOUT, b"\n");
                        }
                        write(STDOUT, b"$ ");
                        write(STDOUT, &buf[..pos]);
                    }
                }
            }
        } else if ch == 127 || ch == 8 {
            if pos > 0 {
                pos -= 1;
                write(STDOUT, b"\x08 \x08");
            }
        } else if ch >= 32 && ch < 127 {
            if pos < buf.len() - 1 {
                buf[pos] = ch;
                write(STDOUT, &[ch]);
                pos += 1;
            }
        }
    }
}
