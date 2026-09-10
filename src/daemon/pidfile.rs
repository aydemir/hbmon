use std::fs::OpenOptions;
use std::path::Path;

use crate::platform::perm::{secure_fix, SecureMode};

pub fn write_pidfile(path: &Path, pid: u32) -> Result<(), String> {
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() {
            return Err(format!("refusing pidfile symlink: {}", path.display()));
        }
    }
    let mut f = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .secure_mode(0o600)
        .open(path)
        .map_err(|e| format!("pidfile {}: {}", path.display(), e))?;
    use std::io::Write;
    writeln!(f, "{}", pid).map_err(|e| e.to_string())?;
    secure_fix(path);
    Ok(())
}

pub fn read_pidfile(path: &Path) -> Result<u32, String> {
    let s = std::fs::read_to_string(path).map_err(|e| format!("read pidfile: {}", e))?;
    s.trim()
        .parse::<u32>()
        .map_err(|e| format!("bad pidfile: {}", e))
}

pub fn remove(path: &Path) {
    std::fs::remove_file(path).ok();
}
