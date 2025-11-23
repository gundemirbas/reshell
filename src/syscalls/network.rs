use super::macros::*;

pub const AF_INET: i32 = 2;
pub const SOCK_STREAM: i32 = 1;
pub const SOL_SOCKET: i32 = 1;
pub const SO_REUSEADDR: i32 = 2;

#[repr(C)]
pub struct SockaddrIn {
    pub sin_family: u16,
    pub sin_port: u16,
    pub sin_addr: u32,
    pub sin_zero: [u8; 8],
}

pub fn socket(domain: i32, socket_type: i32, protocol: i32) -> isize {
    syscall3!(41, domain, socket_type, protocol)
}

pub fn bind(sockfd: i32, addr: &SockaddrIn) -> isize {
    syscall3!(49, sockfd, addr as *const SockaddrIn, 16)
}

pub fn listen(sockfd: i32, backlog: i32) -> isize {
    syscall2!(50, sockfd, backlog)
}

pub fn accept(sockfd: i32) -> isize {
    syscall3!(43, sockfd, 0, 0)
}

pub fn setsockopt(sockfd: i32, level: i32, optname: i32, optval: i32) -> isize {
    syscall5!(54, sockfd, level, optname, &optval as *const i32, 4)
}
