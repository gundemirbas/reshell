use crate::syscalls::*;
use crate::io::print;

use super::server_utils;
use super::http_handler;

pub use http_handler::handle_http_request_inline;

use server_utils::htons;
use core::sync::atomic::{Ordering, AtomicI32};

static SERVER_SOCKET_FD: AtomicI32 = AtomicI32::new(-1);

pub fn close_server_socket() {
    let fd = SERVER_SOCKET_FD.load(Ordering::Acquire);
    if fd >= 0 {
        close(fd);
        SERVER_SOCKET_FD.store(-1, Ordering::Release);
    }
}

pub fn start_http_server(port: u16) {
    let sockfd = socket(AF_INET, SOCK_STREAM, 0);
    if sockfd < 0 {
        print(b"Error creating socket\n");
        return;
    }
    
    SERVER_SOCKET_FD.store(sockfd as i32, Ordering::Release);
    
    let optval = 1;
    setsockopt(sockfd as i32, SOL_SOCKET, SO_REUSEADDR, optval);
    
    let addr = SockaddrIn {
        sin_family: AF_INET as u16,
        sin_port: htons(port),
        sin_addr: 0,
        sin_zero: [0u8; 8],
    };
    
    if bind(sockfd as i32, &addr) < 0 {
        print(b"Error binding socket\n");
        close(sockfd as i32);
        return;
    }
    
    if listen(sockfd as i32, 10) < 0 {
        print(b"Error listening on socket\n");
        close(sockfd as i32);
        return;
    }
    
    loop {
        use crate::syscalls::should_shutdown;
        if should_shutdown() {
            print(b"[HTTP] Shutdown requested, closing server\n");
            close(sockfd as i32);
            return;
        }
        let client_fd = accept(sockfd as i32);
        if client_fd < 0 {
            if should_shutdown() {
                print(b"[HTTP] Accept failed during shutdown\n");
                return;
            }
            continue;
        }
        
        let mut request = [0u8; 4096];
        let n = read(client_fd as i32, &mut request);
        if n > 0 {
            use crate::network::websocket::{is_websocket_upgrade, handle_websocket_connection};
            
            if is_websocket_upgrade(&request[..n as usize]) {
                handle_websocket_connection(client_fd as i32, &request[..n as usize]);
                
                use crate::system::thread::start_websocket_thread;
                if !start_websocket_thread(client_fd as i32) {
                    print(b"[ERROR] Failed to start WebSocket thread, closing connection\n");
                    close(client_fd as i32);
                }
                // Don't close client_fd here - thread will handle it
                continue;
            } else {
                handle_http_request_inline(client_fd as i32, &request[..n as usize]);
            }
        }
        close(client_fd as i32);
    }
}
