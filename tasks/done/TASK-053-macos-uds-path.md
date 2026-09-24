---
id: TASK-053
title: "macOS ignored sock yolu (UDS 104 bayt) + serve körlüğü"
status: done
priority: P1
created: 2026-09-24
updated: 2026-09-24
environment: macos
labels: [macos, tests, ci, daemon]
depends_on: []
---

# TASK-053 — macOS ignored sock yolu + serve körlüğü

## Amaç

CI `build-test (macos-latest)` kırmızı: 2 ignored test
(`cleanup_protects_live_daemon_files`, `list_shows_live_monitors_by_uuid`)
`wait_for_ready`'de (15 sn, `daemon never came up`) düşüyor. Ana süit
yeşil — borç bu iki testte.

## Kapsam

- Kök neden: iki test de `--sock` için `tempfile::tempdir()` dizini
  verir. macOS'te `tempfile` `$TMPDIR` (`/var/folders/.../T/`, ~60 bayt)
  altını kullanır; sock yolu ~115+ bayta çıkar, macOS UDS sınırı
  (104) aşılır → `bind` düşer. `run_daemon`'da `serve` hatası `let _ =`
  ile yutuluyordu (daemon kör koşar, istemci 15 sn yoklar).
- Yapılacaklar:
  1. `hermetic_dir()` helper (`tests/integration.rs`): unix'te
     `/tmp` altında kısa dizin (`hbmon-t*`), Windows'ta varsayılan
     (named pipe dosya yolu değil). 2 ignored test + unix
     `debug_assert!(sock.len() < 104)` (gelecekte sessiz kırılmasın).
  2. `serve_error` olayı (`src/daemon/daemon.rs`): `serve`/`bind`
     hatası `.jsonl`'a düşer (`error` alanıyla); socket yokken bile
     log dosyası yerinde olduğundan tanı konur. Yaşam döngüsü aynı,
     yeni IPC op yok, ek bağımlılık yok.
  3. RFC §8.3 "Tam Liste" + EN aynasına `serve_error` satırı.
- Yapılmayacaklar: `uuid()` kısaltma (TASK-012 yasağı), sock yolu
  hash'leme/kesme (handshake sözleşmesi), `serve` imza değişikliği,
  Linux/Windows daemon davranışı.

## Uygulama Planı

1. Helper + 2 testte kullanım + debug_assert'ler.
2. `EventLogger` `Arc` ile serve thread'ine taşınır, `Err`'de
   `serve_error` append.
3. RFC TR+EN satırı.
4. Kalite kapıları (Win): `cargo fmt`, `clippy -- -D warnings`,
   `cargo test --locked -j2` + `-- --ignored` yeşil; cross clippy
   (aarch64-apple-darwin, x86_64-unknown-linux-gnu) temiz.
5. Push + CI macos yeşili → `done`.

## Etkilenen Dosyalar

- `tests/integration.rs`, `src/daemon/daemon.rs`
- `HBMON-RFC.md`, `HBMON-RFC-EN.md`

## Doğrulama

- Yerel (Win): full süit 21 passed/2 ignored, `-- --ignored` 2 passed,
  fmt+clippy temiz, cross clippy (mac/linux) temiz.
- CI `build-test (macos-latest)` yeşil (test + test-ignored) —
  koşu `35938529809`: 5/5 job success (macos, windows, 2×ubuntu, size).
