use crate::syscalls::*;

pub fn parse_http_request(request: &[u8]) -> (&[u8], &[u8], &[u8]) {
    let mut method_end = 0;
    while method_end < request.len() && request[method_end] != b' ' {
        method_end += 1;
    }
    
    if method_end >= request.len() {
        return (b"GET", b"/", b"");
    }
    
    let method = &request[..method_end];
    
    let path_start = method_end + 1;
    let mut path_end = path_start;
    while path_end < request.len() && request[path_end] != b' ' && request[path_end] != b'?' {
        path_end += 1;
    }
    
    let path = if path_end > path_start {
        &request[path_start..path_end]
    } else {
        b"/"
    };
    
    let mut body_start = 0;
    for i in 0..request.len().saturating_sub(3) {
        if request[i] == b'\r' && request[i+1] == b'\n' && 
           request[i+2] == b'\r' && request[i+3] == b'\n' {
            body_start = i + 4;
            break;
        }
    }
    
    let body = if body_start > 0 && body_start < request.len() {
        &request[body_start..]
    } else {
        b""
    };
    
    (method, path, body)
}

pub fn handle_http_request_inline(client_fd: i32, request: &[u8]) {
    use crate::assets::{TERMINAL_HTML, TERMINAL_JS};
    
    let (method, path, _body) = parse_http_request(request);
    
    if method != b"GET" {
        let response = b"HTTP/1.1 405 Method Not Allowed\r\nContent-Type: text/html\r\nContent-Length: 23\r\n\r\n<h1>405 Not Allowed</h1>";
        write(client_fd, response);
        return;
    }
    
    if path == b"/" || path == b"/terminal.html" {
        send_simple_response(client_fd, b"200 OK", b"text/html; charset=utf-8", TERMINAL_HTML);
    } else if path == b"/terminal.js" {
        send_simple_response(client_fd, b"200 OK", b"application/javascript", TERMINAL_JS);
    } else {
        let response = b"HTTP/1.1 404 Not Found\r\nContent-Type: text/html\r\nContent-Length: 20\r\n\r\n<h1>404 Not Found</h1>";
        write(client_fd, response);
    }
}

fn send_simple_response(client_fd: i32, status: &[u8], content_type: &[u8], body: &[u8]) {
    let mut header = [0u8; 512];
    let mut pos = 0;
    
    for &b in b"HTTP/1.1 " { header[pos] = b; pos += 1; }
    for &b in status { if pos >= 512 { break; } header[pos] = b; pos += 1; }
    for &b in b"\r\nContent-Type: " { if pos >= 512 { break; } header[pos] = b; pos += 1; }
    for &b in content_type { if pos >= 512 { break; } header[pos] = b; pos += 1; }
    for &b in b"\r\nContent-Length: " { if pos >= 512 { break; } header[pos] = b; pos += 1; }
    
    let len = body.len();
    let mut len_str = [0u8; 20];
    let mut digits = 0;
    let mut temp = len;
    if temp == 0 { len_str[0] = b'0'; digits = 1; }
    else { while temp > 0 { len_str[digits] = b'0' + (temp % 10) as u8; temp /= 10; digits += 1; } }
    
    for i in (0..digits).rev() { if pos >= 512 { break; } header[pos] = len_str[i]; pos += 1; }
    for &b in b"\r\n\r\n" { if pos >= 512 { break; } header[pos] = b; pos += 1; }
    
    write(client_fd, &header[..pos]);
    write(client_fd, body);
}
