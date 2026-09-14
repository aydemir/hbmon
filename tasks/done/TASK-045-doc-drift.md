---
id: TASK-045
title: "Milestone-dili doc drift temizliği (M1-stub yorumları)"
status: done

## Kayıt (2026-09-14)

- 5 yorum düzeltildi (`detach.rs`, `signal.rs` x2, `cli/mod.rs`,
  `winffi.rs`). `winffi.rs` "bInheritHandles=FALSE zorunlu" notu
  korundu (tarihçe değil, aktif uyarı). Kod değişikliği yok.
priority: P3
created: 2026-09-14
updated: 2026-09-14
environment: both
labels: [docs, hygiene]
depends_on: [TASK-006]
---

# TASK-045 — Milestone-dili doc drift temizliği (M1-stub yorumları)

## Amaç

Gerçekleşmiş milestone'ların "stub" dili kodda kalmış, okuyanı
yanıltıyor (hepsi implement edildi):

- `src/platform/detach.rs:6-7` "M1'de stub"
- `src/platform/signal.rs:6-7` "(M1 stub: no-op)"
- `src/platform/signal.rs:63` "RFC'ye M4'te not düşülür" (düşüldü)
- `src/cli/mod.rs:80` "windows M2'de pipe enumerate" (yapıldı)
- `src/platform/winffi.rs:4-5` "M2:/M3:" dili

## Kapsam

- Yalnızca yorum satırı düzeltmesi; kod değişikliği yok.
- Yapılmayacaklar: davranış değişikliği.

## Etkilenen Dosyalar

- `src/platform/{detach,signal,winffi}.rs`, `src/cli/mod.rs`

## Doğrulama

- `cargo fmt/clippy/test` etkilenmez (yorum), fmt yeşil
