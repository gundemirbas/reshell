use core::sync::atomic::{AtomicUsize, AtomicBool, Ordering};

const MAX_INPUT: usize = 512;
const MAX_OUTPUT: usize = 4096;

pub struct ShellSession {
    // Input buffer (commands from client)
    input_buffer: [AtomicUsize; MAX_INPUT],
    input_len: AtomicUsize,
    
    // Output buffer (shell output to client)
    output_buffer: [AtomicUsize; MAX_OUTPUT],
    output_write_pos: AtomicUsize,
    output_read_pos: AtomicUsize,
    
    // Session state
    active: AtomicBool,
}

impl ShellSession {
    pub const fn new() -> Self {
        Self {
            input_buffer: [const { AtomicUsize::new(0) }; MAX_INPUT],
            input_len: AtomicUsize::new(0),
            output_buffer: [const { AtomicUsize::new(0) }; MAX_OUTPUT],
            output_write_pos: AtomicUsize::new(0),
            output_read_pos: AtomicUsize::new(0),
            active: AtomicBool::new(false),
        }
    }
    
    pub fn activate(&self) {
        self.input_len.store(0, Ordering::Release);
        self.output_write_pos.store(0, Ordering::Release);
        self.output_read_pos.store(0, Ordering::Release);
        self.active.store(true, Ordering::Release);
    }
    
    pub fn deactivate(&self) {
        self.active.store(false, Ordering::Release);
    }
    
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }
    
    // Input methods (stdin simulation)
    pub fn append_input(&self, ch: u8) {
        let len = self.input_len.load(Ordering::Acquire);
        if len < MAX_INPUT {
            self.input_buffer[len].store(ch as usize, Ordering::Release);
            self.input_len.store(len + 1, Ordering::Release);
        }
    }
    
    pub fn get_input(&self, out: &mut [u8]) -> usize {
        let len = self.input_len.load(Ordering::Acquire).min(out.len());
        for i in 0..len {
            out[i] = self.input_buffer[i].load(Ordering::Acquire) as u8;
        }
        len
    }
    
    pub fn clear_input(&self) {
        self.input_len.store(0, Ordering::Release);
    }
    
    pub fn backspace_input(&self) {
        let len = self.input_len.load(Ordering::Acquire);
        if len > 0 {
            self.input_len.store(len - 1, Ordering::Release);
        }
    }
    
    pub fn input_len(&self) -> usize {
        self.input_len.load(Ordering::Acquire)
    }
    
    // Output methods (stdout simulation)
    pub fn write_output(&self, data: &[u8]) {
        let mut write_pos = self.output_write_pos.load(Ordering::Acquire);
        
        for &byte in data {
            if write_pos >= MAX_OUTPUT {
                break;
            }
            self.output_buffer[write_pos].store(byte as usize, Ordering::Release);
            write_pos += 1;
        }
        
        self.output_write_pos.store(write_pos, Ordering::Release);
    }
    
    pub fn read_output(&self, out: &mut [u8]) -> usize {
        let read_pos = self.output_read_pos.load(Ordering::Acquire);
        let write_pos = self.output_write_pos.load(Ordering::Acquire);
        
        if read_pos >= write_pos {
            return 0;
        }
        
        let available = write_pos - read_pos;
        let copy_len = available.min(out.len());
        
        for i in 0..copy_len {
            out[i] = self.output_buffer[read_pos + i].load(Ordering::Acquire) as u8;
        }
        
        self.output_read_pos.store(read_pos + copy_len, Ordering::Release);
        copy_len
    }
    
    pub fn has_output(&self) -> bool {
        self.output_read_pos.load(Ordering::Acquire) < self.output_write_pos.load(Ordering::Acquire)
    }
}

// Global session pool
const MAX_SESSIONS: usize = 16;
static SESSIONS: [ShellSession; MAX_SESSIONS] = [const { ShellSession::new() }; MAX_SESSIONS];

pub fn get_session(idx: usize) -> Option<&'static ShellSession> {
    if idx < MAX_SESSIONS {
        Some(&SESSIONS[idx])
    } else {
        None
    }
}

pub fn allocate_session() -> Option<usize> {
    for i in 0..MAX_SESSIONS {
        if !SESSIONS[i].is_active() {
            SESSIONS[i].activate();
            return Some(i);
        }
    }
    None
}

pub fn free_session(idx: usize) {
    if idx < MAX_SESSIONS {
        SESSIONS[idx].deactivate();
    }
}
