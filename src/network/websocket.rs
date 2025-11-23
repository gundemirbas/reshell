use crate::syscalls::{read, write, close, STDOUT};
use crate::io::print;
use crate::system::crypto::{sha1, base64_encode};
use core::sync::atomic::{AtomicUsize, Ordering};

// Shared message buffer for broadcasting
const MAX_MESSAGES: usize = 100;
const MAX_MSG_LEN: usize = 512;

// Shell command queue
const MAX_SHELL_CMDS: usize = 10;
static SHELL_CMD_BUFFER: [AtomicUsize; MAX_SHELL_CMDS * (MAX_MSG_LEN + 1)] = [const { AtomicUsize::new(0) }; MAX_SHELL_CMDS * (MAX_MSG_LEN + 1)];
static SHELL_CMD_WRITE_IDX: AtomicUsize = AtomicUsize::new(0);
static SHELL_CMD_READ_IDX: AtomicUsize = AtomicUsize::new(0);

// Input buffer for building commands (flat array)
const MAX_INPUT_LEN: usize = 512;
static INPUT_BUFFERS: [AtomicUsize; 16 * 512] = [const { AtomicUsize::new(0) }; 16 * 512];
static INPUT_BUFFER_LENS: [AtomicUsize; 16] = [const { AtomicUsize::new(0) }; 16];

fn get_input_base(client_idx: usize) -> usize {
    client_idx * MAX_INPUT_LEN
}

fn append_to_input(client_idx: usize, ch: u8) {
    let base = get_input_base(client_idx);
    let len = INPUT_BUFFER_LENS[client_idx].load(Ordering::Acquire);
    if len < MAX_INPUT_LEN {
        INPUT_BUFFERS[base + len].store(ch as usize, Ordering::Release);
        INPUT_BUFFER_LENS[client_idx].store(len + 1, Ordering::Release);
    }
}

fn get_input_buffer(client_idx: usize, out: &mut [u8]) -> usize {
    let base = get_input_base(client_idx);
    let len = INPUT_BUFFER_LENS[client_idx].load(Ordering::Acquire).min(out.len());
    for i in 0..len {
        out[i] = INPUT_BUFFERS[base + i].load(Ordering::Acquire) as u8;
    }
    len
}

fn clear_input_buffer(client_idx: usize) {
    INPUT_BUFFER_LENS[client_idx].store(0, Ordering::Release);
}

fn backspace_input(client_idx: usize) {
    let len = INPUT_BUFFER_LENS[client_idx].load(Ordering::Acquire);
    if len > 0 {
        INPUT_BUFFER_LENS[client_idx].store(len - 1, Ordering::Release);
    }
}

pub fn queue_shell_command(cmd: &[u8]) {
    let write_idx = SHELL_CMD_WRITE_IDX.load(Ordering::Acquire);
    let read_idx = SHELL_CMD_READ_IDX.load(Ordering::Acquire);
    
    // Check if queue is full
    if write_idx - read_idx >= MAX_SHELL_CMDS {
        print(b"[Shell] Command queue full!\n");
        return;
    }
    
    let slot = write_idx % MAX_SHELL_CMDS;
    let base = slot * (MAX_MSG_LEN + 1);
    
    let len = cmd.len().min(MAX_MSG_LEN);
    SHELL_CMD_BUFFER[base].store(len, Ordering::Release);
    
    for i in 0..len {
        SHELL_CMD_BUFFER[base + 1 + i].store(cmd[i] as usize, Ordering::Release);
    }
    
    SHELL_CMD_WRITE_IDX.fetch_add(1, Ordering::Release);
}

pub fn get_shell_command(out_buf: &mut [u8]) -> usize {
    let read_idx = SHELL_CMD_READ_IDX.load(Ordering::Acquire);
    let write_idx = SHELL_CMD_WRITE_IDX.load(Ordering::Acquire);
    
    if read_idx >= write_idx {
        return 0;
    }
    
    let slot = read_idx % MAX_SHELL_CMDS;
    let base = slot * (MAX_MSG_LEN + 1);
    
    let len = SHELL_CMD_BUFFER[base].load(Ordering::Acquire);
    if len == 0 || len > MAX_MSG_LEN {
        SHELL_CMD_READ_IDX.fetch_add(1, Ordering::Release);
        return 0;
    }
    
    let copy_len = len.min(out_buf.len());
    for i in 0..copy_len {
        out_buf[i] = SHELL_CMD_BUFFER[base + 1 + i].load(Ordering::Acquire) as u8;
    }
    
    SHELL_CMD_READ_IDX.fetch_add(1, Ordering::Release);
    copy_len
}

