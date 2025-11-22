pub const STDIN: i32 = 0;
pub const STDOUT: i32 = 1;
pub const STDERR: i32 = 2;

unsafe fn sys_write_raw(fd: i32, buf: *const u8, count: usize) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 1,
            in("rdi") fd,
            in("rsi") buf,
            in("rdx") count,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
        );
    }
    ret
}

unsafe fn sys_read_raw(fd: i32, buf: *mut u8, count: usize) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 0,
            in("rdi") fd,
            in("rsi") buf,
            in("rdx") count,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
        );
    }
    ret
}

unsafe fn sys_fork_raw() -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 57,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
        );
    }
    ret
}

unsafe fn sys_execve_raw(filename: *const u8, argv: *const *const u8, envp: *const *const u8) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 59,
            in("rdi") filename,
            in("rsi") argv,
            in("rdx") envp,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
        );
    }
    ret
}

unsafe fn sys_waitpid_raw(pid: i32, status: *mut i32, options: i32) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 61,
            in("rdi") pid,
            in("rsi") status,
            in("rdx") options,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
        );
    }
    ret
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

// Safe wrapper functions
pub fn write(fd: i32, buf: &[u8]) -> isize {
    unsafe { sys_write_raw(fd, buf.as_ptr(), buf.len()) }
}

pub fn read(fd: i32, buf: &mut [u8]) -> isize {
    unsafe { sys_read_raw(fd, buf.as_mut_ptr(), buf.len()) }
}

pub fn fork() -> isize {
    unsafe { sys_fork_raw() }
}

pub fn execve(path: &[u8], argv: &[*const u8]) -> isize {
    let envp: [*const u8; 1] = [core::ptr::null()];
    unsafe { sys_execve_raw(path.as_ptr(), argv.as_ptr(), envp.as_ptr()) }
}

pub fn waitpid(pid: i32, status: &mut i32) -> isize {
    unsafe { sys_waitpid_raw(pid, status as *mut i32, 0) }
}

// Clone syscall for thread creation
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
    // Stack'e fonksiyon pointer'ını yaz
    let stack_top = stack as *mut u64;
    let func_slot = unsafe { stack_top.offset(-2) };  // 2 word geriye (16 byte alignment için)
    unsafe { *func_slot = func as u64 };
    
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "mov rax, 56",      // clone syscall
            "syscall",
            "test rax, rax",
            "jnz 2f",           // parent branch
            
            // Child thread
            "xor rbp, rbp",     // clear frame pointer  
            "pop rax",          // get function from stack
            "call rax",         // call it
            
            // Exit if function returns (shouldn't happen)
            "mov rax, 60",
            "xor rdi, rdi",
            "syscall",
            
            "2:",               // parent continues
            in("rdi") flags,
            in("rsi") func_slot,
            in("rdx") 0u64,
            in("r10") 0u64,
            in("r8") 0u64,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
        );
    }
    ret
}

// nanosleep syscall
#[repr(C)]
pub struct TimeSpec {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}

unsafe fn sys_nanosleep_raw(req: *const TimeSpec, rem: *mut TimeSpec) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 35,
            in("rdi") req,
            in("rsi") rem,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
        );
    }
    ret
}

pub fn nanosleep(seconds: i64, nanoseconds: i64) -> isize {
    let req = TimeSpec {
        tv_sec: seconds,
        tv_nsec: nanoseconds,
    };
    unsafe { sys_nanosleep_raw(&req, core::ptr::null_mut()) }
}

// Signal syscalls
pub const SIGUSR1: i32 = 10;

unsafe fn sys_kill_raw(pid: i32, sig: i32) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 62,
            in("rdi") pid,
            in("rsi") sig,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
        );
    }
    ret
}

pub fn kill(pid: i32, sig: i32) -> isize {
    unsafe { sys_kill_raw(pid, sig) }
}

unsafe fn sys_gettid_raw() -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 186,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
        );
    }
    ret
}

pub fn gettid() -> isize {
    unsafe { sys_gettid_raw() }
}

// chdir syscall for cd command
unsafe fn sys_chdir_raw(path: *const u8) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 80,
            in("rdi") path,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
        );
    }
    ret
}

pub fn chdir(path: &[u8]) -> isize {
    unsafe { sys_chdir_raw(path.as_ptr()) }
}

// getcwd syscall
// Returns the length of the path on success, or negative error code on failure
unsafe fn sys_getcwd_raw(buf: *mut u8, size: usize) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 79,
            in("rdi") buf,
            in("rsi") size,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
        );
    }
    ret
}

// getcwd returns the buffer pointer on success (which we convert to length)
// or negative error code on failure
pub fn getcwd(buf: &mut [u8]) -> isize {
    let buf_ptr = buf.as_mut_ptr();
    let ret = unsafe { sys_getcwd_raw(buf_ptr, buf.len()) };
    
    // If ret is positive (pointer), calculate the string length
    if ret > 0 {
        // Find the null terminator
        let mut len = 0;
        while len < buf.len() && buf[len] != 0 {
            len += 1;
        }
        len as isize
    } else {
        ret // Return error code
    }
}

// open syscall for directory
pub const O_RDONLY: i32 = 0;
pub const O_WRONLY: i32 = 1;
pub const O_RDWR: i32 = 2;
pub const O_CREAT: i32 = 0x40;
pub const O_TRUNC: i32 = 0x200;
pub const O_DIRECTORY: i32 = 0x10000;

unsafe fn sys_open_raw(path: *const u8, flags: i32, mode: i32) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 2,
            in("rdi") path,
            in("rsi") flags,
            in("rdx") mode,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
        );
    }
    ret
}

pub fn open(path: &[u8], flags: i32) -> isize {
    unsafe { sys_open_raw(path.as_ptr(), flags, 0) }
}

