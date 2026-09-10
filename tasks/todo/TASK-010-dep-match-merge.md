---
id: TASK-010
title: "DepMatch birlestirme (daemon/health duplikasyonu)"
status: todo
priority: P3
created: 2026-09-09
updated: 2026-09-09
environment: both
labels: [daemon, health, cleanup]
depends_on: []
---

# TASK-010 — DepMatch birlestirme (daemon/health duplikasyonu)

## Amaç

Ayni 3 alanli struct iki kez tanimli: `health/dep_missing.rs:90` (pub) ve
`daemon.rs:523` (private). Okuma maliyetini dusurmek; davranis degisikligi yok.

## Kapsam

- `scan_out_for_dep` dogrudan `health::dep_missing::DepMatch` donsun;
  daemon'daki private struct silinsin, kullanim noktasi (`dep_found`) yeni tipe tasinsin
- Unit test imzasi guncellenir (`dep_scan_reads_only_new_bytes`)

- Yapilmayacaklar: desen seti degisikligi, `run_daemon` bolme (o TASK-011)

## Uygulama Planı

1. `daemon.rs`: private `DepMatch` sil, `use crate::health::dep_missing::DepMatch`
2. `cargo fmt --check` + `cargo clippy --locked --all-targets -- -D warnings`
3. `cargo test --locked -j2` + CI

## Etkilenen Dosyalar

- `src/daemon/daemon.rs`

## Doğrulama

- `cargo test --locked -j2` yesil + CI yesil
- `grep -n "struct DepMatch" src/` tek sonuc doner
