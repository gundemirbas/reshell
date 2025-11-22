// Simple SHA1 implementation for WebSocket
// Based on RFC 3174

pub fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h0: u32 = 0x67452301;
    let mut h1: u32 = 0xEFCDAB89;
    let mut h2: u32 = 0x98BADCFE;
    let mut h3: u32 = 0x10325476;
    let mut h4: u32 = 0xC3D2E1F0;
    
    let ml = data.len() as u64 * 8;
    
    let mut padded = [0u8; 128];
    let mut padded_len = 0;
    
    for &b in data {
        if padded_len < 128 {
            padded[padded_len] = b;
            padded_len += 1;
        }
    }
    
    if padded_len < 128 {
        padded[padded_len] = 0x80;
        padded_len += 1;
    }
    
    while (padded_len % 64) != 56 {
        if padded_len < 128 {
            padded[padded_len] = 0;
            padded_len += 1;
        } else {
            break;
        }
    }
    
    for i in 0..8 {
        if padded_len < 128 {
            padded[padded_len] = ((ml >> (56 - i * 8)) & 0xff) as u8;
            padded_len += 1;
        }
    }
    
    let chunks = padded_len / 64;
    for chunk_idx in 0..chunks {
        let chunk = &padded[chunk_idx * 64..(chunk_idx + 1) * 64];
        let mut w = [0u32; 80];
        
        for i in 0..16 {
            w[i] = ((chunk[i * 4] as u32) << 24)
                 | ((chunk[i * 4 + 1] as u32) << 16)
                 | ((chunk[i * 4 + 2] as u32) << 8)
                 | (chunk[i * 4 + 3] as u32);
        }
        
        for i in 16..80 {
            w[i] = (w[i-3] ^ w[i-8] ^ w[i-14] ^ w[i-16]).rotate_left(1);
        }
        
        let mut a = h0;
        let mut b = h1;
        let mut c = h2;
        let mut d = h3;
        let mut e = h4;
        
        for i in 0..80 {
            let (f, k) = if i < 20 {
                ((b & c) | ((!b) & d), 0x5A827999)
            } else if i < 40 {
                (b ^ c ^ d, 0x6ED9EBA1)
            } else if i < 60 {
                ((b & c) | (b & d) | (c & d), 0x8F1BBCDC)
            } else {
                (b ^ c ^ d, 0xCA62C1D6)
            };
            
            let temp = a.rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(w[i]);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        
        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }
    
    let mut result = [0u8; 20];
    for i in 0..4 {
        result[i] = ((h0 >> (24 - i * 8)) & 0xff) as u8;
        result[i + 4] = ((h1 >> (24 - i * 8)) & 0xff) as u8;
        result[i + 8] = ((h2 >> (24 - i * 8)) & 0xff) as u8;
        result[i + 12] = ((h3 >> (24 - i * 8)) & 0xff) as u8;
        result[i + 16] = ((h4 >> (24 - i * 8)) & 0xff) as u8;
    }
    
    result
}

// Simple Base64 encoding
pub fn base64_encode(data: &[u8], output: &mut [u8]) -> usize {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    
    let mut pos = 0;
    let mut i = 0;
    
    while i + 2 < data.len() {
        if pos + 4 > output.len() { break; }
        
        let b1 = data[i];
        let b2 = data[i + 1];
        let b3 = data[i + 2];
        
        output[pos] = TABLE[(b1 >> 2) as usize];
        output[pos + 1] = TABLE[(((b1 & 0x03) << 4) | (b2 >> 4)) as usize];
        output[pos + 2] = TABLE[(((b2 & 0x0f) << 2) | (b3 >> 6)) as usize];
        output[pos + 3] = TABLE[(b3 & 0x3f) as usize];
        
        pos += 4;
        i += 3;
    }
    
    let remaining = data.len() - i;
    if remaining > 0 && pos + 4 <= output.len() {
        let b1 = data[i];
        output[pos] = TABLE[(b1 >> 2) as usize];
        
        if remaining == 1 {
            output[pos + 1] = TABLE[((b1 & 0x03) << 4) as usize];
            output[pos + 2] = b'=';
            output[pos + 3] = b'=';
            pos += 4;
        } else {
            let b2 = data[i + 1];
            output[pos + 1] = TABLE[(((b1 & 0x03) << 4) | (b2 >> 4)) as usize];
            output[pos + 2] = TABLE[((b2 & 0x0f) << 2) as usize];
            output[pos + 3] = b'=';
            pos += 4;
        }
    }
    
    pos
}
