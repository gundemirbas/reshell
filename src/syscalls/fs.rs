macro_rules! syscall1 {
    ($num:expr, $arg1:expr) => {{
        let ret: isize;
        unsafe {
            core::arch::asm!(
                "syscall",
                in("rax") $num,
                in("rdi") $arg1,
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

pub fn chdir(path: &[u8]) -> isize {
    syscall1!(80, path.as_ptr())
}

pub fn getcwd(buf: &mut [u8]) -> isize {
    let ret = syscall2!(79, buf.as_mut_ptr(), buf.len());
    
    if ret > 0 {
        buf.iter().position(|&b| b == 0).unwrap_or(buf.len()) as isize
    } else {
        ret
    }
}

#[repr(C)]
pub struct LinuxDirent64 {
    pub d_ino: u64,
    pub d_off: i64,
    pub d_reclen: u16,
    pub d_type: u8,
    pub d_name: [u8; 0],
}

pub fn getdents64(fd: i32, buf: &mut [u8]) -> isize {
    syscall3!(217, fd, buf.as_mut_ptr(), buf.len())
}