pub fn open_with_mode(path: &[u8], flags: i32, mode: i32) -> isize {
    unsafe { sys_open_raw(path.as_ptr(), flags, mode) }
}

// close syscall
unsafe fn sys_close_raw(fd: i32) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 3,
            in("rdi") fd,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
        );
    }
    ret
}

pub fn close(fd: i32) -> isize {
    unsafe { sys_close_raw(fd) }
}

// getdents64 syscall for listing directories
#[repr(C)]
pub struct LinuxDirent64 {
    pub d_ino: u64,
    pub d_off: i64,
    pub d_reclen: u16,
    pub d_type: u8,
    pub d_name: [u8; 0],
}

unsafe fn sys_getdents64_raw(fd: i32, dirp: *mut u8, count: usize) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 217,
            in("rdi") fd,
            in("rsi") dirp,
            in("rdx") count,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
        );
    }
    ret
}

pub fn getdents64(fd: i32, buf: &mut [u8]) -> isize {
    unsafe { sys_getdents64_raw(fd, buf.as_mut_ptr(), buf.len()) }
}

// ioctl for terminal settings (for raw mode)
unsafe fn sys_ioctl_raw(fd: i32, request: u64, arg: u64) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 16,
            in("rdi") fd,
            in("rsi") request,
            in("rdx") arg,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
        );
    }
    ret
}

pub fn ioctl(fd: i32, request: u64, arg: u64) -> isize {
    unsafe { sys_ioctl_raw(fd, request, arg) }
}

// Terminal constants
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
    pub c_ispeed: u32,
    pub c_ospeed: u32,
}

// stat syscall for file info
#[repr(C)]
pub struct Stat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_nlink: u64,
    pub st_mode: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub __pad0: u32,
    pub st_rdev: u64,
    pub st_size: i64,
    pub st_blksize: i64,
    pub st_blocks: i64,
    pub st_atime: i64,
    pub st_atime_nsec: i64,
    pub st_mtime: i64,
    pub st_mtime_nsec: i64,
    pub st_ctime: i64,
    pub st_ctime_nsec: i64,
    pub __unused: [i64; 3],
}

pub const S_IFMT: u32 = 0o170000;
pub const S_IFDIR: u32 = 0o040000;

unsafe fn sys_stat_raw(pathname: *const u8, statbuf: *mut Stat) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 4,
            in("rdi") pathname,
            in("rsi") statbuf,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
        );
    }
    ret
}

pub fn stat(path: &[u8], statbuf: &mut Stat) -> isize {
    unsafe { sys_stat_raw(path.as_ptr(), statbuf as *mut Stat) }
}

// access syscall
pub const F_OK: i32 = 0;
pub const X_OK: i32 = 1;

unsafe fn sys_access_raw(pathname: *const u8, mode: i32) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 21,
            in("rdi") pathname,
            in("rsi") mode,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
        );
    }
    ret
}

pub fn access(path: &[u8], mode: i32) -> isize {
    unsafe { sys_access_raw(path.as_ptr(), mode) }
}

// readlink syscall
unsafe fn sys_readlink_raw(pathname: *const u8, buf: *mut u8, bufsiz: usize) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 89,
            in("rdi") pathname,
            in("rsi") buf,
            in("rdx") bufsiz,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
        );
    }
    ret
}

pub fn readlink(path: &[u8], buf: &mut [u8]) -> isize {
    unsafe { sys_readlink_raw(path.as_ptr(), buf.as_mut_ptr(), buf.len()) }
}

// Socket constants
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

// socket syscall
unsafe fn sys_socket_raw(domain: i32, socket_type: i32, protocol: i32) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 41,
            in("rdi") domain,
            in("rsi") socket_type,
            in("rdx") protocol,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
        );
    }
    ret
}

pub fn socket(domain: i32, socket_type: i32, protocol: i32) -> isize {
    unsafe { sys_socket_raw(domain, socket_type, protocol) }
}

// bind syscall
unsafe fn sys_bind_raw(sockfd: i32, addr: *const SockaddrIn, addrlen: u32) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 49,
            in("rdi") sockfd,
            in("rsi") addr,
            in("rdx") addrlen,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
        );
    }
    ret
}

pub fn bind(sockfd: i32, addr: &SockaddrIn) -> isize {
    unsafe { sys_bind_raw(sockfd, addr as *const SockaddrIn, 16) }
}

// listen syscall
unsafe fn sys_listen_raw(sockfd: i32, backlog: i32) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 50,
            in("rdi") sockfd,
            in("rsi") backlog,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
        );
    }
    ret
}

pub fn listen(sockfd: i32, backlog: i32) -> isize {
    unsafe { sys_listen_raw(sockfd, backlog) }
}

// accept syscall
unsafe fn sys_accept_raw(sockfd: i32, addr: *mut u8, addrlen: *mut u32) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 43,
            in("rdi") sockfd,
            in("rsi") addr,
            in("rdx") addrlen,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
        );
    }
    ret
}

pub fn accept(sockfd: i32) -> isize {
    unsafe { sys_accept_raw(sockfd, core::ptr::null_mut(), core::ptr::null_mut()) }
}

// setsockopt syscall
unsafe fn sys_setsockopt_raw(sockfd: i32, level: i32, optname: i32, optval: *const i32, optlen: u32) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 54,
            in("rdi") sockfd,
            in("rsi") level,
            in("rdx") optname,
            in("r10") optval,
            in("r8") optlen,
            lateout("rax") ret,
            lateout("rcx") _,
            lateout("r11") _,
        );
    }
    ret
}

pub fn setsockopt(sockfd: i32, level: i32, optname: i32, optval: i32) -> isize {
    unsafe { sys_setsockopt_raw(sockfd, level, optname, &optval as *const i32, 4) }
}
