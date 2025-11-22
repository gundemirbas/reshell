use core::cell::UnsafeCell;
use crate::syscalls::{gettid, nanosleep, write, sys_clone_with_func};
use crate::syscalls::{STDOUT, CLONE_VM, CLONE_FS, CLONE_FILES, CLONE_SIGHAND, CLONE_THREAD};
use crate::io::print;

pub struct StaticTid {
    tid: UnsafeCell<i32>,
}

unsafe impl Sync for StaticTid {}

impl StaticTid {
    pub const fn new() -> Self {
        Self {
            tid: UnsafeCell::new(0),
        }
    }
    
    pub unsafe fn set(&self, value: i32) {
        unsafe { *self.tid.get() = value; }
    }
    
    pub unsafe fn get(&self) -> i32 {
        unsafe { *self.tid.get() }
    }
}

pub static TICKER_TID: StaticTid = StaticTid::new();

pub struct ThreadStack {
    data: UnsafeCell<[u8; 16384]>,
}

unsafe impl Sync for ThreadStack {}

impl ThreadStack {
    const fn new() -> Self {
        Self {
            data: UnsafeCell::new([0u8; 16384]),
        }
    }
    
    fn with_mut<F, R>(&self, f: F) -> R 
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        unsafe {
            let buf = &mut *self.data.get();
            f(buf)
        }
    }
}

static THREAD_STACK: ThreadStack = ThreadStack::new();

fn ticker_func() -> ! {
    let tid = gettid();
    unsafe {
        TICKER_TID.set(tid as i32);
    }
    
    loop {
        write(STDOUT, b"#");
        nanosleep(10, 0);
    }
}

pub fn start_ticker_thread() {
    let flags = CLONE_VM | CLONE_FS | CLONE_FILES | 
                CLONE_SIGHAND | CLONE_THREAD;
    
    let tid = THREAD_STACK.with_mut(|stack| {
        let stack_size = stack.len();
        let stack_ptr = stack.as_mut_ptr();
        unsafe {
            let stack_top = stack_ptr.add(stack_size);
            let aligned = (stack_top as usize & !15) as *mut u8;
            sys_clone_with_func(flags, aligned, ticker_func)
        }
    });
    
    if tid < 0 {
        print(b"[Warning: Could not start ticker thread]\n");
    }
}
