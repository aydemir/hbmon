---
id: TASK-012
title: "test uuid cakisma sertlestirme (pid-tekrar-kullanim flake)"
status: todo
priority: P3
created: 2026-09-10
updated: 2026-09-10
environment: both
labels: [tests, flake]
depends_on: []
---

# TASK-012 — test uuid cakisma sertlestirme (pid-tekrar-kullanim flake)

## Amaç

`tests/integration.rs::uuid()` yalnizca proses pid'i kullaniyor. Art arda
kosularda pid tekrar kullanilirsa, onceki kosunun 60s linger daemon'inin
sock'uyla cakisip `MONITOR_ALREADY_EXISTS` / yanlis `woke_on` uretebilir.
TASK-011 dogrulamasinda `wait_until_dep_missing_returns_early` kirli kutuda
2/5 patladi, temiz kutuda 3/3 yesil — uretim kodu suphesiz, test izolasyonu supheli.

## Kapsam

- `uuid()`'ya zaman/nonce ekle (örn. `itest-<pid>-<nanos>-<tag>`), pid-tekrarinda da benzersiz olsun
- Stale sock savunmasi zaten var (`spawn_watch` stale temizler); degismez

- Yapilmayacaklar: uretim kodu degisikligi, timeout degerleriyle oynama

## Uygulama Planı

1. `tests/integration.rs::uuid()` genislet
2. Ust uste 3x `cargo test --locked -j2` tam yesil gozlemle
3. `cargo fmt --check` + CI

## Etkilenen Dosyalar

- `tests/integration.rs`

## Doğrulama

- Art arda 3 tam suitte 8/8 yesil
