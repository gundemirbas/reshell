pub fn bytes_equal(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    for i in 0..a.len() {
        if a[i] != b[i] {
            return false;
        }
    }
    true
}

pub fn trim_newline(buf: &[u8]) -> &[u8] {
    let mut end = buf.len();
    while end > 0 && (buf[end - 1] == b'\n' || buf[end - 1] == b'\r') {
        end -= 1;
    }
    &buf[..end]
}

pub fn trim_spaces(s: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = s.len();
    
    while start < s.len() && s[start] == b' ' {
        start += 1;
    }
    
    while end > start && (s[end - 1] == b' ' || s[end - 1] == 0) {
        end -= 1;
    }
    
    &s[start..end]
}

pub fn split_first_word(input: &[u8]) -> (&[u8], &[u8]) {
    for i in 0..input.len() {
        if input[i] == b' ' {
            return (&input[..i], &input[i + 1..]);
        }
    }
    (input, &[])
}

fn bytes_less_than(a: &[u8], b: &[u8]) -> bool {
    let min_len = if a.len() < b.len() { a.len() } else { b.len() };
    
    for i in 0..min_len {
        if a[i] < b[i] {
            return true;
        } else if a[i] > b[i] {
            return false;
        }
    }
    
    a.len() < b.len()
}

pub fn sort_entries(entries: &mut [[u8; 256]], count: usize) {
    if count <= 1 {
        return;
    }
    
    for i in 0..count {
        for j in 0..(count - i - 1) {
            let mut len_a = 0;
            while len_a < 256 && entries[j][len_a] != 0 {
                len_a += 1;
            }
            
            let mut len_b = 0;
            while len_b < 256 && entries[j + 1][len_b] != 0 {
                len_b += 1;
            }
            
            if len_a > 0 && len_b > 0 {
                let a = &entries[j][..len_a];
                let b = &entries[j + 1][..len_b];
                
                if !bytes_less_than(a, b) {
                    let temp = entries[j];
                    entries[j] = entries[j + 1];
                    entries[j + 1] = temp;
                }
            }
        }
    }
}
