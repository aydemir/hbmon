---
id: TASK-029
title: "log --event + list --state filtresi"
status: done
priority: P2
created: 2026-09-13
updated: 2026-09-13
environment: both
labels: [cli, context-economy, operability]
depends_on: [TASK-028]
---

# TASK-029 — log --event + list --state filtresi

## Amaç

Ajan tüm `.jsonl`'u çekmeden ilgili olayı, tüm liste çıktısını çekmeden
ilgili durumdakileri görsün (context ekonomisi, pull-modele uygun).

## Kapsam

- Yapılacaklar:
  - `log --event <ev>`: server-side filtre, eşleşen son N satır
    (`log_tail` req'e opsiyonel `event`, geriye uyumlu)
  - `list --state <s> --live-only`: client-side filtre
  - Unit + integration kilit testleri
- Yapılmayacaklar (out-of-scope):
  - `log --follow` (push; hedef kilidine aykırı)
  - Regex filtre (tam eşleşme yeter)

## Uygulama Planı

1. `src/eventlog/mod.rs`: `tail_filter(path, n, event)`
2. `src/daemon/handler.rs`: `log_tail` op'unda `event` filtresi
3. `src/cli/log.rs`: `--event`; `src/cli/list.rs`: `--state`, `--live-only`
4. Testler + kalite kapıları

## Etkilenen Dosyalar

- `src/eventlog/mod.rs`, `src/daemon/handler.rs`
- `src/cli/log.rs`, `src/cli/list.rs`
- `tests/integration.rs`

## Doğrulama

- `cargo fmt && cargo clippy --locked --all-targets -- -D warnings && cargo test --locked -j2`
