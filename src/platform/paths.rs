//! Platform socket/file path conventions (RFC Katman 3 — keşif).
//!
//! Unix:    `/tmp/hbmon-<uuid>.sock` + `.jsonl` / `.pid` / `.out`
//! Windows: `\\.\pipe\hbmon-<uuid>` (named pipe) + `%TEMP%\hbmon-<uuid>.*`
//!
//! Bütün varsayılan yol üretimi burada; üst katmanlar (`daemon`,
//! `cli`, `ipc`) ham `/tmp` literali içermez. `SockAddr` unix'te
//! `PathBuf` alias'ıdır — unix çağrı noktaları sıfır-churn ile aynen
//! derlenir; Windows'ta pipe adı taşıyan küçük struct'tır.

use std::path::PathBuf;

#[cfg(unix)]
pub type SockAddr = PathBuf;

#[cfg(windows)]
#[derive(Debug, Clone)]
pub struct SockAddr {
    name: String,
}

#[cfg(windows)]
impl SockAddr {
    pub fn pipe_name(&self) -> &str {
        &self.name
    }
}

/// `--sock` / `$HBMON_SOCK` ile verilen açık adresi SockAddr'a çevirir.
pub fn from_explicit(p: PathBuf) -> SockAddr {
    #[cfg(unix)]
    {
        p
    }
    #[cfg(windows)]
    {
        SockAddr {
            name: from_explicit_windows(p),
        }
    }
}

/// `$HBMON_SOCK` değeri → SockAddr.
pub fn from_env(s: &str) -> SockAddr {
    #[cfg(unix)]
    {
        PathBuf::from(s)
    }
    #[cfg(windows)]
    {
        SockAddr {
            name: if s.starts_with(r"\\.\pipe\") {
                s.to_string()
            } else {
                format!(r"\\.\pipe\{}", s)
            },
        }
    }
}

#[cfg(windows)]
fn from_explicit_windows(p: PathBuf) -> String {
    let s = p.to_string_lossy().to_string();
    if s.starts_with(r"\\.\pipe\") {
        s
    } else {
        // `--sock /tmp/hbmon-x.sock` gibi unix-tarzı değer verilirse
        // uuid'yi kurtar, yoksa adı aynen pipe yap.
        match stem_uuid(&s) {
            Some(u) => format!(r"\\.\pipe\hbmon-{}", u),
            None => format!(r"\\.\pipe\{}", s),
        }
    }
}

/// `hbmon-<uuid>` eser adından uuid'yi çeker (.sock/.jsonl/.pid/.out).
/// Her platformda aynı (TASK-017 `list` taraması için ortak yardımcı).
/// Üretilen uuid'ler hex olduğundan sonek çakışması olmaz.
pub fn uuid_from_base(base: &str) -> Option<String> {
    let base = base.rsplit('/').next().unwrap_or(base);
    let base = base.rsplit('\\').next().unwrap_or(base);
    let base = base.strip_prefix("hbmon-")?;
    let base = base
        .strip_suffix(".sock")
        .or_else(|| base.strip_suffix(".jsonl"))
        .or_else(|| base.strip_suffix(".pid"))
        .or_else(|| base.strip_suffix(".out"))
        .unwrap_or(base);
    if base.is_empty() {
        None
    } else {
        Some(base.to_string())
    }
}

/// `hbmon-<uuid>.sock` / `hbmon-<uuid>` kalıbından uuid'yi çeker.
#[cfg(windows)]
fn stem_uuid(s: &str) -> Option<String> {
    uuid_from_base(s)
}

/// Varsayılan transport adresi (uuid yoksa üretilmez — caller üretir).
pub fn default_sock(uuid: &str) -> SockAddr {
    #[cfg(unix)]
    {
        PathBuf::from(format!("/tmp/hbmon-{}.sock", uuid))
    }
    #[cfg(windows)]
    {
        SockAddr {
            name: format!(r"\\.\pipe\hbmon-{}", uuid),
        }
    }
}

pub fn default_log(uuid: &str) -> PathBuf {
    base_dir().join(format!("hbmon-{}.jsonl", uuid))
}

