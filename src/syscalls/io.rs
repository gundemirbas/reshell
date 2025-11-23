use super::macros::*;

pub const STDIN: i32 = 0;
pub const STDOUT: i32 = 1;
pub const STDERR: i32 = 2;

pub fn write(fd: i32, buf: &[u8]) -> isize {
    syscall3!(1, fd, buf.as_ptr(), buf.len())
}

pub fn read(fd: i32, buf: &mut [u8]) -> isize {
    syscall3!(0, fd, buf.as_mut_ptr(), buf.len())
}

pub const O_RDONLY: i32 = 0;
pub const O_DIRECTORY: i32 = 0x10000;

pub fn open(path: &[u8], flags: i32) -> isize {
    syscall3!(2, path.as_ptr(), flags, 0)
}

pub fn close(fd: i32) -> isize {
    syscall1!(3, fd)
}

#[repr(C)]
pub struct PollFd {
    pub fd: i32,
    pub events: i16,
    pub revents: i16,
}

pub const POLLIN: i16 = 0x001;

pub fn poll(fds: &mut [PollFd], timeout: i32) -> isize {
    syscall3!(7, fds.as_mut_ptr(), fds.len(), timeout)
}
