// WebSocket protocol implementation
use crate::syscalls::{read, write, close, STDOUT};
use crate::io::print;
use crate::crypto::{sha1, base64_encode};

// Check if request is WebSocket upgrade
pub fn is_websocket_upgrade(request: &[u8]) -> bool {
    // Simple check: look for "websocket" anywhere in request
    for i in 0..request.len().saturating_sub(9) {
        if &request[i..i+9] == b"websocket" || 
           &request[i..i+9] == b"WebSocket" {
            return true;
        }
    }
    false
}

// Handle WebSocket connection
pub fn handle_websocket(client_fd: i32, request: &[u8]) {
    print(b"[WS] Parsing request...\n");
    
    // Extract Sec-WebSocket-Key
    let mut key = [0u8; 64];
    let mut key_len = 0;
    
    print(b"[WS] Looking for key header...\n");
    
    for i in 0..request.len().saturating_sub(19) {
        if i + 19 <= request.len() {
            // Safe slice comparison
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
                // Print position (simplified)
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
    
    // Compute accept key
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
    
    print(b"[WS] Response sent, waiting for frames...\n");
    
    // WebSocket frame loop
    websocket_frame_loop(client_fd);
    
    print(b"[WS] Connection closed\n");
    close(client_fd);
}

// WebSocket frame loop
fn websocket_frame_loop(client_fd: i32) {
    let mut buf = [0u8; 1024];
    
    loop {
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
                
                write(STDOUT, b"[WS] Received: ");
                write(STDOUT, &payload[..payload_end]);
                write(STDOUT, b"\n");
                
                send_websocket_text(client_fd, &payload[..payload_end]);
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
