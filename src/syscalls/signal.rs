use super::macros::*;
use core::sync::atomic::{AtomicBool, Ordering};

pub static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

pub fn request_shutdown() {
    SHUTDOWN_REQUESTED.store(true, Ordering::Release);
}

pub const SIGINT: i32 = 2;
pub const SIGTERM: i32 = 15;
pub const SIGPIPE: i32 = 13;
pub const SIG_IGN: usize = 1;

#[repr(C)]
pub struct SigAction {
    pub sa_handler: usize,
    pub sa_flags: u64,
    pub sa_restorer: usize,
    pub sa_mask: [u64; 16],
}

impl SigAction {
    pub const fn new() -> Self {
        Self {
            sa_handler: 0,
            sa_flags: 0,
            sa_restorer: 0,
            sa_mask: [0; 16],
        }
    }
}

pub fn rt_sigaction(signum: i32, act: *const SigAction, oldact: *mut SigAction) -> isize {
    syscall4!(13, signum, act, oldact, 8)
}

extern "C" fn signal_restorer() {
    unsafe {
        core::arch::asm!(
            "mov rax, 15",
            "syscall",
            options(noreturn)
        );
    }
}

extern "C" fn signal_handler(_signum: i32) {
    SHUTDOWN_REQUESTED.store(true, Ordering::Release);
}

pub fn setup_signal_handlers() -> bool {
    let mut sa = SigAction::new();
    sa.sa_handler = signal_handler as usize;
    sa.sa_flags = 0x04000000; // SA_RESTORER
    sa.sa_restorer = signal_restorer as usize;
    
    if rt_sigaction(SIGINT, &sa as *const SigAction, core::ptr::null_mut()) < 0 {
        return false;
    }
    
    if rt_sigaction(SIGTERM, &sa as *const SigAction, core::ptr::null_mut()) < 0 {
        return false;
    }
    
    let mut ignore = SigAction::new();
    ignore.sa_handler = SIG_IGN;
    ignore.sa_flags = 0x04000000;
    ignore.sa_restorer = signal_restorer as usize;
    
    if rt_sigaction(SIGPIPE, &ignore as *const SigAction, core::ptr::null_mut()) < 0 {
        return false;
    }
    
    true
}

pub fn should_shutdown() -> bool {
    SHUTDOWN_REQUESTED.load(Ordering::Acquire)
}
