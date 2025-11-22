use crate::syscalls::*;
use crate::io::print;
use crate::parser::DirentParser;
use crate::utils::sort_entries;

// File permission: 0644 (rw-r--r--)
const FILE_MODE: i32 = 0o644;

fn htons(port: u16) -> u16 {
    ((port & 0xff) << 8) | ((port >> 8) & 0xff)
}

fn send_response(client_fd: i32, status: &[u8], content_type: &[u8], body: &[u8]) {
    let mut response = [0u8; 8192];
    let mut pos = 0;
    
    // HTTP status line
    let status_line = b"HTTP/1.1 ";
    for &b in status_line {
        response[pos] = b;
        pos += 1;
    }
    for &b in status {
        response[pos] = b;
        pos += 1;
    }
    response[pos] = b'\r';
    pos += 1;
    response[pos] = b'\n';
    pos += 1;
    
    // Content-Type header
    let ct_header = b"Content-Type: ";
    for &b in ct_header {
        response[pos] = b;
        pos += 1;
    }
    for &b in content_type {
        response[pos] = b;
        pos += 1;
    }
    response[pos] = b'\r';
    pos += 1;
    response[pos] = b'\n';
    pos += 1;
    
    // Content-Length header
    let cl_header = b"Content-Length: ";
    for &b in cl_header {
        response[pos] = b;
        pos += 1;
    }
    
    // Convert body length to string
    let body_len = body.len();
    let mut len_str = [0u8; 20];
    let mut len_digits = 0;
    let mut temp = body_len;
    if temp == 0 {
        len_str[0] = b'0';
        len_digits = 1;
    } else {
        while temp > 0 {
            len_str[len_digits] = b'0' + (temp % 10) as u8;
            temp /= 10;
            len_digits += 1;
        }
    }
    
    for i in (0..len_digits).rev() {
        response[pos] = len_str[i];
        pos += 1;
    }
    response[pos] = b'\r';
    pos += 1;
    response[pos] = b'\n';
    pos += 1;
    
    // Empty line
    response[pos] = b'\r';
    pos += 1;
    response[pos] = b'\n';
    pos += 1;
    
    // Body
    for i in 0..body_len {
        if pos >= response.len() {
            break;
        }
        response[pos] = body[i];
        pos += 1;
    }
    
    write(client_fd, &response[..pos]);
}

