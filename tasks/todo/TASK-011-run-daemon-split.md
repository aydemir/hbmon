---
id: TASK-011
title: "run_daemon bolme (400 satir hotspot + Shared mutex corbasi)"
status: todo
priority: P3
created: 2026-09-09
updated: 2026-09-09
environment: both
labels: [daemon, refactor, readability]
depends_on: [TASK-010]
---

# TASK-011 — run_daemon bolme (400 satir hotspot + Shared mutex corbasi)

## Amaç

`run_daemon` (~400 satir: setup + spawn + 500ms loop + exit mapping) dosyanin
merge-catisma surtunme noktasidir. Davranis korunarak okunabilirligi yukseltmek.

## Kapsam

- Loop govdesi `poll_once(&mut PollState)`'e cikar (tick: tree metrics,
  stall/OOM/dep/timeout, eventlog); setup/spawn/exit mapping yerinde kalir
- `Shared`'daki 15 ayri `Mutex` icin tek `Mutex<MonitorState>` degerlendirilir;
  contended degilse birlestir, contended ise gerekcesiyle birak
- `dispatch` (~130 satir) op basina zaten ayri; dokunma

- Yapilmayacaklar: davranis degisikligi, yeni metrik, IPC sema degisikligi

## Uygulama Planı

1. `PollState` struct + `poll_once` cikarma (kucuk adimlar, her adimda test)
2. `cargo fmt --check` + `cargo clippy --locked --all-targets -- -D warnings`
3. `cargo test --locked -j2` + CI (44 unit + 8 integration yesil kalmali)

## Etkilenen Dosyalar

- `src/daemon/daemon.rs`

## Doğrulama

- `cargo test --locked -j2` yesil + CI yesil
- `run_daemon` <150 satir, loop govdesi ayri fonksiyon
