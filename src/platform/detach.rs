//! Process detach + child process-group (RFC Katman 1).
//!
//! Unix: double-fork + setsid + stdio→/dev/null + cwd=/ + umask 077
//! (mevcut `daemonize()` birebir taşındı). Child `setpgid(0,0)` ile
//! kendi grubunda başlar — `kill_pgroup` hedefi.
//! Windows: M3'te self re-spawn (`DETACHED_PROCESS |
//! CREATE_NEW_PROCESS_GROUP`) + Job Object; M1'de stub.

/// Parent'tan kop, daemon bağlamına geç.
///
/// Unix: double-fork + setsid (dönerse daemon'dur).
/// Windows: kendini `--detach` bayrağı düşürülmüş argv ile detached
/// yeniden-spawn eder, parent hemen `exit(0)` döner (dönmez).
/// Handshake parent'ta ZATEN basılmıştır; child cwd'yi inherit eder
/// (build workdir'i korunur) ve `run_daemon`'u foreground koşar.
pub fn detach() -> Result<(), String> {
    #[cfg(unix)]
    {
        unix_detach()
    }
    #[cfg(windows)]
    {
        windows_detach()
    }
}

#[cfg(unix)]
fn unix_detach() -> Result<(), String> {
    unsafe {
        let p1 = libc::fork();
        if p1 < 0 {
            return Err("fork(1) failed".to_string());
        }
        if p1 > 0 {
            std::process::exit(0);
        }
        if libc::setsid() == -1 {
            return Err("setsid failed".to_string());
        }
        let p2 = libc::fork();
        if p2 < 0 {
            return Err("fork(2) failed".to_string());
        }
        if p2 > 0 {
            std::process::exit(0);
        }
        libc::close(0);
        libc::close(1);
        libc::close(2);
        let fd = libc::open(b"/dev/null\0".as_ptr() as *const libc::c_char, libc::O_RDWR);
        if fd >= 0 {
            libc::dup2(fd, 0);
            libc::dup2(fd, 1);
            libc::dup2(fd, 2);
            if fd > 2 {
                libc::close(fd);
            }
        }
    }
    crate::platform::signal::daemon_posture();
    let _ = std::env::set_current_dir("/");
    unsafe {
        libc::umask(0o077)
    };
    Ok(())
}

/// Child'ı kendi process grubunda başlat (sonradan grupça kill için).
pub fn child_group(cmd: &mut std::process::Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.pre_exec(|| {
            unsafe {
                libc::setpgid(0, 0);
            }
            Ok(())
        });
    }
    #[cfg(windows)]
    {
        // Grup kapsamı Job Object ile (child_spawned); flag gerekmez.
        let _ = cmd;
    }
}

/// Spawn sonrası kanca: Windows'ta child Job Object'e alınır ve
/// `kill_pgroup` kayıt defterine işlenir. Unix: no-op.
pub fn child_spawned(child: &std::process::Child) {
    #[cfg(unix)]
    {
        let _ = child;
    }
    #[cfg(windows)]
    {
        windows_child_spawned(child);
    }
}

