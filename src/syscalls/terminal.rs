pub const TCGETS: u64 = 0x5401;
pub const TCSETS: u64 = 0x5402;
pub const ICANON: u32 = 0x00000002;
pub const ECHO: u32 = 0x00000008;

#[repr(C)]
pub struct Termios {
    pub c_iflag: u32,
    pub c_oflag: u32,
    pub c_cflag: u32,
    pub c_lflag: u32,
    pub c_line: u8,
    pub c_cc: [u8; 32],
    pub _padding: [u8; 3],
    pub c_ispeed: u32,
    pub c_ospeed: u32,
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

pub fn ioctl(fd: i32, request: u64, arg: u64) -> isize {
    syscall3!(16, fd, request, arg)
}
