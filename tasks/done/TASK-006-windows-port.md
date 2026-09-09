---
id: TASK-006
title: "Windows portu (named pipe + Win32 inspector)"
status: done
priority: P2
created: 2026-09-09
updated: 2026-09-09
environment: both
labels: [windows, platform, ipc, daemon]
depends_on: []
---

# TASK-006 — Windows portu (named pipe + Win32 inspector)

## Amaç

Windows'taki LLM ajanı da aynı sözleşmeyle build izleyebilmeli:
`watch --detach` → `status` → `wait` → `kill`/`shutdown`, aynı JSON
şeması, aynı exit-code haritası. Kullanıcı = LLM ajanı; TUI/insan-gözü yok.

## İlke kilidi (decisions.md:9 koşulu)

Windows `decisions.md`'de koşulluydu: "yalnızca CI windows job +
test edecek cihazla". Bu makine Windows + cargo 1.98 çalışıyor, CI'a
`windows-latest` ekleniyor → koşul bu task ile kapanır. Gerçekleşme
kaydı `decisions.md`'ye işlenir.

Korunan ilkeler:

- **Sıfır yeni bağımlılık.** Win32 erişimi ham `extern "system"`
  bloklarıyla (`src/proc/macos.rs` libproc emsali). `Cargo.toml`'a
  crate eklenmez.
- **OS kodu sızmaz.** `daemon.rs` / `health` / `protocol.rs`'ta
  `#[cfg(windows)]` dallanması yok; her şey `platform::` ve `ipc::`
  arkasında. RFC §10 trait sözleşmesi genişler, delinmez.
- **Wire format değişmez.** `protocol.rs`'a dokunulmaz; named pipe
  üzerinden aynı newline-delimited JSON v1.
- **Best-effort, never fatal.** macOS CPU=0.0 emsali: Windows'ta
  ölçülemeyen metrik belgeli eksik olarak 0 döner, daemon asla paniklemez.
- **Güvenlik modeli korunur.** 0600 unix'te aynen; Windows'ta
  `%TEMP%` + pipe ACL varsayılanı belgelenir, symlink/`..` reddi
  platformdan bağımsız hale getirilir.

## Kapsam

- Yapılacaklar: M1–M4 (altta). `exec`, `watch`, `status`, `wait`,
  `kill`, `shutdown`, `cleanup` hepsi Windows'ta çalışır.
- Yapılmayacaklar: TUI/Web, multi-build first-class, Prometheus,
  plugin, WMI-tabanlı tam argv, EventLog-tabanlı OOM (v1 boş döner),
  `journalctl`, Homebrew/Nix, `/proc`-eşdeğer tam network sayımı
  (net_tcp v1'de 0, belgeli).

## Uygulama Planı

### M1 — Windows'ta `cargo build` geçer (iskele)

1. `src/platform/paths.rs` (yeni): `sock_addr(uuid) -> SockAddr`
   (unix: `/tmp/hbmon-<uuid>.sock` PathBuf; windows: pipe adı
   `\\.\pipe\hbmon-<uuid>`), `log_path/pid_path/out_path`
   (windows: `%TEMP%` — `std::env::temp_dir()`), `scan_dir()`.
   Taşınan hardcode'lar: `daemon.rs:54-57,60`, `exec.rs:28-29`,
   `cli/mod.rs:68,89`, `watch.rs` doc satırları.
2. `src/platform/perm.rs` (yeni): `secure_open(path) -> OpenOptions`
   (unix: `.mode(0o600)`; windows: düz) + `secure_fix(path)` (unix:
   `set_permissions(0o600)`; windows: no-op). Kullananlar:
   `pidfile.rs`, `eventlog/mod.rs:27-33`, `daemon.rs:169-175`.
   `..`/symlink reddi (`eventlog:22-26,98-104`, `pidfile:6-10`)
   platform-bağımsız helper'a taşınır (Windows'ta da geçerli).
