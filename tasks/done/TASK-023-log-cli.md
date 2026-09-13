---
id: TASK-023
title: "hbmon log CLI (log_tail op)"
status: done
priority: P2
created: 2026-09-13
updated: 2026-09-13
environment: both
labels: [cli, context-economy, operability]
depends_on: []
---

# TASK-023 — hbmon log CLI

## Amaç

`log_tail` op'u daemon'da var ama CLI yok; ajan tüm `.jsonl`'u cat'leyip
context yakıyor. Hedef kilidi = context ekonomisi.

## Kapsam

- `hbmon log --sock --tail N` (varsayılan 20): `log_tail` op'una ince CLI,
  salt-okunur, JSON çıktı
- Yapılmayacaklar: full-text search/grep dili, streaming/tail -f
  (ihtiyaç kanıtlanırsa ayrı task)

## Uygulama Planı

1. `src/cli/log.rs` + `cli/mod.rs` dispatch
2. Integration test: olay yazdır + `log --tail` alt-küme doğrula
3. `cargo fmt` + clippy + `cargo test --locked -j2`

## Etkilenen Dosyalar

- `src/cli/log.rs` (yeni)
- `src/cli/mod.rs`

## Doğrulama

- Yeni kilitli test yeşil + tam süit yeşil

## Gerçekleşme Notu (2026-09-13)

- `src/cli/log.rs` (yeni): `log --sock --tail N` → `log_tail` op, JSON çıktı.
- Buluntu: tail HAM SATIR (string) döner, obje değil — test parse ederek
  doğrular (uuid eşleşmesi).
- Kapılar: fmt + clippy + windows-gnu temiz; tam süit 52+3+11 yeşil
  (2 gerekçeli-ignore).
