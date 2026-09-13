---
id: TASK-028
title: "JSON sözleşme kilidi"
status: done
priority: P1
created: 2026-09-13
updated: 2026-09-13
environment: both
labels: [contract, context-economy, tests]
depends_on: [TASK-027]
---

# TASK-028 — JSON sözleşme kilidi

## Amaç

Ajan `status`/`log`/`list` çıktısını parse eder. Sessiz alan silme ajanı
bozar. `tests/drift.rs` sadece op isimlerini kilitler; zorunlu alanları
kilitlemez.

## Kapsam

- Yapılacaklar:
  - Canlı-daemon integration testi: `status` (full+compact), `log_tail`,
    `list` zorunlu alanları assert et
  - Alan silme/yeniden adlandırma = test kızarır
- Yapılmayacaklar (out-of-scope):
  - `schemas/*.json` dosyaları (gereksiz ağırlık; test kilidi yeter)
  - Yeni alan yasağı (ekleme serbest, silme yasak)

## Uygulama Planı

1. `tests/integration.rs`: `contract_keys_stable` testi
2. Kalite kapıları

## Etkilenen Dosyalar

- `tests/integration.rs`

## Doğrulama

- `cargo fmt && cargo clippy --locked --all-targets -- -D warnings && cargo test --locked -j2`