#[cfg(windows)]
fn windows_detach() -> Result<(), String> {
    use super::winffi;
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {}", e))?;
    let args = filtered_argv();
    // Komut satırı: `"exe" "arg1" ...` (CreateProcessW mutable buffer ister).
    let mut cmdline = quote_arg(&exe.to_string_lossy());
    for a in &args {
        cmdline.push(' ');
        cmdline.push_str(&quote_arg(a));
    }
    let mut cmd_w = winffi::wide_nul(&cmdline);
    // stdio = NUL. bInheritHandles=FALSE ile child NE capture pipe'larını
    // NE DE başka handle devralır (yukarıdaki modül notu).
    let nul = winffi::wide_nul("NUL");
    let open_nul = || unsafe {
        winffi::CreateFileW(
            nul.as_ptr(),
            winffi::GENERIC_READ | winffi::GENERIC_WRITE,
            winffi::FILE_SHARE_READ | winffi::FILE_SHARE_WRITE,
            std::ptr::null_mut(),
            winffi::OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        )
    };
    let si = winffi::StartupInfo {
        cb: std::mem::size_of::<winffi::StartupInfo>() as u32,
        reserved: std::ptr::null_mut(),
        desktop: std::ptr::null_mut(),
        title: std::ptr::null_mut(),
        x: 0,
        y: 0,
        xsize: 0,
        ysize: 0,
        xcount: 0,
        ycount: 0,
        fill_attr: 0,
        flags: winffi::STARTF_USESTDHANDLES,
        show_window: 0,
        reserved2: 0,
        reserved2_ptr: std::ptr::null_mut(),
        std_input: open_nul(),
        std_output: open_nul(),
        std_error: open_nul(),
    };
    if !winffi::valid(si.std_input) || !winffi::valid(si.std_output) || !winffi::valid(si.std_error)
    {
        return Err("detach: NUL open failed".to_string());
    }
    let flags0 = winffi::DETACHED_PROCESS
        | winffi::CREATE_NEW_PROCESS_GROUP
        | winffi::CREATE_BREAKAWAY_FROM_JOB;
    let mut pi: winffi::ProcessInfo = unsafe { std::mem::zeroed() };
    let mut ok = unsafe {
        winffi::CreateProcessW(
            std::ptr::null(),
            cmd_w.as_mut_ptr(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0, // bInheritHandles = FALSE (kritik: pipe devralma yok)
            flags0,
            std::ptr::null_mut(),
            std::ptr::null(),
            &si,
            &mut pi,
        )
    };
    if ok == 0 {
        // Kısıtlı job (breakaway izni yok): bayraksız tekrar dene.
        ok = unsafe {
            winffi::CreateProcessW(
                std::ptr::null(),
                cmd_w.as_mut_ptr(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
                winffi::DETACHED_PROCESS | winffi::CREATE_NEW_PROCESS_GROUP,
                std::ptr::null_mut(),
                std::ptr::null(),
                &si,
                &mut pi,
            )
        };
    }
    unsafe {
        winffi::CloseHandle(si.std_input);
        winffi::CloseHandle(si.std_output);
        winffi::CloseHandle(si.std_error);
        if ok != 0 {
            winffi::CloseHandle(pi.process);
            winffi::CloseHandle(pi.thread);
        }
    }
    if ok == 0 {
        return Err(format!(
            "detach spawn: {}",
            unsafe { winffi::GetLastError() }
        ));
    }
    std::process::exit(0);
}

/// CreateProcessW komut satırı için MSVC-tarzı argüman alıntılama.
#[cfg(windows)]
fn quote_arg(s: &str) -> String {
    if s.is_empty() {
        return "\"\"".to_string();
    }
    let needs = s.chars().any(|c| c == ' ' || c == '\t' || c == '"' || c == '\n');
    if !needs {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    let mut backslashes = 0;
    for c in s.chars() {
        if c == '\\' {
            backslashes += 1;
        } else if c == '"' {
            for _ in 0..backslashes * 2 + 1 {
                out.push('\\');
            }
            out.push('"');
            backslashes = 0;
        } else {
            for _ in 0..backslashes {
                out.push('\\');
            }
            backslashes = 0;
            out.push(c);
        }
    }
    for _ in 0..backslashes * 2 {
        out.push('\\');
    }
    out.push('"');
    out
}

/// argv'den SADECE hbmon'un `--detach` bayrağını düşür:
/// `--` sonrasındaki build komutuna dokunulmaz, ilk eşleşme alınır.
/// (`--detach=true`/`--detach true` varyantları da kapsanır.)
#[cfg(windows)]
fn filtered_argv() -> Vec<String> {
    let mut out = vec![];
    let mut seen_dd = false;
    let mut stripped = false;
    for a in std::env::args().skip(1) {
        if a == "--" {
            seen_dd = true;
            out.push(a);
            continue;
        }
        if !seen_dd && !stripped && (a == "--detach" || a.starts_with("--detach=")) {
            stripped = true;
            continue;
        }
        out.push(a);
    }
    out
}

#[cfg(windows)]
fn windows_child_spawned(child: &std::process::Child) {
    use std::os::windows::io::AsRawHandle;
    use super::winffi;
    let proc = child.as_raw_handle() as winffi::HANDLE;
    let job = unsafe { winffi::CreateJobObjectW(std::ptr::null_mut(), std::ptr::null()) };
    if !winffi::valid(job) {
        return; // job yoksa kill_pgroup tree-kill'e düşer
    }
    let limits = winffi::JobBasicLimits::kill_on_close();
    let ok = unsafe {
        winffi::SetInformationJobObject(
            job,
            winffi::JOB_OBJECT_BASIC_LIMIT_INFO,
            &limits as *const _ as *const std::ffi::c_void,
            std::mem::size_of::<winffi::JobBasicLimits>() as u32,
        ) != 0 && winffi::AssignProcessToJobObject(job, proc) != 0
    };
    if ok {
        super::signal::register_job(child.id(), job);
    } else {
        unsafe {
            winffi::CloseHandle(job);
        }
    }
}
