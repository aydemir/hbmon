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

/// `hbmon-<uuid>.sock` / `hbmon-<uuid>` kalıbından uuid'yi çeker.
#[cfg(windows)]
fn stem_uuid(s: &str) -> Option<String> {
    let base = s.replace('\\', "/").rsplit('/').next()?.to_string();
    let base = base.strip_prefix("hbmon-")?;
    let base = base.strip_suffix(".sock").unwrap_or(base);
    if base.is_empty() {
        None
    } else {
        Some(base.to_string())
    }
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
