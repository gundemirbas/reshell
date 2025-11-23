use crate::syscalls::{read, write, close, STDOUT};
use crate::io::print;
use crate::system::crypto::{sha1, base64_encode};
use crate::shell::{allocate_session, free_session, get_session, execute_command_in_session};

// Legacy broadcast/queue functions for backwards compatibility with main.rs
// These are no longer used in per-session websocket implementation
pub fn broadcast_message(_msg: &[u8]) {
    // No-op: each session now has its own I/O
}

pub fn get_shell_command(_out_buf: &mut [u8]) -> usize {
    // No-op: commands are executed directly in sessions
    0
}

pub fn is_websocket_upgrade(request: &[u8]) -> bool {
    for i in 0..request.len().saturating_sub(9) {
        if &request[i..i+9] == b"websocket" || 
           &request[i..i+9] == b"WebSocket" {
            return true;
        }
    }
    false
}

// Handle WebSocket connection
pub fn handle_websocket_connection(client_fd: i32, request: &[u8]) {
    print(b"[WS] Parsing request...\n");
    
    let mut key = [0u8; 64];
    let mut key_len = 0;
    
    print(b"[WS] Looking for key header...\n");
    
    for i in 0..request.len().saturating_sub(19) {
        if i + 19 <= request.len() {
            let mut match_found = true;
            let pattern = b"Sec-WebSocket-Key: ";
            for k in 0..19 {
                if request[i + k] != pattern[k] {
                    match_found = false;
                    break;
                }
            }
            
            if match_found {
                print(b"[WS] Key header found at position ");
                print(b"\n");
                
                let mut j = i + 19;
                while j < request.len() && key_len < 64 {
                    if request[j] == b'\r' || request[j] == b'\n' {
                        break;
                    }
                    key[key_len] = request[j];
                    key_len += 1;
                    j += 1;
                }
                break;
            }
        }
    }
    
    if key_len == 0 {
        print(b"[WS] ERROR: No key found!\n");
        close(client_fd);
        return;
    }
    
    print(b"[WS] Key length: ");
    write(STDOUT, &key[..key_len.min(30)]);
    print(b"\n");
    
    print(b"[WS] Computing accept key...\n");
    
    let magic = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let mut combined = [0u8; 128];
    let mut combined_len = 0;
    
    for i in 0..key_len.min(60) {
        if combined_len < 128 {
            combined[combined_len] = key[i];
            combined_len += 1;
        }
    }
    
    for i in 0..magic.len() {
        if combined_len < 128 {
            combined[combined_len] = magic[i];
            combined_len += 1;
        }
    }
    
    print(b"[WS] Computing SHA1...\n");
    let hash = sha1(&combined[..combined_len]);
    
    print(b"[WS] Encoding base64...\n");
    let mut accept_key = [0u8; 32];
    let accept_len = base64_encode(&hash, &mut accept_key);
    
    print(b"[WS] Accept key: ");
    write(STDOUT, &accept_key[..accept_len.min(28)]);
    print(b"\n");
    
    // Build response
    let mut response = [0u8; 512];
    let mut pos = 0;
    
    let header = b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ";
    for &b in header {
        response[pos] = b;
        pos += 1;
    }
    
    for i in 0..accept_len {
        response[pos] = accept_key[i];
        pos += 1;
    }
    
    let trailer = b"\r\n\r\n";
    for &b in trailer {
        response[pos] = b;
        pos += 1;
    }
    
    print(b"[WS] Sending upgrade response...\n");
    let write_result = write(client_fd, &response[..pos]);
    if write_result < 0 {
        print(b"[WS] Failed to send upgrade response\n");
        close(client_fd);
        return;
    }
    
    print(b"[WS] Response sent, starting thread for frames...\n");
    
    // Don't close client_fd here - thread will handle it
}

// WebSocket frame loop (public version for thread - called from thread with known index)
pub fn websocket_frame_loop_with_index(client_fd: i32, client_idx: usize) {
    use crate::io::print_number;
    
    // Allocate a shell session for this connection
    let session_id = match allocate_session() {
        Some(id) => id,
        None => {
            print(b"[WS] Failed to allocate session\n");
            close(client_fd);
            return;
        }
    };
    
    print(b"[WS] Client ");
    print_number(client_idx as i64);
    print(b" allocated session ");
    print_number(session_id as i64);
    print(b"\n");
    
    websocket_frame_loop(client_fd, session_id);
    
    // Free session when connection closes
    free_session(session_id);
    print(b"[WS] Session ");
    print_number(session_id as i64);
    print(b" freed\n");
}

