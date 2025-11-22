use core::cell::UnsafeCell;
use crate::syscalls::{write, STDOUT};
use crate::io::print_number;

pub static ENV_STORAGE: EnvStorage = EnvStorage::new();
pub static HISTORY: HistoryStorage = HistoryStorage::new();
pub static ALIASES: AliasStorage = AliasStorage::new();

pub struct EnvStorage {
    vars: UnsafeCell<[[u8; 256]; 32]>,
    count: UnsafeCell<usize>,
}

unsafe impl Sync for EnvStorage {}

impl EnvStorage {
    pub const fn new() -> Self {
        Self {
            vars: UnsafeCell::new([[0u8; 256]; 32]),
            count: UnsafeCell::new(0),
        }
    }
    
    pub fn set(&self, name: &[u8], value: &[u8]) -> bool {
        unsafe {
            let count = *self.count.get();
            if count >= 32 {
                return false;
            }
            
            let vars = &mut *self.vars.get();
            let mut buf = [0u8; 256];
            let mut idx = 0;
            
            for &b in name {
                if idx >= 128 { return false; }
                buf[idx] = b;
                idx += 1;
            }
            buf[idx] = b'=';
            idx += 1;
            
            for &b in value {
                if idx >= 255 { return false; }
                buf[idx] = b;
                idx += 1;
            }
            
            vars[count] = buf;
            *self.count.get() = count + 1;
            true
        }
    }
    
    pub fn get(&self, name: &[u8], out_buf: &mut [u8]) -> usize {
        unsafe {
            let count = *self.count.get();
            let vars = &*self.vars.get();
            
            for i in 0..count {
                let var = &vars[i];
                let mut j = 0;
                let mut matched = true;
                
                while j < name.len() && j < 256 && var[j] != 0 {
                    if var[j] != name[j] {
                        matched = false;
                        break;
                    }
                    j += 1;
                }
                
                if matched && var[j] == b'=' {
                    j += 1;
                    let mut idx = 0;
                    while var[j] != 0 && idx < out_buf.len() {
                        out_buf[idx] = var[j];
                        idx += 1;
                        j += 1;
                    }
                    return idx;
                }
            }
            0
        }
    }
    
    pub fn iter<F>(&self, mut f: F) where F: FnMut(&[u8]) {
        unsafe {
            let count = *self.count.get();
            let vars = &*self.vars.get();
            
            for i in 0..count {
                let var = &vars[i];
                let mut len = 0;
                while len < 256 && var[len] != 0 {
                    len += 1;
                }
                if len > 0 {
                    f(&var[..len]);
                }
            }
        }
    }
}

pub struct HistoryStorage {
    entries: UnsafeCell<[[u8; 128]; 10]>,
    count: UnsafeCell<usize>,
    index: UnsafeCell<usize>,
}

unsafe impl Sync for HistoryStorage {}

impl HistoryStorage {
    pub const fn new() -> Self {
        Self {
            entries: UnsafeCell::new([[0u8; 128]; 10]),
            count: UnsafeCell::new(0),
            index: UnsafeCell::new(0),
        }
    }
    
    pub fn add(&self, cmd: &[u8]) {
        if cmd.is_empty() || cmd.len() > 127 {
            return;
        }
        
        unsafe {
            let entries = &mut *self.entries.get();
            let count = *self.count.get();
            let index = *self.index.get();
            
            // Manual copy instead of copy_from_slice
            for i in 0..cmd.len().min(127) {
                entries[index][i] = cmd[i];
            }
            entries[index][cmd.len().min(127)] = 0;
            
            *self.index.get() = (index + 1) % 10;
            if count < 10 {
                *self.count.get() = count + 1;
            }
        }
    }
    