static MESSAGE_BUFFER: [AtomicUsize; MAX_MESSAGES * (MAX_MSG_LEN + 1)] = [const { AtomicUsize::new(0) }; MAX_MESSAGES * (MAX_MSG_LEN + 1)];
static MESSAGE_COUNT: AtomicUsize = AtomicUsize::new(0);
static MESSAGE_READ_INDEX: [AtomicUsize; 16] = [const { AtomicUsize::new(0) }; 16]; // Per-client read position

pub fn broadcast_message(msg: &[u8]) {
    let count = MESSAGE_COUNT.load(Ordering::Acquire);
    let slot = count % MAX_MESSAGES;
    let base = slot * (MAX_MSG_LEN + 1);
    
    // Store length
    MESSAGE_BUFFER[base].store(msg.len().min(MAX_MSG_LEN), Ordering::Release);
    
    // Store message data
    for i in 0..msg.len().min(MAX_MSG_LEN) {
        MESSAGE_BUFFER[base + 1 + i].store(msg[i] as usize, Ordering::Release);
    }
    
    MESSAGE_COUNT.fetch_add(1, Ordering::Release);
}

fn get_next_message(client_idx: usize, out_buf: &mut [u8]) -> usize {
    let read_idx = MESSAGE_READ_INDEX[client_idx].load(Ordering::Acquire);
    let write_idx = MESSAGE_COUNT.load(Ordering::Acquire);
    
    if read_idx >= write_idx {
        return 0; // No new messages
    }
    
    let slot = read_idx % MAX_MESSAGES;
    let base = slot * (MAX_MSG_LEN + 1);
    
    let len = MESSAGE_BUFFER[base].load(Ordering::Acquire);
    if len == 0 || len > MAX_MSG_LEN {
        MESSAGE_READ_INDEX[client_idx].fetch_add(1, Ordering::Release);
        return 0;
    }
    
    let copy_len = len.min(out_buf.len());
    for i in 0..copy_len {
        out_buf[i] = MESSAGE_BUFFER[base + 1 + i].load(Ordering::Acquire) as u8;
    }
    
    MESSAGE_READ_INDEX[client_idx].fetch_add(1, Ordering::Release);
    
    use crate::io::{print, print_number};
    print(b"[WS Client ");
    print_number(client_idx as i64);
    print(b"] Got message len=");
    print_number(copy_len as i64);
    print(b"\n");
    
    copy_len
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
    // Initialize read position to current message count
    MESSAGE_READ_INDEX[client_idx].store(MESSAGE_COUNT.load(Ordering::Acquire), Ordering::Release);
    
    use crate::io::{print_number};
    print(b"[WS] Client ");
    print_number(client_idx as i64);
    print(b" starting at message index ");
    print_number(MESSAGE_COUNT.load(Ordering::Acquire) as i64);
    print(b"\n");
    
    websocket_frame_loop(client_fd, client_idx);
}

// WebSocket frame loop
fn websocket_frame_loop(client_fd: i32, client_idx: usize) {
    use crate::syscalls::{poll, PollFd, POLLIN};
    
    let mut buf = [0u8; 1024];
    let mut msg_buf = [0u8; MAX_MSG_LEN];
    
    loop {
        use crate::syscalls::should_shutdown;
        if should_shutdown() {
            print(b"[WS] Shutdown requested, closing\n");
            break;
        }
        
        // Check for new broadcast messages to send
        loop {
            let msg_len = get_next_message(client_idx, &mut msg_buf);
            if msg_len == 0 {
                break;
            }
            send_websocket_text(client_fd, &msg_buf[..msg_len]);
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
                        // Enter pressed - execute command
                        let mut cmd = [0u8; 512];
                        let buf_len = get_input_buffer(client_idx, &mut cmd);
                        
                        if buf_len > 0 {
                            // Echo the command to all clients
                            let mut echo_msg = [0u8; 520];
                            let mut echo_len = 0;
                            let prefix = b"> ";
                            for &b in prefix {
                                echo_msg[echo_len] = b;
                                echo_len += 1;
                            }
                            for j in 0..buf_len {
                                echo_msg[echo_len] = cmd[j];
                                echo_len += 1;
                            }
                            echo_msg[echo_len] = b'\n';
                            echo_len += 1;
                            broadcast_message(&echo_msg[..echo_len]);
                            
                            // Queue for execution
                            queue_shell_command(&cmd[..buf_len]);
                            
                            // Clear buffer
                            clear_input_buffer(client_idx);
                        }
                    } else if ch == 0x7f || ch == 0x08 {
                        // Backspace/Delete
                        backspace_input(client_idx);
                    } else if ch >= 32 && ch < 127 {
                        // Printable character
                        append_to_input(client_idx, ch);
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
