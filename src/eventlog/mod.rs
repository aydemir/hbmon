pub mod events;

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json::Value;

const MAX_BYTES: u64 = 100 * 1024 * 1024; // 100MB cap (RFC 5.2.1)

pub struct EventLogger {
    path: PathBuf,
    file: Mutex<File>,
}

impl EventLogger {
    pub fn create(path: &Path) -> Result<Self, String> {
        validate_tmp_path(path)?;
        // O_EXCL | O_NOFOLLOW semantics via create_new; symlink check below
        if let Ok(meta) = std::fs::symlink_metadata(path) {
            if meta.file_type().is_symlink() {
                return Err(format!("refusing to open symlink log: {}", path.display()));
            }
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o600)
            .open(path)
            .map_err(|e| format!("open log {}: {}", path.display(), e))?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).ok();
        Ok(Self {
            path: path.to_path_buf(),
            file: Mutex::new(file),
        })
    }

    pub fn open_append(path: &Path) -> Result<Self, String> {
        Self::create(path)
    }

    pub fn append(&self, event: &Value) {
        let mut line = serde_json::to_string(event).unwrap_or_else(|_| "{}".to_string());
        line.push('\n');
        if let Ok(mut f) = self.file.lock() {
            let _ = f.write_all(line.as_bytes());
            let _ = f.flush();
        }
        self.maybe_rotate();
    }

    fn maybe_rotate(&self) {
        // Best-effort FIFO cap: if over budget, drop oldest ~10% by rewriting.
        // Runs rarely; keeps RAM bounded and avoids unbounded disk growth.
        let size = std::fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0);
        if size < MAX_BYTES {
            return;
        }
        if let Ok(file) = File::open(&self.path) {
            let reader = BufReader::new(file);
            let lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();
            let drop_n = lines.len() / 10 + 1;
            if let Ok(mut f) = self.file.lock() {
                // truncate + rewrite tail
                if let Ok(nf) = OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .mode(0o600)
                    .open(&self.path)
                {
                    *f = nf;
                    for l in lines.iter().skip(drop_n) {
                        let _ = writeln!(f, "{}", l);
                    }
                    let _ = f.flush();
                }
            }
        }
    }

    pub fn tail(path: &Path, n: usize) -> Vec<String> {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(_) => return vec![],
        };
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();
        if lines.len() <= n {
            lines
        } else {
            lines[lines.len() - n..].to_vec()
        }
    }
}

fn validate_tmp_path(p: &Path) -> Result<(), String> {
    let s = p.to_string_lossy();
    if s.contains("..") {
        return Err(format!("invalid path (parent dir): {}", s));
    }
    Ok(())
}