3. `src/platform/signal.rs` (yeni): `enum Sig { Term, Kill, Int, Hup }`.
   `daemon/signals.rs::parse_signal` buraya taşınır, `i32` yerine
   `Sig` döner. `install_daemon_posture()` → `platform::daemon_posture()`
   (unix: mevcut SIGHUP/SIGPIPE ignore; windows: no-op).
   `daemon.rs:kill_pgroup` → `platform::kill_pgroup(pid, Sig)`
   (unix: `kill(-pid)` aynen; windows: Job-Object terminate —
   M3'te içi dolar, M1'de stub).
4. `util/time.rs:21-38`: `gmtime_r` → cfg: unix `gmtime_r`,
   windows `gmtime_s` (imza farkına dikkat) veya küçük saf-Rust
   sivil-takvim hesabı. Testler (`epoch_zero`, `known_date`) aynen.
5. Derleme gate'leri: `proc/linux.rs` + `metrics/mem.rs` +
   `metrics/{io,fds,net}.rs` arkasına `#[cfg(unix)]`
   (`proc/mod.rs`, `metrics/mod.rs`'ta). Not: `io/fds/net`
   std-only olduğu için bugün derlenir ama ölü koddur (/proc yok);
   gate ile Windows binary'sine girmez. `cgroup.rs` std-only +
   zaten `None` döner → dokunulmaz. `oom.rs` derlenir (std-only);
   `check()` içine `#[cfg(windows)]` erken-`vec![]` dönüşü + gerekçe.
6. Milestone: `cargo build` Windows'ta yeşil (M2/M3 içleri stub olabilir).

### M2 — IPC transport soyutlaması (UDS → named pipe)

7. `ipc/codec.rs`: `BufReader<UnixStream>` / `&mut UnixStream`
   imzaları → `impl BufRead` / `impl Write` generiği. Testler
   `UnixStream::pair()` yerine `std::io::Cursor`/in-memory pipe ile
   (platform-bağımsız hale gelir).
8. `ipc/uds.rs` → `ipc/transport/mod.rs` + `unix.rs` (mevcut kod taşınır,
   imza korunur) + `windows.rs` (yeni, ~150 satır ham FFI):
   `CreateNamedPipeW` (PIPE_TYPE_MESSAGE, tek instance/uuid başına
   isim), `ConnectNamedPipe` + `CreateFileW` client, senkron
   request/response; timeout `SetNamedPipeHandleState`/`WaitForSingleObject`
   veya `SetCommTimeouts` yerine read-timeout emülasyonu (basit:
   blocking + client tarafı timeout; v1 yeter).
   Ortak imza: `serve(&SockAddr, handler)`, `send_request(&SockAddr,
   &Value, secs) -> Value`, + `can_connect(&SockAddr) -> bool`.
9. `cli/mod.rs:109` (cleanup) ve `daemon.rs:100` (double-spawn guard)
   `UnixStream::connect` → `ipc::can_connect`. `resolve_sock`
   `scan_dir()` + `hbmon-*.sock` yerine Windows'ta pipe listesi
   (`\\.\pipe\hbmon-*` enumerate; yoksa `$HBMON_SOCK`/pipe adı zorunlu
   — fallback belgesi).
10. Milestone: `exec` + `status` Windows'ta çalışır (foreground yol +
    IPC roundtrip). `protocol.rs` değişmediğinin kanıtı: aynı test.

### M3 — Windows inspector + detach lifecycle