    pub fn list(&self) {
        unsafe {
            let entries = &*self.entries.get();
            let count = *self.count.get();
            let index = *self.index.get();
            
            if count == 0 {
                write(STDOUT, b"No history\n");
                return;
            }
            
            for i in 0..count {
                let idx = if count < 10 {
                    i
                } else {
                    (index + i) % 10
                };
                
                let entry = &entries[idx];
                let mut len = 0;
                while len < 128 && entry[len] != 0 {
                    len += 1;
                }
                
                if len > 0 {
                    write(STDOUT, b"  ");
                    print_number((i + 1) as i64);
                    write(STDOUT, b": ");
                    write(STDOUT, &entry[..len]);
                    write(STDOUT, b"\n");
                }
            }
        }
    }
}

pub struct AliasStorage {
    aliases: UnsafeCell<[([u8; 32], [u8; 128]); 16]>,
    count: UnsafeCell<usize>,
}

unsafe impl Sync for AliasStorage {}

impl AliasStorage {
    pub const fn new() -> Self {
        Self {
            aliases: UnsafeCell::new([([0u8; 32], [0u8; 128]); 16]),
            count: UnsafeCell::new(0),
        }
    }
    
    pub fn set(&self, name: &[u8], value: &[u8]) -> bool {
        if name.is_empty() || name.len() > 31 || value.len() > 127 {
            return false;
        }
        
        unsafe {
            let aliases = &mut *self.aliases.get();
            let count = &mut *self.count.get();
            
            for i in 0..*count {
                let (alias_name, alias_value) = &mut aliases[i];
                let mut matched = true;
                let check_len = name.len().min(32);
                for j in 0..check_len {
                    if alias_name[j] != name[j] {
                        matched = false;
                        break;
                    }
                }
                if matched && (name.len() >= 31 || (name.len() < 32 && alias_name[name.len()] == 0)) {
                    *alias_value = [0u8; 128];
                    for i in 0..value.len() {
                        alias_value[i] = value[i];
                    }
                    return true;
                }
            }
            
            if *count >= 16 {
                return false;
            }
            
            let (alias_name, alias_value) = &mut aliases[*count];
            *alias_name = [0u8; 32];
            *alias_value = [0u8; 128];
            
            for i in 0..name.len() {
                alias_name[i] = name[i];
            }
            for i in 0..value.len() {
                alias_value[i] = value[i];
            }
            
            *count += 1;
            true
        }
    }
    
    pub fn get(&self, name: &[u8], output: &mut [u8]) -> usize {
        unsafe {
            let aliases = &*self.aliases.get();
            let count = *self.count.get();
            
            for i in 0..count {
                let (alias_name, alias_value) = &aliases[i];
                let mut matched = true;
                let check_len = name.len().min(32);
                for j in 0..check_len {
                    if alias_name[j] != name[j] {
                        matched = false;
                        break;
                    }
                }
                if matched && (name.len() >= 31 || (name.len() < 32 && alias_name[name.len()] == 0)) {
                    let mut len = 0;
                    while len < 128 && alias_value[len] != 0 {
                        len += 1;
                    }
                    let copy_len = len.min(output.len());
                    for i in 0..copy_len {
                        output[i] = alias_value[i];
                    }
                    return copy_len;
                }
            }
            0
        }
    }
    
    pub fn list(&self) {
        unsafe {
            let aliases = &*self.aliases.get();
            let count = *self.count.get();
            
            if count == 0 {
                write(STDOUT, b"No aliases defined\n");
                return;
            }
            
            for i in 0..count {
                let (alias_name, alias_value) = &aliases[i];
                
                let mut name_len = 0;
                while name_len < 32 && alias_name[name_len] != 0 {
                    name_len += 1;
                }
                
                let mut value_len = 0;
                while value_len < 128 && alias_value[value_len] != 0 {
                    value_len += 1;
                }
                
                if name_len > 0 && value_len > 0 {
                    write(STDOUT, b"alias ");
                    write(STDOUT, &alias_name[..name_len]);
                    write(STDOUT, b"='");
                    write(STDOUT, &alias_value[..value_len]);
                    write(STDOUT, b"'\n");
                }
            }
        }
    }
}
