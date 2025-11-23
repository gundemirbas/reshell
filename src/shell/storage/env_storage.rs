use core::sync::atomic::{AtomicBool, AtomicUsize, AtomicPtr, Ordering};

pub static ENV_STORAGE: EnvStorage = EnvStorage::new();

pub struct EnvStorage {
    vars: AtomicPtr<[[u8; 256]; 32]>,
    count: AtomicUsize,
    initialized: AtomicBool,
}

impl EnvStorage {
    pub const fn new() -> Self {
        Self {
            vars: AtomicPtr::new(core::ptr::null_mut()),
            count: AtomicUsize::new(0),
            initialized: AtomicBool::new(false),
        }
    }
    
    fn ensure_init(&self) {
        if self.initialized.load(Ordering::Acquire) {
            return;
        }
        
        use crate::syscalls::{sys_mmap, PROT_READ, PROT_WRITE, MAP_PRIVATE, MAP_ANONYMOUS, write, STDERR};
        
        let size = core::mem::size_of::<[[u8; 256]; 32]>();
        let ptr = unsafe {
            sys_mmap(
                core::ptr::null_mut(),
                size,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0
            )
        } as *mut [[u8; 256]; 32];
        
        if ptr.is_null() || ptr as isize == -1 {
            write(STDERR, b"[ERROR] Failed to allocate ENV_STORAGE\n");
            return;
        }
        
        unsafe {
            for i in 0..32 {
                for j in 0..256 {
                    (*ptr)[i][j] = 0;
                }
            }
        }
        
        self.vars.store(ptr, Ordering::Release);
        self.initialized.store(true, Ordering::Release);
    }
    
    pub fn set(&self, name: &[u8], value: &[u8]) -> bool {
        self.ensure_init();
        
        let vars_ptr = self.vars.load(Ordering::Acquire);
        if vars_ptr.is_null() {
            return false;
        }
        
        let count = self.count.load(Ordering::Acquire);
        if count >= 32 {
            return false;
        }
        
        unsafe {
            let vars = &mut *vars_ptr;
            let buf = &mut vars[count];
            
            for i in 0..256 {
                buf[i] = 0;
            }
            
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
        }
        
        self.count.store(count + 1, Ordering::Release);
        true
    }
    
    pub fn get(&self, name: &[u8], out_buf: &mut [u8]) -> usize {
        self.ensure_init();
        
        let vars_ptr = self.vars.load(Ordering::Acquire);
        if vars_ptr.is_null() {
            return 0;
        }
        
        let count = self.count.load(Ordering::Acquire);
        
        unsafe {
            let vars = &*vars_ptr;
            
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
        }
        0
    }
    
    pub fn iter<F>(&self, mut f: F) where F: FnMut(&[u8]) {
        self.ensure_init();
        
        let vars_ptr = self.vars.load(Ordering::Acquire);
        if vars_ptr.is_null() {
            return;
        }
        
        let count = self.count.load(Ordering::Acquire);
        
        unsafe {
            let vars = &*vars_ptr;
            
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

