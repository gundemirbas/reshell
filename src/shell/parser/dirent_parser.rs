use crate::syscalls::LinuxDirent64;

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
        
        let remaining = self.buf.len() - self.pos;
        if remaining < core::mem::size_of::<LinuxDirent64>() {
            return None;
        }
        
        let reclen = unsafe {
            let dirent_ptr = self.buf.as_ptr().add(self.pos) as *const LinuxDirent64;
            (*dirent_ptr).d_reclen as usize
        };
        
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
        let max_end = core::cmp::min(self.pos + reclen, self.buf.len());
        
        while name_end < max_end && self.buf[name_end] != 0 {
            name_end += 1;
        }
        
        let entry = DirentEntry {
            name: &self.buf[name_start..name_end],
        };
        
        self.pos += reclen;
        Some(entry)
    }
}

pub struct DirentEntry<'a> {
    pub name: &'a [u8],
}
