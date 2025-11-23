use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use crate::syscalls::{read, write, STDIN, STDOUT};

// Output capture for broadcasting to WebSocket
static OUTPUT_CAPTURE_ENABLED: AtomicBool = AtomicBool::new(false);
static OUTPUT_BUFFER: [AtomicUsize; 4096] = [const { AtomicUsize::new(0) }; 4096];
static OUTPUT_BUFFER_LEN: AtomicUsize = AtomicUsize::new(0);

pub fn enable_output_capture() {
    OUTPUT_BUFFER_LEN.store(0, Ordering::Release);
    OUTPUT_CAPTURE_ENABLED.store(true, Ordering::Release);
}

pub fn disable_output_capture() {
    OUTPUT_CAPTURE_ENABLED.store(false, Ordering::Release);
}

pub fn get_captured_output(out: &mut [u8]) -> usize {
    let len = OUTPUT_BUFFER_LEN.load(Ordering::Acquire);
    let copy_len = len.min(out.len());
    
    for i in 0..copy_len {
        out[i] = OUTPUT_BUFFER[i].load(Ordering::Acquire) as u8;
    }
    
    OUTPUT_BUFFER_LEN.store(0, Ordering::Release);
    copy_len
}

fn append_to_capture(s: &[u8]) {
    if !OUTPUT_CAPTURE_ENABLED.load(Ordering::Acquire) {
        return;
    }
    
    let mut len = OUTPUT_BUFFER_LEN.load(Ordering::Acquire);
    
    for &byte in s {
        if len >= 4096 {
            break;
        }
        OUTPUT_BUFFER[len].store(byte as usize, Ordering::Release);
        len += 1;
    }
    
    OUTPUT_BUFFER_LEN.store(len, Ordering::Release);
}

pub fn print(s: &[u8]) {
    write(STDOUT, s);
    append_to_capture(s);
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
    
    fn len(&self) -> usize {
        let mut len = 0;
        unsafe {
            while len < 4096 && *self.ptr.add(len) != 0 {
                len += 1;
            }
        }
        len
    }
    
    pub fn as_bytes(&self) -> &[u8] {
        let len = self.len();
        unsafe { core::slice::from_raw_parts(self.ptr, len) }
    }
}

pub struct StaticBuffer {
    data: UnsafeCell<[u8; 128]>,
    locked: AtomicBool,
}

unsafe impl Sync for StaticBuffer {}

impl StaticBuffer {
    pub const fn new() -> Self {
        Self {
            data: UnsafeCell::new([0u8; 128]),
            locked: AtomicBool::new(false),
        }
    }
    
    pub fn with_mut<F, R>(&self, f: F) -> R 
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        while self.locked.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
            core::hint::spin_loop();
        }
        
        let result = unsafe {
            let buf = &mut *self.data.get();
            f(buf)
        };
        
        self.locked.store(false, Ordering::Release);
        result
    }
}

pub fn read_line(buf: &mut [u8]) -> usize {
    let n = read(STDIN, buf);
    if n > 0 { n as usize } else { 0 }
}

pub fn read_line_with_tab(buf: &mut [u8]) -> usize {
    use crate::syscalls::ioctl;
    use crate::syscalls::{Termios, TCGETS, TCSETS, ICANON, ECHO};
    
    let mut old_term = Termios {
        c_iflag: 0,
        c_oflag: 0,
        c_cflag: 0,
        c_lflag: 0,
        c_line: 0,
        c_cc: [0u8; 32],
        _padding: [0u8; 3],
        c_ispeed: 0,
        c_ospeed: 0,
    };
    
    let old_ptr = &mut old_term as *mut Termios as u64;
    if ioctl(STDIN, TCGETS, old_ptr) < 0 {
        return read_line(buf);
    }
    
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
        } else if ch == 9 {
            if pos == 1 {
                let ch = buf[0];
                let completion: Option<&[u8]> = match ch {
                    b'l' => Some(b"ls "),
                    b'c' => Some(b"cd "),
                    b'p' => Some(b"pwd"),
                    b'e' => Some(b"echo "),
                    b'a' => Some(b"alias "),
                    b'h' => Some(b"history"),
                    b's' => Some(b"serve "),
                    _ => None,
                };
                
                if let Some(comp) = completion {
                    write(STDOUT, b"\x08 \x08");
                    write(STDOUT, comp);
                    pos = 0;
                    for &b in comp {
                        if pos < buf.len() - 1 {
                            buf[pos] = b;
                            pos += 1;
                        }
                    }
                } else {
                    write(STDOUT, b"\x07");
                }
            } else {
                write(STDOUT, b"\x07");
            }
        } else if ch == 127 || ch == 8 {
            if pos > 0 {
                pos -= 1;
                write(STDOUT, b"\x08 \x08");
            }
        } else if ch == 3 {
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
    
    ioctl(STDIN, TCSETS, old_ptr);
    
    pos
}