// WebSocket frame loop
fn websocket_frame_loop(client_fd: i32, session_id: usize) {
    use crate::syscalls::{poll, PollFd, POLLIN};
    
    let session = match get_session(session_id) {
        Some(s) => s,
        None => {
            print(b"[WS] Invalid session\n");
            return;
        }
    };
    
    let mut buf = [0u8; 1024];
    let mut output_buf = [0u8; 4096];
    
    // Send welcome message
    send_websocket_text(client_fd, b"Welcome to ReShell!\n$ ");
    
    loop {
        use crate::syscalls::should_shutdown;
        if should_shutdown() {
            print(b"[WS] Shutdown requested, closing\n");
            break;
        }
        
        // Check for output from shell session
        if session.has_output() {
            let len = session.read_output(&mut output_buf);
            if len > 0 {
                send_websocket_text(client_fd, &output_buf[..len]);
                // Send prompt after output
                send_websocket_text(client_fd, b"$ ");
            }
        }
        
        // Poll with 50ms timeout to check for incoming data
        let mut poll_fds = [PollFd {
            fd: client_fd,
            events: POLLIN,
            revents: 0,
        }];
        
        let poll_result = poll(&mut poll_fds, 50); // 50ms timeout
        
        if poll_result < 0 {
            print(b"[WS] Poll error\n");
            break;
        }
        
        if poll_result == 0 {
            // Timeout - no data, continue to check broadcasts
            continue;
        }
        
        // Data available, read it
        let n = read(client_fd, &mut buf);
        
        if n < 0 {
            print(b"[WS] Read error\n");
            break;
        }
        
        if n == 0 {
            print(b"[WS] Client disconnected (EOF)\n");
            break;
        }
        
        let n = n as usize;
        
        if n < 2 {
            continue;
        }
        
        let _fin = (buf[0] & 0x80) != 0;
        let opcode = buf[0] & 0x0F;
        let masked = (buf[1] & 0x80) != 0;
        let mut payload_len = (buf[1] & 0x7F) as usize;
        
        let mut offset = 2usize;
        
        if payload_len == 126 {
            if n < 4 { continue; }
            payload_len = ((buf[2] as usize) << 8) | (buf[3] as usize);
            offset = 4;
        } else if payload_len == 127 {
            continue;
        }
        
        let mut mask = [0u8; 4];
        if masked {
            if n < offset + 4 { continue; }
            mask.copy_from_slice(&buf[offset..offset+4]);
            offset += 4;
        }
        
        if n < offset + payload_len {
            continue;
        }
        
        match opcode {
            0x1 => { // Text frame
                let mut payload = [0u8; 512];
                let payload_end = payload_len.min(512);
                for i in 0..payload_end {
                    payload[i] = buf[offset + i] ^ mask[i % 4];
                }
                
                // Process each character
                for i in 0..payload_end {
                    let ch = payload[i];
                    
                    if ch == b'\n' || ch == b'\r' {
                        // Enter pressed - execute command in session
                        let mut cmd = [0u8; 512];
                        let cmd_len = session.get_input(&mut cmd);
                        
                        if cmd_len > 0 {
                            // Echo newline
                            send_websocket_text(client_fd, b"\n");
                            
                            // Execute command in this session
                            execute_command_in_session(session, &cmd[..cmd_len]);
                            
                            // Clear input
                            session.clear_input();
                        } else {
                            // Empty command, just send prompt
                            send_websocket_text(client_fd, b"\n$ ");
                        }
                    } else if ch == 0x7f || ch == 0x08 {
                        // Backspace/Delete
                        if session.input_len() > 0 {
                            session.backspace_input();
                            // Send backspace sequence to client
                            send_websocket_text(client_fd, b"\x08 \x08");
                        }
                    } else if ch >= 32 && ch < 127 {
                        // Printable character
                        session.append_input(ch);
                        // Echo character back to client
                        send_websocket_text(client_fd, &[ch]);
                    }
                }
            }
            0x8 => { // Close frame
                print(b"[WS] Close frame received\n");
                let close_frame = [0x88, 0x00];
                write(client_fd, &close_frame);
                break;
            }
            0x9 => { // Ping frame
                let pong = [0x8A, 0x00];
                write(client_fd, &pong);
            }
            _ => {}
        }
    }
}

// Send text frame
fn send_websocket_text(fd: i32, payload: &[u8]) {
    let mut frame = [0u8; 1024];
    let mut pos = 0;
    
    frame[pos] = 0x81;
    pos += 1;
    
    let len = payload.len();
    if len < 126 {
        frame[pos] = len as u8;
        pos += 1;
    } else if len < 65536 {
        frame[pos] = 126;
        pos += 1;
        frame[pos] = (len >> 8) as u8;
        pos += 1;
        frame[pos] = (len & 0xFF) as u8;
        pos += 1;
    } else {
        return;
    }
    
    for i in 0..len {
        if pos >= frame.len() { break; }
        frame[pos] = payload[i];
        pos += 1;
    }
    
    write(fd, &frame[..pos]);
}
