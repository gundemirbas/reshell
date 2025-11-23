pub fn find_in_path(cmd: &[u8], out_buf: &mut [u8]) -> bool {
    const PATHS: &[&[u8]] = &[
        b"/bin/",
        b"/usr/bin/",
        b"/usr/local/bin/",
    ];
    
    if cmd.len() > 0 && cmd[0] == b'/' {
        let mut idx = 0;
        for &b in cmd {
            if idx >= out_buf.len() - 1 {
                break;
            }
            out_buf[idx] = b;
            idx += 1;
        }
        out_buf[idx] = 0;
        return true;
    }
    
    for path in PATHS {
        let mut idx = 0;
        for &b in *path {
            if idx >= out_buf.len() - cmd.len() - 1 {
                break;
            }
            out_buf[idx] = b;
            idx += 1;
        }
        
        for &b in cmd {
            if idx >= out_buf.len() - 1 {
                break;
            }
            out_buf[idx] = b;
            idx += 1;
        }
        out_buf[idx] = 0;
        
        return true;
    }
    false
}
