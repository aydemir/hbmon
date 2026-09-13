use rand::RngCore;

/// 16 hex-char uuid (64-bit, e.g. "a3f9c1e27b4d90f2").
/// Kısa tutulur (/tmp yolu + unix sock ~108 char sınırı) ama 32-bit
/// çakışma payı paralel daemon'da dardı (TASK-027: 4 byte → 8 byte).
pub fn generate_uuid() -> String {
    let mut bytes = [0u8; 8];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex_encode(&bytes)
}

/// `--uuid` açık değeri sock/out/log/pid yolu türetir
/// (`hbmon-<uuid>.sock`); path kaçağına karşı kilit (TASK-027).
/// Test uuid'leri (`itest-<pid>-<nanos>-<tag>`) ve eski 8-char
/// üretimler geçerlidir.
pub fn validate_uuid(u: &str) -> Result<(), String> {
    if u.is_empty() || u.len() > 64 {
        return Err(format!("invalid --uuid (len 1..=64): {:?}", u));
    }
    if !u
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(format!("invalid --uuid (charset [A-Za-z0-9_-]): {:?}", u));
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_is_16_hex() {
        let u = generate_uuid();
        assert_eq!(u.len(), 16);
        assert!(u.bytes().all(|b| b.is_ascii_hexdigit()));
        assert_ne!(generate_uuid(), u);
    }

    #[test]
    fn validate_accepts_known_shapes() {
        for ok in [
            "a3f9c1e2",                    // eski 8-char üretim
            &generate_uuid(),              // yeni 16-char üretim
            "itest-123-456789789-compact", // test uuid formu
            "a-B_9",
        ] {
            assert!(validate_uuid(ok).is_ok(), "{ok}");
        }
    }

    #[test]
    fn validate_rejects_path_escape() {
        for bad in [
            "",
            "../evil",
            "a/b",
            "a\\b",
            ".",
            "..",
            "a b",
            "ğ",
            &"x".repeat(65),
        ] {
            assert!(validate_uuid(bad).is_err(), "{bad:?}");
        }
    }
}
