use rand::RngCore;

/// 8 hex-char short uuid (e.g. "a3f9c1e2").
/// Full UUID4 would be nicer, but short ids keep /tmp paths readable
/// and are collision-safe enough for local monitors.
pub fn generate_uuid() -> String {
    let mut bytes = [0u8; 4];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex_encode(&bytes)
}

fn hex_encode(b: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(b.len() * 2);
    for &byte in b {
        s.push(HEX[(byte >> 4) as usize] as char);
        s.push(HEX[(byte & 0xf) as usize] as char);
    }
    s
}