pub fn default_pidfile(uuid: &str) -> PathBuf {
    base_dir().join(format!("hbmon-{}.pid", uuid))
}

pub fn default_out(uuid: &str) -> PathBuf {
    base_dir().join(format!("hbmon-{}.out", uuid))
}

pub fn default_workdir() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| base_dir())
}

#[cfg(unix)]
fn base_dir() -> PathBuf {
    PathBuf::from("/tmp")
}

#[cfg(windows)]
fn base_dir() -> PathBuf {
    std::env::temp_dir()
}

/// Handshake JSON / log satırlarındaki adres metni.
pub fn sock_display(a: &SockAddr) -> String {
    #[cfg(unix)]
    {
        a.to_string_lossy().to_string()
    }
    #[cfg(windows)]
    {
        a.name.clone()
    }
}

/// Double-spawn guard + cleanup liveness: bu adreste canlı monitor var mı?
pub fn sock_exists(a: &SockAddr) -> bool {
    #[cfg(unix)]
    {
        a.exists()
    }
    #[cfg(windows)]
    {
        pipe_probe(&a.name)
    }
}

/// `serve()` öncesi bayat dosya temizliği. Windows'ta pipe dosya
/// değildir → no-op (bind zaten sahiplenmeyi çözer).
pub fn sock_remove(a: &SockAddr) {
    #[cfg(unix)]
    {
        std::fs::remove_file(a).ok();
    }
    #[cfg(windows)]
    {
        let _ = a;
    }
}

/// Var olan yol bir socket dosyası DEĞİLSE hata metni (TASK-047/S1).
/// `spawn_watch` bayat-socket temizliği eskiden koşulsuz `remove_file`
/// çağırıyordu: `--sock /tmp/veri.txt` gibi bir yazım hatası kullanıcı
/// dosyasını sessizce siliyordu. Symlink de socket sayılmaz (RFC 13.2.1:
/// symlink açılışta reddedilir) → korunur. Yolun YOK olması normaldir
/// (ilk spawn) ve çakışma sayılmaz. Windows: dosya semantiği yok.
pub fn sock_non_socket(a: &SockAddr) -> Option<String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        match std::fs::symlink_metadata(a) {
            Ok(m) if !m.file_type().is_socket() => Some(format!(
                "refusing to use non-socket path: {} (pass a fresh --sock)",
                a.display()
            )),
            _ => None,
        }
    }
    #[cfg(windows)]
    {
        let _ = a;
        None
    }
}

/// Bayat socket temizliği — yalnız gerçek socket dosyası için. Socket
/// olmayan yolda `Err` (veri kaybı koruması, TASK-047/S1); daemon'un
/// kendi çıkış temizliği `sock_remove` ile ayrı kalır. Yol yoksa no-op.
pub fn sock_remove_stale(a: &SockAddr) -> Result<(), String> {
    if let Some(msg) = sock_non_socket(a) {
        return Err(msg);
    }
    #[cfg(unix)]
    {
        match std::fs::remove_file(a) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("remove {}: {}", a.display(), e)),
        }
    }
    #[cfg(windows)]
    {
        let _ = a;
        Ok(())
    }
}

/// Convention-scan dizini (RFC Katman 3B). Unix: `/tmp`.
pub fn scan_dir() -> PathBuf {
    #[cfg(unix)]
    {
        PathBuf::from("/tmp")
    }
    #[cfg(windows)]
    {
        std::env::temp_dir()
    }
}

/// En yeni `hbmon-*` adresi → SockAddr. Unix: `/tmp` mtime taraması;
/// Windows: `\\.\pipe\hbmon-*` enumerate (FindFirstFileW, yazma
/// zamanına göre en yeni). Yoksa `None` (caller `$HBMON_SOCK` ister).
pub fn scan_newest_sock() -> Option<SockAddr> {
    #[cfg(unix)]
    {
        let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
        if let Ok(dir) = std::fs::read_dir("/tmp") {
            for e in dir.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if name.starts_with("hbmon-") && name.ends_with(".sock") {
                    let p = e.path();
                    let mtime = e.metadata().and_then(|m| m.modified()).ok();
                    if let Some(mt) = mtime {
                        if best.as_ref().map(|(t, _)| mt > *t).unwrap_or(true) {
                            best = Some((mt, p));
                        }
                    }
                }
            }
        }
        best.map(|(_, p)| p)
    }
    #[cfg(windows)]
    {
        scan_newest_pipe()
    }
}

