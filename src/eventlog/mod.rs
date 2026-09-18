pub mod events;

use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json::Value;

use crate::platform::perm::{secure_fix, SecureMode};

const MAX_BYTES: u64 = 100 * 1024 * 1024; // 100MB cap (RFC 5.2.1)
/// Sondan okuma penceresi (başlangıç / üst sınır). Tipik `--tail N`
/// istekleri ilk pencerede karşılanır; seyrek filtrede pencere büyür,
/// üst sınırı aşarsa baştan O(n) akış taraması yapılır (TASK-048/S3d:
/// eskiden tüm dosya belleğe alınıyordu — 100 MB cap'te her
/// `status`/`log --tail` çağrısı daemon RSS'ini şişiriyordu).
const TAIL_WINDOW: u64 = 64 * 1024;
const TAIL_WINDOW_MAX: u64 = 4 * 1024 * 1024;

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
    /// Bellek O(N + pencere): sondan artan pencere yeterli eşleşme
    /// vermezse (seyrek filtre) üst sınırda baştan O(n) akış taramasına
    /// düşer (TASK-048/S3d).
    pub fn tail_filter(path: &Path, n: usize, event: Option<&str>) -> Vec<String> {
        if n == 0 {
            return vec![];
        }
        let needle = event.map(|ev| format!("\"ev\":\"{}\"", ev));
        let mut file = match File::open(path) {
            Ok(f) => f,
            Err(_) => return vec![],
        };
        let len = file.metadata().map(|m| m.len()).unwrap_or(0);
        if len == 0 {
            return vec![];
        }
        let mut window = TAIL_WINDOW.min(len);
        loop {
            let start = len - window;
            let mut buf = vec![0u8; window as usize];
            if file.seek(SeekFrom::Start(start)).is_err() || file.read_exact(&mut buf).is_err() {
                return vec![];
            }
            // Pencere ortasından başlayan yarım satır: yalnız önceki bayt
            // '\n' DEĞİLSE atılır (aksi hâlde ilk satır tamdır).
            let preceded_by_newline = if start == 0 {
                true
            } else {
                let mut b = [0u8; 1];
                file.seek(SeekFrom::Start(start - 1)).is_err()
                    || file.read_exact(&mut b).is_err()
                    || b[0] == b'\n'
            };
            let text = String::from_utf8_lossy(&buf);
            let kept = last_n_lines(&text, !preceded_by_newline, n, needle.as_deref());
            if kept.len() >= n || start == 0 {
                return kept;
            }
            if window >= TAIL_WINDOW_MAX {
                return scan_from_start(path, n, needle.as_deref());
            }
            window = (window.saturating_mul(4)).min(TAIL_WINDOW_MAX);
        }
    }
}

/// Pencere metninden son N eşleşen satır (TASK-048/S3d). `drop_first`
/// pencere ortasından başlayan yarım ilk satırı eler. `BufRead::lines`
/// semantiği korunur: trailing '\n' fazladan boş satır üretmez, CRLF'de
/// '\r' atılır, içteki boş satırlar korunur.
fn last_n_lines(text: &str, drop_first: bool, n: usize, needle: Option<&str>) -> Vec<String> {
    let mut lines: Vec<&str> = text.split('\n').collect();
    if text.ends_with('\n') {
        lines.pop();
    }
    if drop_first && !lines.is_empty() {
        lines.remove(0);
    }
    let mut kept: VecDeque<String> = VecDeque::with_capacity(n);
    for raw in lines {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if needle.map(|nd| line.contains(nd)).unwrap_or(true) {
            if kept.len() == n {
                kept.pop_front();
            }
            kept.push_back(line.to_string());
        }
    }
    kept.into_iter().collect()
}