fn parse_http_request(request: &[u8]) -> (&[u8], &[u8], &[u8]) {
    // Parse first line: METHOD PATH HTTP/1.1
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
    
    // Find body (after \r\n\r\n)
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

fn generate_directory_listing(path: &[u8]) -> ([u8; 4096], usize) {
    let mut html = [0u8; 4096];
    let mut pos = 0;
    
    // HTML header with CSS and upload form
    let header = b"<!DOCTYPE html>
<html>
<head>
    <meta charset='utf-8'>
    <title>Directory Listing</title>
    <style>
        body { font-family: monospace; margin: 20px; background: #1e1e1e; color: #d4d4d4; }
        h1 { color: #4ec9b0; }
        ul { list-style: none; padding: 0; }
        li { padding: 5px; }
        a { color: #569cd6; text-decoration: none; }
        a:hover { text-decoration: underline; }
        .upload { margin: 20px 0; padding: 15px; background: #2d2d2d; border-radius: 5px; }
        input, button { padding: 8px; margin: 5px; background: #3e3e3e; color: #d4d4d4; border: 1px solid #555; border-radius: 3px; }
        button { cursor: pointer; }
        button:hover { background: #4e4e4e; }
        .info { color: #858585; font-size: 0.9em; margin-top: 20px; }
    </style>
</head>
<body>
    <h1>[DIR] Directory Listing</h1>
    <div class='upload'>
        <h3>[UPLOAD] Upload File</h3>
        <form method='POST' action='/upload'>
            <input type='text' name='filename' placeholder='Filename' required style='width: 300px;'>
            <br>
            <textarea name='content' placeholder='File content' rows='8' cols='60'></textarea>
            <br>
            <button type='submit'>Upload</button>
        </form>
    </div>
    <h3>[FILES] Files:</h3>
    <ul>";
    
    for &b in header {
        if pos >= html.len() { break; }
        html[pos] = b;
        pos += 1;
    }
    
    // Open directory
    let fd = open(path, O_RDONLY | O_DIRECTORY);
    if fd < 0 {
        let error_msg = b"<li>Error opening directory</li></ul></body></html>";
        for &b in error_msg {
            if pos >= html.len() { break; }
            html[pos] = b;
            pos += 1;
        }
        return (html, pos);
    }
    
    // Collect entries
    let mut entries = [[0u8; 256]; 128];
    let mut count = 0;
    let mut buf = [0u8; 2048];
    
    loop {
        let nread = getdents64(fd as i32, &mut buf);
        if nread <= 0 {
            break;
        }
        
        let mut parser = DirentParser::new(&buf[..nread as usize]);
        while let Some(entry) = parser.next() {
            if count >= 128 {
                break;
            }
            
            let name = entry.name;
            if name.len() > 0 && name.len() < 256 {
                for i in 0..name.len() {
                    entries[count][i] = name[i];
                }
                entries[count][name.len()] = 0;
                count += 1;
            }
        }
        
        if count >= 128 {
            break;
        }
    }
    
    close(fd as i32);
    
    // Sort entries
    sort_entries(&mut entries, count);
    
    // Generate HTML list
    for i in 0..count {
        let mut len = 0;
        while len < 256 && entries[i][len] != 0 {
            len += 1;
        }
        
        if len > 0 {
            let li_start = b"<li><a href=\"";
            for &b in li_start {
                if pos >= html.len() { break; }
                html[pos] = b;
                pos += 1;
            }
            
            for j in 0..len {
                if pos >= html.len() { break; }
                html[pos] = entries[i][j];
                pos += 1;
            }
            
            let li_mid = b"\">";
            for &b in li_mid {
                if pos >= html.len() { break; }
                html[pos] = b;
                pos += 1;
            }
            
            for j in 0..len {
                if pos >= html.len() { break; }
                html[pos] = entries[i][j];
                pos += 1;
            }
            
            let li_end = b"</a></li>";
            for &b in li_end {
                if pos >= html.len() { break; }
                html[pos] = b;
                pos += 1;
            }
        }
    }
    
    // HTML footer
    let footer = b"</ul>
    <div class='info'>
        <p>Powered by nostd Rust Shell v0.4.0</p>
        <p>Tip: Use curl to upload: <code>curl -X POST http://localhost:PORT/upload -d 'filename=test.txt&amp;content=Hello'</code></p>
    </div>
</body>
</html>";
    
    for &b in footer {
        if pos >= html.len() { break; }
        html[pos] = b;
        pos += 1;
    }
    
    (html, pos)
}

fn handle_post_upload(body: &[u8], filename_out: &mut [u8], content_out: &mut [u8]) -> (usize, usize, bool) {
    // Parse POST body: filename=xxx&content=yyy
    let mut filename_start = 0;
    let mut filename_end = 0;
    let mut content_start = 0;
    let mut content_end = body.len();
    
    // Find "filename="
    for i in 0..body.len().saturating_sub(9) {
        if body[i..i+9] == *b"filename=" {
            filename_start = i + 9;
            break;
        }
    }
    
    if filename_start == 0 {
        return (0, 0, false);
    }
    
    // Find end of filename (& or end)
    filename_end = filename_start;
    while filename_end < body.len() && body[filename_end] != b'&' {
        filename_end += 1;
    }
    
    // Find "content="
    for i in filename_end..body.len().saturating_sub(8) {
        if body[i..i+8] == *b"content=" {
            content_start = i + 8;
            break;
        }
    }
    
    if content_start == 0 || filename_start >= filename_end {
        return (0, 0, false);
    }
    
    let filename = &body[filename_start..filename_end];
    let content = &body[content_start..content_end];
    
    // URL decode
    let name_len = url_decode(filename, filename_out);
    let content_len = url_decode(content, content_out);
    
    // Create file
    let mut path = [0u8; 256];
    let mut pos = 0;
    for i in 0..name_len {
        path[pos] = filename_out[i];
        pos += 1;
    }
    path[pos] = 0;
    
    let fd = open_with_mode(&path[..pos+1], O_WRONLY | O_CREAT | O_TRUNC, FILE_MODE);
    if fd < 0 {
        return (name_len, content_len, false);
    }
    
    write(fd as i32, &content_out[..content_len]);
    close(fd as i32);
    
    (name_len, content_len, true)
}

fn url_decode(input: &[u8], output: &mut [u8]) -> usize {
    let mut out_pos = 0;
    let mut i = 0;
    
    while i < input.len() && out_pos < output.len() {
        if input[i] == b'+' {
            output[out_pos] = b' ';
            out_pos += 1;
            i += 1;
        } else if input[i] == b'%' && i + 2 < input.len() {
            // Hex decode
            let h1 = hex_to_byte(input[i+1]);
            let h2 = hex_to_byte(input[i+2]);
            if h1 < 16 && h2 < 16 {
                output[out_pos] = (h1 << 4) | h2;
                out_pos += 1;
                i += 3;
            } else {
                output[out_pos] = input[i];
                out_pos += 1;
                i += 1;
            }
        } else {
            output[out_pos] = input[i];
            out_pos += 1;
            i += 1;
        }
    }
    
    out_pos
}

fn hex_to_byte(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => 255,
    }
}

fn serve_file(path: &[u8]) -> ([u8; 65536], usize, &'static [u8]) {
    let mut content = [0u8; 65536];
    
    let fd = open(path, O_RDONLY);
    if fd < 0 {
        return (content, 0, b"text/plain");
    }
    
    let n = read(fd as i32, &mut content);
    close(fd as i32);
    
    if n <= 0 {
        return (content, 0, b"text/plain");
    }
    
    // Detect content type based on extension
    let content_type = detect_content_type(path);
    
    (content, n as usize, content_type)
}

fn detect_content_type(path: &[u8]) -> &'static [u8] {
    // Find last dot
    let mut dot_pos = None;
    for i in (0..path.len()).rev() {
        if path[i] == b'.' {
            dot_pos = Some(i);
            break;
        }
        if path[i] == b'/' || path[i] == 0 {
            break;
        }
    }
    
    if let Some(pos) = dot_pos {
        let ext = &path[pos+1..];
        
        // Check common extensions
        if ext.starts_with(b"html") || ext.starts_with(b"htm") {
            return b"text/html; charset=utf-8";
        }
        if ext.starts_with(b"txt") {
            return b"text/plain; charset=utf-8";
        }
        if ext.starts_with(b"js") {
            return b"application/javascript";
        }
        if ext.starts_with(b"json") {
            return b"application/json";
        }
        if ext.starts_with(b"css") {
            return b"text/css";
        }
        if ext.starts_with(b"png") {
            return b"image/png";
        }
        if ext.starts_with(b"jpg") || ext.starts_with(b"jpeg") {
            return b"image/jpeg";
        }
        if ext.starts_with(b"gif") {
            return b"image/gif";
        }
        if ext.starts_with(b"svg") {
            return b"image/svg+xml";
        }
    }
    
    b"application/octet-stream"
}

fn generate_upload_success_page(filename: &[u8], content: &[u8]) -> ([u8; 8192], usize) {
    let mut html = [0u8; 8192];
    let mut pos = 0;
    
    let header = b"<!DOCTYPE html>
<html>
<head>
    <meta charset='utf-8'>
    <title>Upload Success</title>
    <style>
        body { font-family: monospace; background: #1e1e1e; color: #4ec9b0; padding: 20px; }
        .container { max-width: 800px; margin: 0 auto; }
        h1 { color: #4ec9b0; }
        .file-info { background: #2d2d2d; padding: 15px; border-radius: 5px; margin: 20px 0; }
        .filename { color: #ce9178; font-weight: bold; }
        .content { background: #1e1e1e; padding: 10px; border-left: 3px solid #4ec9b0; margin: 10px 0; white-space: pre-wrap; color: #d4d4d4; }
        a { color: #569cd6; text-decoration: none; }
        a:hover { text-decoration: underline; }
    </style>
</head>
<body>
    <div class='container'>
        <h1>[OK] Upload Successful!</h1>
        <div class='file-info'>
            <p>File: <span class='filename'>";
    
    for &b in header {
        if pos >= html.len() { break; }
        html[pos] = b;
        pos += 1;
    }
    
    // Filename
    for &b in filename {
        if pos >= html.len() { break; }
        html[pos] = b;
        pos += 1;
    }
    
    let mid = b"</span></p>
            <p>Size: ";
    for &b in mid {
        if pos >= html.len() { break; }
        html[pos] = b;
        pos += 1;
    }
    
    // Content length
    let content_len = content.len();
    let mut len_str = [0u8; 20];
    let mut len_digits = 0;
    let mut temp = content_len;
    if temp == 0 {
        len_str[0] = b'0';
        len_digits = 1;
    } else {
        while temp > 0 {
            len_str[len_digits] = b'0' + (temp % 10) as u8;
            temp /= 10;
            len_digits += 1;
        }
    }
    for i in (0..len_digits).rev() {
        if pos >= html.len() { break; }
        html[pos] = len_str[i];
        pos += 1;
    }
    
    let content_header = b" bytes</p>
            <p>Content:</p>
            <div class='content'>";
    for &b in content_header {
        if pos >= html.len() { break; }
        html[pos] = b;
        pos += 1;
    }
    
    // Content (HTML escaped)
    for &b in content {
        if pos >= html.len() - 10 { break; }
        match b {
            b'<' => {
                for &c in b"&lt;" {
                    html[pos] = c;
                    pos += 1;
                }
            }
            b'>' => {
                for &c in b"&gt;" {
                    html[pos] = c;
                    pos += 1;
                }
            }
            b'&' => {
                for &c in b"&amp;" {
                    html[pos] = c;
                    pos += 1;
                }
            }
            _ => {
                html[pos] = b;
                pos += 1;
            }
        }
    }
    
    let footer = b"</div>
        </div>
        <a href='/'>Back to listing</a>
    </div>
</body>
</html>";
    
    for &b in footer {
        if pos >= html.len() { break; }
        html[pos] = b;
        pos += 1;
    }
    
    (html, pos)
}

pub fn start_http_server(port: u16) {
    print(b"Starting HTTP server on port ");
    let mut port_str = [0u8; 10];
    let mut digits = 0;
    let mut temp = port as usize;
    while temp > 0 {
        port_str[digits] = b'0' + (temp % 10) as u8;
        temp /= 10;
        digits += 1;
    }
    for i in (0..digits).rev() {
        write(STDOUT, &[port_str[i]]);
    }
    print(b"...\n");
    
    // Create socket
    let sockfd = socket(AF_INET, SOCK_STREAM, 0);
    if sockfd < 0 {
        print(b"Error creating socket\n");
        return;
    }
    
    // Set SO_REUSEADDR
    let optval = 1;
    setsockopt(sockfd as i32, SOL_SOCKET, SO_REUSEADDR, optval);
    
    // Bind to port
    let addr = SockaddrIn {
        sin_family: AF_INET as u16,
        sin_port: htons(port),
        sin_addr: 0, // INADDR_ANY (0.0.0.0)
        sin_zero: [0u8; 8],
    };
    
    if bind(sockfd as i32, &addr) < 0 {
        print(b"Error binding socket\n");
        close(sockfd as i32);
        return;
    }
    
    // Listen
    if listen(sockfd as i32, 10) < 0 {
        print(b"Error listening on socket\n");
        close(sockfd as i32);
        return;
    }
    
    print(b"Server running! Press Ctrl+C to stop.\n");
    print(b"Visit http://localhost:");
    for i in (0..digits).rev() {
        write(STDOUT, &[port_str[i]]);
    }
    print(b"/\n\n");
    
    // Get current directory
    let mut cwd = [0u8; 512];
    cwd[0] = b'.';
    cwd[1] = 0;
    let cwd_len = getcwd(&mut cwd);
    let cwd_path = if cwd_len > 0 {
        &cwd[..cwd_len as usize]
    } else {
        b"."
    };
    
    // Accept loop
    loop {
        let client_fd = accept(sockfd as i32);
        if client_fd < 0 {
            continue;
        }
        
        // Read request
        let mut request = [0u8; 8192];
        let n = read(client_fd as i32, &mut request);
        
        if n > 0 {
            let (method, path, body) = parse_http_request(&request[..n as usize]);
            
            // Log request
            print(b"[");
            write(STDOUT, method);
            print(b" ");
            write(STDOUT, path);
            print(b"]\n");
            
            // Handle POST /upload
            if method == b"POST" && (path == b"/upload" || path == b"/") {
                let mut filename_buf = [0u8; 256];
                let mut content_buf = [0u8; 4096];
                let (name_len, content_len, success) = handle_post_upload(body, &mut filename_buf, &mut content_buf);
                
                if success {
                    let (success_html, html_len) = generate_upload_success_page(
                        &filename_buf[..name_len],
                        &content_buf[..content_len]
                    );
                    send_response(
                        client_fd as i32,
                        b"200 OK",
                        b"text/html; charset=utf-8",
                        &success_html[..html_len]
                    );
                } else {
                    let error_html = b"<!DOCTYPE html><html><head><meta charset='utf-8'><title>Upload Failed</title></head><body style='font-family:monospace;background:#1e1e1e;color:#f48771;padding:20px;'><h1>[ERROR] Upload Failed</h1><p>Could not create file.</p><a href='/' style='color:#569cd6;'>Back to listing</a></body></html>";
                    send_response(
                        client_fd as i32,
                        b"400 Bad Request",
                        b"text/html; charset=utf-8",
                        error_html
                    );
                }
            } else if method == b"GET" {
                // Check if requesting a specific file
                if path.len() > 1 && path[0] == b'/' {
                    // Serve file
                    let mut file_path = [0u8; 512];
                    let mut fp_len = 0;
                    
                    // Add current directory
                    if cwd_len > 0 {
                        for i in 0..cwd_len as usize {
                            file_path[fp_len] = cwd[i];
                            fp_len += 1;
                        }
                    } else {
                        file_path[0] = b'.';
                        fp_len = 1;
                    }
                    
                    // Add path from URL (skip leading /)
                    file_path[fp_len] = b'/';
                    fp_len += 1;
                    for i in 1..path.len() {
                        if fp_len >= 511 { break; }
                        file_path[fp_len] = path[i];
                        fp_len += 1;
                    }
                    file_path[fp_len] = 0;
                    
                    let (file_content, file_len, content_type) = serve_file(&file_path[..fp_len+1]);
                    
                    if file_len > 0 {
                        send_response(
                            client_fd as i32,
                            b"200 OK",
                            content_type,
                            &file_content[..file_len]
                        );
                    } else {
                        // File not found, show directory listing
                        let mut path_buf = [0u8; 512];
                        let mut path_len = 0;
                        
                        if cwd_len > 0 {
                            for i in 0..cwd_len as usize {
                                path_buf[path_len] = cwd[i];
                                path_len += 1;
                            }
                        } else {
                            path_buf[0] = b'.';
                            path_len = 1;
                        }
                        path_buf[path_len] = 0;
                        
                        let (html, html_len) = generate_directory_listing(&path_buf[..path_len + 1]);
                        
                        send_response(
                            client_fd as i32,
                            b"200 OK",
                            b"text/html; charset=utf-8",
                            &html[..html_len]
                        );
                    }
                } else {
                    // Root path - show directory listing
                    let mut path_buf = [0u8; 512];
                    let mut path_len = 0;
                    
                    if cwd_len > 0 {
                        for i in 0..cwd_len as usize {
                            path_buf[path_len] = cwd[i];
                            path_len += 1;
                        }
                    } else {
                        path_buf[0] = b'.';
                        path_len = 1;
                    }
                    path_buf[path_len] = 0;
                    
                    let (html, html_len) = generate_directory_listing(&path_buf[..path_len + 1]);
                    
                    send_response(
                        client_fd as i32,
                        b"200 OK",
                        b"text/html; charset=utf-8",
                        &html[..html_len]
                    );
                }
            } else {
                // Method not allowed
                let error_html = b"<!DOCTYPE html><html><head><title>Method Not Allowed</title></head><body style='font-family:monospace;background:#1e1e1e;color:#f48771;padding:20px;'><h1>405 Method Not Allowed</h1><p>Only GET and POST are supported.</p></body></html>";
                send_response(
                    client_fd as i32,
                    b"405 Method Not Allowed",
                    b"text/html; charset=utf-8",
                    error_html
                );
            }
        }
        
        close(client_fd as i32);
    }
}