/// Windows: `\\.\pipe\hbmon-*` içinde yazma zamanı en yeni pipe.
#[cfg(windows)]
fn scan_newest_pipe() -> Option<SockAddr> {
    // WIN32_FIND_DATAW düzeni: attrs(4) + creation(8) + last_access(8) +
    // last_write(8: lo+hi) + sizes/reserved(16) + name[260] + alt[14].
    #[repr(C)]
    struct FindData {
        _head: [u8; 20],
        write_lo: u32,
        write_hi: u32,
        _mid: [u8; 16],
        name: [u16; 260],
        _alt: [u16; 14],
    }
    extern "system" {
        fn FindFirstFileW(pattern: *const u16, data: *mut FindData) -> super::winffi::HANDLE;
        fn FindNextFileW(h: super::winffi::HANDLE, data: *mut FindData) -> super::winffi::BOOL;
        fn FindClose(h: super::winffi::HANDLE) -> super::winffi::BOOL;
    }
    use super::winffi;
    let pattern = winffi::wide_nul(r"\\.\pipe\hbmon-*");
    let mut data: FindData = unsafe { std::mem::zeroed() };
    let h = unsafe { FindFirstFileW(pattern.as_ptr(), &mut data as *mut FindData) };
    if !winffi::valid(h) {
        return None;
    }
    let mut best: Option<((u32, u32), String)> = None;
    loop {
        let end = data
            .name
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(data.name.len());
        let name = String::from_utf16_lossy(&data.name[..end]);
        if name.starts_with("hbmon-") {
            let key = (data.write_hi, data.write_lo);
            if best.as_ref().map(|(k, _)| key > *k).unwrap_or(true) {
                best = Some((key, name));
            }
        }
        if unsafe { FindNextFileW(h, &mut data as *mut FindData) } == 0 {
            break;
        }
    }
    unsafe {
        FindClose(h);
    }
    best.map(|(_, n)| SockAddr {
        name: format!(r"\\.\pipe\{}", n),
    })
}

