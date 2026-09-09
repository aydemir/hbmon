//! Dosya izin soyutlaması: pid/sock/jsonl/out hepsi unix'te `0600`.
//!
//! `OpenOptionsExt::mode` unix-only olduğu için çağrı noktaları
//! `std::os::unix` import edemezdi. `SecureMode::secure_mode` her
//! platformda derlenir: unix'te mode bitini koyar, Windows'ta no-op
//! (ACL varsayılanı; bkz. TASK-006 ilke notu).

use std::fs::OpenOptions;
use std::path::Path;

pub trait SecureMode {
    fn secure_mode(&mut self, mode: u32) -> &mut Self;
}

#[cfg(unix)]
impl SecureMode for OpenOptions {
    fn secure_mode(&mut self, mode: u32) -> &mut Self {
        use std::os::unix::fs::OpenOptionsExt;
        self.mode(mode)
    }
}

#[cfg(windows)]
impl SecureMode for OpenOptions {
    fn secure_mode(&mut self, _mode: u32) -> &mut Self {
        self
    }
}

/// Açılış sonrası izin düzeltmesi (`set_permissions(0o600)` eşdeğeri).
pub fn secure_fix(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).ok();
    }
    #[cfg(windows)]
    {
        let _ = path;
    }
}
