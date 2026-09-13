pub mod events;

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json::Value;

use crate::platform::perm::{secure_fix, SecureMode};

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
            .secure_mode(0o600)
            .open(path)
            .map_err(|e| format!("open log {}: {}", path.display(), e))?;
        secure_fix(path);
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
            let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();
            let drop_n = lines.len() / 10 + 1;
            if let Ok(mut f) = self.file.lock() {
                // truncate + rewrite tail
                if let Ok(nf) = OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .secure_mode(0o600)
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
        Self::tail_filter(path, n, None)
    }

    /// Son N satırın `event` alt-kümesi (TASK-029): `None` = filtresiz.
    /// Eşleşme `new_event` şeklindeki `"ev":"<ad>"` alt-dizesinedir —
    /// satırlar zaten compact-JSON olduğundan parse maliyeti yok.
    pub fn tail_filter(path: &Path, n: usize, event: Option<&str>) -> Vec<String> {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(_) => return vec![],
        };
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();
        let kept: Vec<String> = match event {
            Some(ev) => {
                let needle = format!("\"ev\":\"{}\"", ev);
                lines.into_iter().filter(|l| l.contains(&needle)).collect()
            }
            None => lines,
        };
        if kept.len() <= n {
            kept
        } else {
            kept[kept.len() - n..].to_vec()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn write_lines(p: &Path, lines: &[&str]) {
        use std::io::Write;
        let mut f = File::create(p).unwrap();
        for l in lines {
            writeln!(f, "{}", l).unwrap();
        }
    }

    #[test]
    fn tail_filter_keeps_last_n_matching() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("t.jsonl");
        write_lines(
            &p,
            &[
                r#"{"v":1,"ev":"metric","uuid":"u"}"#,
                r#"{"v":1,"ev":"exit","uuid":"u"}"#,
                r#"{"v":1,"ev":"metric","uuid":"u"}"#,
            ],
        );
        // Filtresiz = tail ile aynı.
        assert_eq!(EventLogger::tail(&p, 2).len(), 2);
        let m = EventLogger::tail_filter(&p, 10, Some("metric"));
        assert_eq!(m.len(), 2);
        assert!(m.iter().all(|l| l.contains(r#""ev":"metric""#)));
        // Sınır eşleşenler içinden son N.
        let one = EventLogger::tail_filter(&p, 1, Some("metric"));
        assert_eq!(one.len(), 1);
        // Eşleşmeyen + kayıp dosya = boş.
        assert!(EventLogger::tail_filter(&p, 5, Some("bogus-ev")).is_empty());
        assert!(
            EventLogger::tail_filter(&dir.path().join("yok.jsonl"), 5, Some("metric")).is_empty()
        );
    }
}
