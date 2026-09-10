//! Sinyal soyutlaması: parse + process-group kill + daemon posture.
//!
//! Wire'da sinyal SAYI olarak taşınır (`protocol.rs` `signal: i32`
//! değişmez): Term=15, Kill=9, Int=2, Hup=1 — unix numaralarıyla
//! birebir, böylece eski/yeni client-daemon karışımları uyumlu kalır.
//! Unix'te `kill(-pgid)` aynen; Windows'ta Job-Object karşılığı M3'te
//! dolar (M1 stub: no-op).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sig {
    Term,
    Kill,
    Int,
    Hup,
}

impl Sig {
    pub fn to_num(self) -> i32 {
        match self {
            Sig::Term => 15,
            Sig::Kill => 9,
            Sig::Int => 2,
            Sig::Hup => 1,
        }
    }

    pub fn from_num(n: i64) -> Self {
        match n {
            9 => Sig::Kill,
            2 => Sig::Int,
            1 => Sig::Hup,
            _ => Sig::Term,
        }
    }

    #[cfg(unix)]
    fn to_libc(self) -> i32 {
        match self {
            Sig::Term => libc::SIGTERM,
            Sig::Kill => libc::SIGKILL,
            Sig::Int => libc::SIGINT,
            Sig::Hup => libc::SIGHUP,
        }
    }
}

pub fn parse_signal(s: &str) -> Result<Sig, String> {
    let u = s.to_uppercase();
    let v = u.strip_prefix("SIG").unwrap_or(&u);
    match v {
        "TERM" | "15" => Ok(Sig::Term),
        "KILL" | "9" => Ok(Sig::Kill),
        "INT" | "2" => Ok(Sig::Int),
        "HUP" | "1" => Ok(Sig::Hup),
        _ => Err(format!("unknown signal: {}", s)),
    }
}

/// Tüm süreç grubuna sinyal (RFC 4.5).
/// Unix: `kill(-pgid)`. Windows: Job-Object terminate (M3);
/// job atanamadıysa ağaçça `TerminateProcess` fallback.
/// Windows'ta graceful TERM yok — Term de Kill de terminate eder
/// (RFC'ye M4'te not düşülür). Dönüş: sinyal gönderilebildi mi?
pub fn kill_pgroup(pid: u32, sig: Sig) -> bool {
    #[cfg(unix)]
    unsafe {
        libc::kill(-(pid as i32), sig.to_libc()) == 0
    }
    #[cfg(windows)]
    {
        windows_kill(pid, sig)
    }
}

#[cfg(windows)]
static JOBS: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<u32, JobHandle>>> =
    std::sync::OnceLock::new();

/// Job handle sarmalayıcı: registry daemon ömrü boyunca tutar, kapatmaz;
/// okuma sadece kill yolunda. Münhasır sahiplenme → Send+Sync güvenli.
#[cfg(windows)]
#[derive(Clone, Copy)]
struct JobHandle(super::winffi::HANDLE);
#[cfg(windows)]
unsafe impl Send for JobHandle {}
#[cfg(windows)]
unsafe impl Sync for JobHandle {}

/// `detach::child_spawned` tarafından çağrılır: root pid → job handle.
#[cfg(windows)]
pub fn register_job(pid: u32, job: super::winffi::HANDLE) {
    if let Ok(mut m) = JOBS
        .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
        .lock()
    {
        m.insert(pid, JobHandle(job));
    }
}

#[cfg(windows)]
fn windows_kill(pid: u32, sig: Sig) -> bool {
    let _ = sig; // Windows: Term/Kill ayrımı yok (terminate).
    let job = JOBS
        .get()
        .and_then(|m| m.lock().ok())
        .and_then(|m| m.get(&pid).copied())
        .map(|j| j.0);
    if let Some(j) = job {
        return unsafe { super::winffi::TerminateJobObject(j, 1) } != 0;
    }
    // Job yoksa (atama başarısızdı): ağaçça öldür.
    crate::proc::windows::kill_tree(pid)
}

/// Daemon sinyal duruşu (RFC 13.1.4): SIGHUP/SIGPIPE ignore.
/// Windows'ta karşılık yok → no-op.
pub fn daemon_posture() {
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGHUP, libc::SIG_IGN);
        libc::signal(libc::SIGPIPE, libc::SIG_IGN);
    }
    #[cfg(windows)]
    {}
}