11. `src/proc/windows.rs` (yeni, ham `extern "system"`, macos.rs
    stub-patterniyle):
    - `list_children`: `CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS)`
      + `Process32FirstW/NextW` (th32ParentProcessID zinciri).
    - `metrics.rss_mb`: `OpenProcess` + `GetProcessMemoryInfo`
      (`WorkingSetSize`).
    - CPU: `GetProcessTimes` (user+kernel FILETIME) → centisecond'a
      böl (`/100_000`), mevcut `CpuTracker` aynen kullanılır
      (100Hz varsayımı korunur; `daemon.rs:288-296`'daki
      `#[cfg(target_os = "linux")]` bloğu `#[cfg(any(linux, windows))]`
      + `cpu_time_centis(pid)` platform helper'ına genellenir).
    - `cmdline`: `QueryFullProcessImageNameW` (best-effort exe yolu —
      macOS "path, not argv" emsali, belgelenir).
    - `fds_open`: `GetProcessHandleCount` (best-effort).
    - `net_tcp/net_udp`: 0 (belgeli eksik; RFC §10'a satır eklenir).
    - `is_alive`: `OpenProcess` + `GetExitCodeProcess != STILL_ACTIVE`.
    - Handle sızıntısı yok: her `OpenProcess`/`Snapshot`'a `CloseHandle`
      (review checklist maddesi).
12. `platform/mod.rs`: windows kolu → `WindowsInspector`.
13. Detach (`daemon.rs:97-121,123-156`): `daemonize()` →
    `platform::detach()`; unix aynen. Windows: `DETACHED_PROCESS |
    CREATE_NEW_PROCESS_GROUP` + stdio NUL + `current_exe` self re-spawn
    (gizli `--watch-child` iç altkomutu? veya doğrudan child Command —
    handshake-sırası korunur: bas → flush → 50ms → spawn → exit 0).
    Child process-group: Job Object (`CreateJobObject` +
    `AssignProcessToJobObject` + `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`);
    `kill_pgroup`/`shutdown`/watchdog TERM→`GenerateConsoleCtrlEvent(CTRL_BREAK)`
    (grup bayrağı sayesinde), KILL→`TerminateJobObject`. `pre_exec
    setpgid` (`daemon.rs:184-190`) → `platform::child_group(&mut Command)`
    (unix: pre_exec; windows: creation_flags + job atama).
14. Milestone: `watch --detach` → `status` → `wait` → `kill` →
    `shutdown` tam turu bu Windows makinede canlı doğrulanır.

### M4 — Test + CI + docs

15. `tests/integration.rs`: `sleep 3/30/45/60` ve `sh -c` yerine
    platform helper (`fn sleep_cmd(s)`, `fn shell_cmd(script)`:
    unix `sleep`/`sh -c`, windows `powershell -NoProfile -Command
    Start-Sleep` / `cmd /C`). 7 testin tamamı OS-nötr; yeni assert yok.
16. CI (`ci.yml`): matrise `windows-latest` satırı.
17. Docs: RFC §4.3 matrisi (Windows satırı ✅), §10'a
    `WindowsInspector` satırı + net/cmdline best-effort notları,
    §14.3'ten Windows maddesi düşer; README'ye Windows kurulum satırı;
    `decisions.md`'ye koşul-kapanış kaydı; `index.json` platform
    `partial-macos` → `stable` (veya `stable-win-beta`).
18. Doğrulama: bu makinede `cargo build`, `cargo test` (stall testi
    ~30sn+ sürer, sabırlı), canlı 5-komut turu; ubuntu/macos
    regresyonu CI'da. `cargo build --release` boyut notu (TASK-002'ye
    girdi olur).

## Etkilenen Dosyalar

- Yeni: `src/platform/{paths,perm,signal}.rs`, `src/proc/windows.rs`,
  `src/ipc/transport/{mod,unix,windows}.rs`
- Değişen: `src/platform/mod.rs`, `src/daemon/{daemon,signals,pidfile}.rs`,
  `src/ipc/{mod,codec}.rs` (+`uds.rs` taşınır), `src/cli/{mod,kill}.rs`,
  `src/cli/{watch,exec}.rs` (sadece path helper), `src/proc/mod.rs`,
  `src/metrics/mod.rs`, `src/util/time.rs`, `src/health/oom.rs`,
  `tests/integration.rs`, `.github/workflows/ci.yml`, `HBMON-RFC.md`,
  `README.md`, `tasks/{KANBAN,decisions}.md`, `index.json`
- Dokunulmayan: `src/ipc/protocol.rs`, `src/health/{stall,dep_missing,timeout}.rs`,
  `src/eventlog/events.rs`, `Cargo.toml` (bağımlılık yok!)

## Doğrulama

- `cargo build` Windows + `cargo test` (7 integration dahil) bu makinede yeşil
- Canlı tur: `watch --detach -- sleep-komutu` → `status` → `wait` →
  `kill` → `shutdown` hepsi 0
- CI: ubuntu + macos + windows yeşil
- `protocol.rs` diff'i boş (wire-format garantisi)
