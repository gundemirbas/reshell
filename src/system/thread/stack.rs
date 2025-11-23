use core::sync::atomic::{AtomicPtr, Ordering};
use crate::syscalls::{sys_mmap, PROT_READ, PROT_WRITE, MAP_PRIVATE, MAP_ANONYMOUS, MAP_STACK};

pub struct ThreadStack {
    ptr: AtomicPtr<u8>,
    size: usize,
}

impl ThreadStack {
    pub const fn new() -> Self {
        Self {
            ptr: AtomicPtr::new(core::ptr::null_mut()),
            size: 131072,  // 128 KB
        }
    }
    
    pub const fn new_large() -> Self {
        Self {
            ptr: AtomicPtr::new(core::ptr::null_mut()),
            size: 524288,  // 512 KB for server threads
        }
    }
    
    pub fn allocate(&self) -> bool {
        let current = self.ptr.load(Ordering::Acquire);
        if !current.is_null() {
            return true;
        }
        
        let allocated = unsafe {
            sys_mmap(
                core::ptr::null_mut(),
                self.size,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS | MAP_STACK,
                -1,
                0
            )
        };
        
        if allocated as isize == -1 || allocated.is_null() {
            return false;
        }
        
        self.ptr.store(allocated, Ordering::Release);
        true
    }
    
    pub fn get_stack_top(&self) -> *mut u8 {
        let ptr = self.ptr.load(Ordering::Acquire);
        if ptr.is_null() {
            return core::ptr::null_mut();
        }
        
        unsafe {
            let stack_top = ptr.add(self.size);
            (stack_top as usize & !15) as *mut u8
        }
    }
}
