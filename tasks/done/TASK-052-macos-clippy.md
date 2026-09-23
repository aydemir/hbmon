---
id: TASK-052
title: "macOS clippy sıfırlama (cfg artefaktı: unused_mut + dead_code)"
status: done
priority: P1
created: 2026-09-23
updated: 2026-09-23
environment: macos
labels: [macos, clippy, hygiene]
depends_on: []
---

# TASK-052 — macOS clippy sıfırlama

## Amaç

CI `build-test (macos-latest)` kırmızı (`-D warnings`): iki lint hatası,
ikisi de platform cfg artefaktı — gerçek ölü kod değil. v0.2.1 sonrası
master CI'ı yeşile döndür.

## Kapsam

- Hatalar (`src/daemon/daemon.rs`, `poll_once`/`Poll`):
  1. `let mut cpu_by_pid` — `unused_mut` (macOS'ta `cfg(linux|windows)`
     bloğu derlenmez, insert hiç çalışmaz; okuma ortaktır).
  2. `Poll::tracker` — `dead_code` (macOS'ta okunmaz; Linux/Windows'ta
     CPU leg için şart).
- Düzeltme: `cfg_attr(target_os = "macos", allow(...))` — yalnız macOS'ta
  sustur, diğer platformlarda lint koruması sürsün.
- Yapılmayacaklar: davranış değişikliği, `mut`/alanın tamamen kaldırılması
  (Linux/Windows CPU leg'i korunur), geniş `allow(dead_code)`.

## Uygulama Planı

1. İki `cfg_attr` ekle + neden yorumu.
2. `rustup target add x86_64-apple-darwin` ile macOS clippy'yi yerelde
   doğrula (CI spyglass yerine gerçek kanıt).
3. Linux `fmt`/`clippy`/`test` yeşil + commit/push.

## Etkilenen Dosyalar

- `src/daemon/daemon.rs`

## Doğrulama

- `cargo clippy --target x86_64-apple-darwin -- -D warnings` temiz
- `cargo fmt` + linux clippy + `cargo test -j2` yeşil, CI macos yeşil
