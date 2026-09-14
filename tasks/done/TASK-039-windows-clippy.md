---
id: TASK-039
title: "Windows clippy sıfırlama (Rust 1.98 lintleri)"
status: done
priority: P1
created: 2026-09-14
updated: 2026-09-14
environment: windows
labels: [windows, clippy, hygiene]
depends_on: [TASK-006]
---

# TASK-039 — Windows clippy sıfırlama (Rust 1.98 lintleri)

## Amaç

Windows otomatı miras denetimi: `cargo clippy --locked --all-targets -- -D warnings`
Windows'ta 4 hatayla kırmızı. Linux CI bu dosyaları derlemediği için görmedi.
Sıfır-uyarı kapısını Windows'ta da kapatmak.

## Kapsam

- `src/ipc/transport/windows.rs`: 3x `clippy::io_other_error`
  (`io::Error::new(ErrorKind::Other, ...)` → `io::Error::other(...)`)
- `src/proc/windows.rs`: `clippy::new_without_default` — `WindowsInspector`
  için `Default` impl (inner + stub, Linux CI stub'ı derler)
- Yapılmayacaklar: davranış değişikliği yok, wire format yok, RFC yok.

## Uygulama Planı

1. 3x `Error::other` dönüşümü
2. 2x `Default` impl (inner + stub)
3. `cargo fmt` + `cargo clippy --locked --all-targets -- -D warnings` + `cargo test --locked -j2`

## Etkilenen Dosyalar

- `src/ipc/transport/windows.rs`
- `src/proc/windows.rs`

## Doğrulama

- `cargo fmt --check` yeşil
- `cargo clippy --locked --all-targets -- -D warnings` yeşil
- `cargo test --locked -j2` yeşil
