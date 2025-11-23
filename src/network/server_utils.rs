pub fn htons(port: u16) -> u16 {
    ((port & 0xff) << 8) | ((port >> 8) & 0xff)
}


