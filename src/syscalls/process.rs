macro_rules! syscall0 {
    ($num:expr) => {{
        let ret: isize;
        unsafe {
            core::arch::asm!(
                "syscall",
                in("rax") $num,
                lateout("rax") ret,
                lateout("rcx") _,
                lateout("r11") _,
            );
        }
        ret
    }};
}

macro_rules! syscall3 {
    ($num:expr, $arg1:expr, $arg2:expr, $arg3:expr) => {{
        let ret: isize;
        unsafe {
            core::arch::asm!(
                "syscall",
                in("rax") $num,
                in("rdi") $arg1,
                in("rsi") $arg2,
                in("rdx") $arg3,
                lateout("rax") ret,
                lateout("rcx") _,
                lateout("r11") _,
            );
        }
        ret
    }};
}

macro_rules! syscall2 {
    ($num:expr, $arg1:expr, $arg2:expr) => {{
        let ret: isize;
        unsafe {
            core::arch::asm!(
                "syscall",
                in("rax") $num,
                in("rdi") $arg1,
                in("rsi") $arg2,
                lateout("rax") ret,
                lateout("rcx") _,
                lateout("r11") _,
            );
        }
        ret
    }};
}

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

pub unsafe fn sys_clone_with_func(
    flags: u64,
    stack: *mut u8,
    func: fn() -> !,
) -> isize {
    let func_ptr = func as usize;
    let aligned_top = (stack as usize) & !0xF;
    let func_slot = (aligned_top - 8) as *mut usize;
    
    *func_slot = func_ptr;
    
    let child_stack = func_slot as *mut u8;
    let ret: isize;
    
    core::arch::asm!(
        "mov rax, 56",
        "syscall",
        "test rax, rax",
        "jnz 2f",
        "pop rax",
        "xor rbp, rbp",
        "and rsp, -16",
        "push rbp",
        "jmp rax",
        "2:",
        in("rdi") flags,
        in("rsi") child_stack,
        in("rdx") 0u64,
        in("r10") 0u64,
        in("r8") 0u64,
        lateout("rax") ret,
        lateout("rcx") _,
        lateout("r11") _,
        lateout("rdi") _,
        lateout("rsi") _,
        lateout("rdx") _,
        lateout("r10") _,
        lateout("r8") _,
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
