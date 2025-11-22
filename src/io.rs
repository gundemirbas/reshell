use core::cell::UnsafeCell;
use crate::syscalls::{read, write, STDIN, STDOUT};

pub fn print(s: &[u8]) {
    write(STDOUT, s);
}

pub fn print_number(n: i64) {
    if n == 0 {
        print(b"0");
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
        write(STDOUT, &[digits[i]]);
    }
}

pub struct CStr {
    ptr: *const u8,
}

impl CStr {
    pub unsafe fn from_ptr(ptr: *const u8) -> Option<Self> {
        if ptr.is_null() {
            None
        } else {
            Some(Self { ptr })
        }
    }
    
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            let mut len = 0;
            while *self.ptr.add(len) != 0 && len < 4096 {
                len += 1;
            }
            core::slice::from_raw_parts(self.ptr, len)
        }
    }
}

pub fn print_cstr(cstr: &CStr) {
    write(STDOUT, cstr.as_bytes());
}

pub struct StaticBuffer {
    data: UnsafeCell<[u8; 128]>,
}

unsafe impl Sync for StaticBuffer {}

impl StaticBuffer {
    pub const fn new() -> Self {
        Self {
            data: UnsafeCell::new([0u8; 128]),
        }
    }
    
    pub fn with_mut<F, R>(&self, f: F) -> R 
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        unsafe {
            let buf = &mut *self.data.get();
            f(buf)
        }
    }
}

pub fn read_line(buf: &mut [u8]) -> usize {
    let n = read(STDIN, buf);
    if n > 0 { n as usize } else { 0 }
}

// Simple tab-completion aware input
pub fn read_line_with_tab(buf: &mut [u8]) -> usize {
    use crate::syscalls::ioctl;
    use crate::syscalls::{Termios, TCGETS, TCSETS, ICANON, ECHO};
    use crate::parser::find_completions;
    use crate::utils::split_first_word;
    
    // Get current terminal settings
    let mut old_term = Termios {
        c_iflag: 0,
        c_oflag: 0,
        c_cflag: 0,
        c_lflag: 0,
        c_line: 0,
        c_cc: [0u8; 32],
        c_ispeed: 0,
        c_ospeed: 0,
    };
    
    let old_ptr = &mut old_term as *mut Termios as u64;
    if ioctl(STDIN, TCGETS, old_ptr) < 0 {
        // If ioctl fails, fall back to regular read
        return read_line(buf);
    }
    
    // Set raw mode
    let mut raw_term = old_term;
    raw_term.c_lflag &= !(ICANON | ECHO);
    
    let raw_ptr = &mut raw_term as *mut Termios as u64;
    ioctl(STDIN, TCSETS, raw_ptr);
    
    let mut pos = 0;
    let mut tmp = [0u8; 1];
    
    loop {
        let n = read(STDIN, &mut tmp);
        if n <= 0 {
            break;
        }
        
        let ch = tmp[0];
        
        if ch == b'\n' || ch == b'\r' {
            write(STDOUT, b"\n");
            buf[pos] = b'\n';
            pos += 1;
            break;
        } else if ch == 9 { // Tab
            if pos > 0 {
                buf[pos] = 0;
                let line = &buf[..pos];
                let (prefix, _) = split_first_word(line);
                
                if prefix.len() > 0 && prefix.len() < 256 {
                    let mut matches = [[0u8; 256]; 16];
                    let match_count = find_completions(prefix, &mut matches);
                    
                    if match_count == 1 {
                        // Clear current line
                        for _ in 0..pos {
                            write(STDOUT, b"\x08 \x08");
                        }
                        
                        // Write completion
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
                            write(STDOUT, b"  ");
                        }
                        write(STDOUT, b"\n$ ");
                        write(STDOUT, &buf[..pos]);
                    }
                }
            }
        } else if ch == 127 || ch == 8 { // Backspace
            if pos > 0 {
                pos -= 1;
                write(STDOUT, b"\x08 \x08");
            }
        } else if ch == 3 { // Ctrl+C
            write(STDOUT, b"^C\n");
            pos = 0;
            break;
        } else if ch >= 32 && ch < 127 {
            if pos < buf.len() - 1 {
                buf[pos] = ch;
                write(STDOUT, &[ch]);
                pos += 1;
            }
        }
    }
    
    // Restore terminal settings
    ioctl(STDIN, TCSETS, old_ptr);
    
    pos
}
