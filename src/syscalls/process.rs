use super::macros::*;

pub fn sys_exit(code: i32) -> ! {
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 60,
            in("rdi") code,
            options(noreturn)
        );
    }
}

pub fn fork() -> isize {
    syscall0!(57)
}

pub fn execve(path: &[u8], argv: &[*const u8]) -> isize {
    syscall3!(59, path.as_ptr(), argv.as_ptr(), core::ptr::null::<*const u8>())
}

pub fn waitpid(pid: i32, status: &mut i32) -> isize {
    syscall3!(61, pid, status as *mut i32, 0)
}

pub const CLONE_VM: u64 = 0x00000100;
pub const CLONE_FS: u64 = 0x00000200;
pub const CLONE_FILES: u64 = 0x00000400;
pub const CLONE_SIGHAND: u64 = 0x00000800;
pub const CLONE_THREAD: u64 = 0x00010000;

// Thread wrapper that calls the actual function
extern "C" fn thread_wrapper() -> ! {
    unsafe {
        // Function pointer is passed via a global static
        if let Some(func) = THREAD_FUNC.take() {
            func();
        }
    }
    sys_exit(1);
}

static mut THREAD_FUNC: Option<fn() -> !> = None;

pub unsafe fn sys_clone_with_func(
    flags: u64,
    stack: *mut u8,
    func: fn() -> !,
) -> isize {
    // Store function pointer in static
    THREAD_FUNC = Some(func);
    
    // Align stack to 16 bytes
    let aligned_stack = ((stack as usize) & !0xF) as *mut u8;
    
    // Call clone syscall with wrapper function
    let ret: isize;
    
    core::arch::asm!(
        "mov rax, 56",           // clone syscall number
        "syscall",
        "test rax, rax",         // Check if we're in child (rax == 0)
        "jnz 2f",                // If parent, jump to return
        // Child process execution
        "xor rbp, rbp",          // Clear frame pointer
        "call {wrapper}",         // Call wrapper function
        "2:",                     // Parent returns here
        wrapper = sym thread_wrapper,
        in("rdi") flags,
        in("rsi") aligned_stack,
        in("rdx") 0u64,
        in("r10") 0u64,
        in("r8") 0u64,
        lateout("rax") ret,
        lateout("rcx") _,
        lateout("r11") _,
    );
    
    ret
}

#[repr(C)]
pub struct TimeSpec {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}

pub fn nanosleep(seconds: i64, nanoseconds: i64) -> isize {
    let req = TimeSpec {
        tv_sec: seconds,
        tv_nsec: nanoseconds,
    };
    syscall2!(35, &req as *const TimeSpec, core::ptr::null_mut::<TimeSpec>())
}

pub fn getpid() -> i32 {
    syscall0!(39) as i32
}

pub fn tgkill(tgid: i32, tid: i32, sig: i32) -> isize {
    syscall3!(234, tgid, tid, sig)
}