/// Windows: pipe adında bir dinleyici var mı? (`CreateFileW` probe —
/// bağlanmaz, sadece varlık yoklar.)
#[cfg(windows)]
fn pipe_probe(name: &str) -> bool {
    use std::os::windows::ffi::OsStrExt;
    let wide: Vec<u16> = std::ffi::OsStr::new(name)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    extern "system" {
        fn CreateFileW(
            name: *const u16,
            access: u32,
            share: u32,
            sec: *mut std::ffi::c_void,
            disp: u32,
            flags: u32,
            tmpl: *mut std::ffi::c_void,
        ) -> *mut std::ffi::c_void;
        fn CloseHandle(h: *mut std::ffi::c_void) -> i32;
    }
    const OPEN_EXISTING: u32 = 3;
    // INVALID_HANDLE_VALUE = -1
    unsafe {
        let h = CreateFileW(
            wide.as_ptr(),
            0,
            0,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        );
        if h.is_null() || h as isize == -1 {
            return false;
        }
        CloseHandle(h);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_from_artifact_names() {
        assert_eq!(
            uuid_from_base("hbmon-a3f9c1e2.sock"),
            Some("a3f9c1e2".to_string())
        );
        assert_eq!(
            uuid_from_base("hbmon-a3f9c1e2.jsonl"),
            Some("a3f9c1e2".to_string())
        );
        assert_eq!(
            uuid_from_base(r"\\.\pipe\hbmon-a3f9c1e2"),
            Some("a3f9c1e2".to_string())
        );
        assert_eq!(uuid_from_base("hbmon-.sock"), None);
        assert_eq!(uuid_from_base("other-a3f9c1e2.sock"), None);
        assert_eq!(uuid_from_base("hbmon-"), None);
    }

    #[test]
    fn default_addrs_match_convention() {
        let s = default_sock("a3f9c1e2");
        #[cfg(unix)]
        assert_eq!(sock_display(&s), "/tmp/hbmon-a3f9c1e2.sock");
        #[cfg(windows)]
        assert_eq!(sock_display(&s), r"\\.\pipe\hbmon-a3f9c1e2");
        assert!(default_log("a3f9c1e2").ends_with("hbmon-a3f9c1e2.jsonl"));
        assert!(default_pidfile("a3f9c1e2").ends_with("hbmon-a3f9c1e2.pid"));
        assert!(default_out("a3f9c1e2").ends_with("hbmon-a3f9c1e2.out"));
    }

    #[test]
    fn env_value_maps_to_sock() {
        #[cfg(unix)]
        {
            let s = from_env("/tmp/hbmon-x.sock");
            assert_eq!(sock_display(&s), "/tmp/hbmon-x.sock");
        }
        #[cfg(windows)]
        {
            let full = from_env(r"\\.\pipe\hbmon-x");
            assert_eq!(sock_display(&full), r"\\.\pipe\hbmon-x");
            let bare = from_env("hbmon-x");
            assert_eq!(sock_display(&bare), r"\\.\pipe\hbmon-x");
        }
    }

    #[cfg(windows)]
    #[test]
    fn explicit_windows_forms_recover_uuid() {
        use std::path::PathBuf;
        // Tam pipe adı aynen geçer.
        let a = from_explicit(PathBuf::from(r"\\.\pipe\hbmon-abc"));
        assert_eq!(sock_display(&a), r"\\.\pipe\hbmon-abc");
        // Unix-tarzı sock yolu uuid'ye indirgenir.
        let b = from_explicit(PathBuf::from("/tmp/hbmon-abc.sock"));
        assert_eq!(sock_display(&b), r"\\.\pipe\hbmon-abc");
        // Çıplak ad pipe yapılır.
        let c = from_explicit(PathBuf::from("hbmon-abc"));
        assert_eq!(sock_display(&c), r"\\.\pipe\hbmon-abc");
    }

    #[cfg(unix)]
    #[test]
    fn explicit_unix_is_identity() {
        use std::path::PathBuf;
        let p = PathBuf::from("/tmp/hbmon-abc.sock");
        assert_eq!(from_explicit(p.clone()), p);
    }

    /// TASK-047/S1: socket-olmayan yol silinmez (veri kaybı koruması).
    #[cfg(unix)]
    #[test]
    fn non_socket_path_refused_and_kept() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("hbmon-userdata.sock");
        std::fs::write(&p, b"important user data").unwrap();
        let addr = from_explicit(p.clone());
        assert!(sock_non_socket(&addr).is_some());
        assert!(sock_remove_stale(&addr).is_err());
        assert_eq!(std::fs::read(&p).unwrap(), b"important user data");
    }

    #[cfg(unix)]
    #[test]
    fn stale_socket_is_removed_and_missing_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("hbmon-stale.sock");
        let l = std::os::unix::net::UnixListener::bind(&p).unwrap();
        let addr = from_explicit(p.clone());
        assert!(sock_non_socket(&addr).is_none());
        assert!(sock_remove_stale(&addr).is_ok());
        assert!(!p.exists());
        drop(l);
        // Yok olan yol: ne çakışma ne hata (idempotent bayat temizlik).
        assert!(sock_non_socket(&addr).is_none());
        assert!(sock_remove_stale(&addr).is_ok());
    }

    /// Symlink socket sayılmaz: hem reddedilir hem hedef korunur.
    #[cfg(unix)]
    #[test]
    fn symlink_path_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("hbmon-real.sock");
        let link = dir.path().join("hbmon-link.sock");
        let l = std::os::unix::net::UnixListener::bind(&real).unwrap();
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let addr = from_explicit(link.clone());
        assert!(sock_non_socket(&addr).is_some());
        assert!(sock_remove_stale(&addr).is_err());
        drop(l);
        assert!(real.exists() && link.exists(), "symlink/hedef korunmalı");
    }
}