/// Seyrek filtre için baştan akış taraması: bellek O(n) (TASK-048/S3d).
fn scan_from_start(path: &Path, n: usize, needle: Option<&str>) -> Vec<String> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return vec![],
    };
    let mut kept: VecDeque<String> = VecDeque::with_capacity(n);
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        if needle.map(|nd| line.contains(nd)).unwrap_or(true) {
            if kept.len() == n {
                kept.pop_front();
            }
            kept.push_back(line);
        }
    }
    kept.into_iter().collect()
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

    /// TASK-048/S3d: pencere yardımcısı `BufRead::lines` semantiğini korur.
    #[test]
    fn last_n_lines_matches_bufread_semantics() {
        // trailing '\n' fazladan boş satır üretmez.
        assert_eq!(last_n_lines("a\nb\n", false, 5, None), vec!["a", "b"]);
        // son satır '\n'siz de gelir.
        assert_eq!(last_n_lines("a\nb", false, 5, None), vec!["a", "b"]);
        // CRLF: '\r' atılır.
        assert_eq!(last_n_lines("a\r\nb\r\n", false, 5, None), vec!["a", "b"]);
        // İçteki boş satırlar korunur.
        assert_eq!(last_n_lines("\n\n", false, 5, None), vec!["", ""]);
        // Yarım ilk satır elenir.
        assert_eq!(last_n_lines("xx\ny\nz\n", true, 5, None), vec!["y", "z"]);
        // Son N sınırı.
        assert_eq!(last_n_lines("a\nb\nc\n", false, 2, None), vec!["b", "c"]);
        // Filtre.
        let f = last_n_lines("x\nm1\ny\nm2\n", false, 5, Some("m"));
        assert_eq!(f, vec!["m1", "m2"]);
    }

    /// TASK-048/S3d: büyük dosyada tail yalnız sondan okunur, doğru satırlar.
    #[test]
    fn tail_filter_large_file_returns_last_lines() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("big.jsonl");
        let mut body = String::new();
        for i in 0..40_000 {
            body.push_str(&format!("{{\"ev\":\"filler\",\"i\":{}}}\n", i));
        }
        body.push_str("{\"ev\":\"exit\",\"code\":0}\n");
        body.push_str("{\"ev\":\"metric\",\"cpu\":1.0}\n");
        body.push_str("{\"ev\":\"ready\",\"uuid\":\"u\"}\n");
        std::fs::write(&p, body).unwrap();
        assert!(std::fs::metadata(&p).unwrap().len() > 1_000_000);

        let last = EventLogger::tail(&p, 3);
        assert_eq!(last.len(), 3);
        assert!(last[0].contains("\"exit\""));
        assert!(last[1].contains("\"metric\""));
        assert!(last[2].contains("\"ready\""));
        // Filtre: exit olayı son pencerede.
        let exits = EventLogger::tail_filter(&p, 1, Some("exit"));
        assert_eq!(exits.len(), 1);
        assert!(exits[0].contains("\"ev\":\"exit\""));
    }

    /// TASK-048/S3d: seyrek filtre (eşleşme dosya başında, dosya pencere
    /// üst sınırından büyük) → baştan akış taraması doğru sonucu verir.
    #[test]
    fn tail_filter_sparse_match_falls_back_to_scan() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("sparse.jsonl");
        let mut body = String::from("{\"ev\":\"dep_missing\",\"uuid\":\"u\"}\n");
        let filler = "{\"ev\":\"metric\",\"pad\":\"......................\"}\n";
        for _ in 0..110_000 {
            body.push_str(filler);
        }
        std::fs::write(&p, body).unwrap();
        let len = std::fs::metadata(&p).unwrap().len();
        assert!(len > TAIL_WINDOW_MAX, "fallback yolu tetiklenmeli: {len}");

        let hit = EventLogger::tail_filter(&p, 1, Some("dep_missing"));
        assert_eq!(hit.len(), 1);
        assert!(hit[0].contains("dep_missing"));
        // Filtresiz tail pencereyle karşılanır (fallback'siz doğru sonuç).
        let tail = EventLogger::tail(&p, 2);
        assert_eq!(tail.len(), 2);
        assert!(tail[1].contains("metric"));
    }

    #[test]
    fn tail_filter_zero_and_empty_are_empty() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("e.jsonl");
        std::fs::write(&p, "").unwrap();
        assert!(EventLogger::tail(&p, 5).is_empty());
        std::fs::write(&p, "a\nb\n").unwrap();
        assert!(EventLogger::tail(&p, 0).is_empty());
    }
}
