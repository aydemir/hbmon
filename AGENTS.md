# AGENTS.md — hbmon ajan talimatı

Kod yazmadan önce `tasks/KANBAN.md:7`'deki **hedef kilidini** ve
`tasks/decisions.md`'yi oku; dış öneri kilide çarparsa elenir (TUI/Web,
plugin sistemi, Prometheus/ML-ETA kapsam dışı). Normatif sözleşme
`HBMON-RFC.md` (TR; `HBMON-RFC-EN.md` aynası), tek sayfalık ajan özeti
`PROTOCOL.md`, hızlı yol `README.md#for-agents`'tedir. RFC ile kod
çelişirse koda güven, farkı drift say (`tests/drift.rs`).

## Kalite kapıları (her Rust düzenlemesinden sonra, sırayla)

```bash
cargo fmt
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked -j2
```

- Sıfır uyarı + sıfır format farkı olmadan bırakma (self-correct).
- CI aynı üçünü `fmt --check` ile koşar + `cargo build --locked` +
  `cargo test -- --ignored` listesi; matrix 4 OS
  (`ubuntu-latest`, `ubuntu-24.04-arm`, `macos-latest`, `windows-latest`).
  `size` job'u uyarı amaçlıdır (<5 MiB hedef, sapmada önce README satırı güncellenir).
- Tek test: `cargo test --locked <ad>`; ortam-bağımlı testler `#[ignore]`
  (proot'ta çalışmaz, gerçek çekirdekte koşar).

## Mimari (tek binary, sıfır runtime bağımlılık)

- `src/cli/` komutlar (`watch status wait kill shutdown log list exec`),
  `src/daemon/` yaşam döngüsü, `src/ipc/` protokol + `transport/unix.rs` |
  `transport/windows.rs` (named pipe, current-user DACL), `src/proc/` +
  `src/metrics/` + `src/health/` (stall/dep-missing/oom/timeout) platform
  başına ayrık: Linux full `/proc`, macOS CPU best-effort + libproc,
  Windows Toolhelp + iphlpapi (UDP yok, cmdline = exe path).
- Detach: Unix setsid + double-fork, Windows `CreateProcessW` +
  `bInheritHandles=FALSE` + Job Object (null-stdio `std::Command` pipe
  devralır — ham API şart).
- Yeni bağımlılık hedef kilidini deler: `tests/drift.rs`
  (`IPC_OPS`, dep allow-list) kızarırsa kasıtlı eklemeyi `decisions.md`'ye yaz.

## Kendi aracıyla doğrulama (gotchas)

- `watch --detach -- <cmd>` → stdout **1. satır** handshake
  `{v,ev:"ready",uuid,sock,log}`; `sock`'u sakla. `exec` ephemeral'dir:
  handshake'teki `sock`/`log` rezerve isimdir, dosya oluşmaz,
  `status`/`wait` yoktur.
- Keşif sırası: `--sock > $HBMON_SOCK > /tmp/hbmon-*.sock` (en yeni);
  `hbmon list` salt-okunur (`--state`, `--live-only`).
- Log'u `cat`leme: `hbmon log --sock $SOCK --tail N` / `--event metric`.
  `status --compact` ucuz yoklamadır.
- Exit: `0 done / 1 failed / 2 dep-missing / 124 timeout / 137 oom /
  3 internal`. `2`'de paketi kur + retry; `wait --until` kanonik isimler
  `done failed dep_missing timeout stall_suspect oom_suspect`
  (alias `stalled oom_killed`; bilinmeyen → `INVALID_UNTIL`, exit 3).
- `stall/oom_suspect` heuristiktir: `status --compact` + `log --event`
  ile doğrula, sonra `kill`/retry kararı ver.
- `status` connect hatası = stale sock → tekrar `watch` et, eski sock'u
  kullanma. `cleanup` (`--older-than`, varsayılan 86400s) canlı daemon
  dosyalarına dokunmaz.
- `Cargo.toml` `exclude` iç süreç dosyalarını registry'den çıkarır
  (tasks/, docs/, .github/); `HBMON-RFC.md` kalır.

## Task disiplini

- Yeni iş: `tasks/_template.md` şablonuyla `tasks/done/TASK-0XX-kisa-ad.md`
  (şu an `todo/` yok, hepsi `done/`), `tasks/KANBAN.md` satırı +
  `tasks/index.json` girdisiyle birlikte; frontmatter `status:` güncel tutulur.
- ID'ler benzersiz olmalı (TASK-006 çakışması örneği: çift başlık yasaktır).
- `0.x` donmuş yüzey minor bumpsız kırılmaz: handshake JSON, exit eşlemesi,
  `wait --until` isimleri, `status --compact` alanları, IPC op'ları.

## Test kuralları

- Integration `uuid()` pid+nanos üretir (TASK-012); kısaltma/sadeleştirme yapma.
- Yeni davranış = yeni/kilitli test; `cargo test` art arda koşularda yeşil kalmalı.

## Commit

- Sadece açık istekle commit/push yap. Mesaj dili: `TASK-0XX done: <kısa>` veya `fix(kapsam): <kısa>`.
