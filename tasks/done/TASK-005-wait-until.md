---
id: TASK-005
title: "wait --until (erken dönüş, eklentisiz uyandırma)"
status: done
priority: P1
created: 2026-09-09
updated: 2026-09-09
environment: both
labels: [ipc, wait, protocol]
depends_on: []
---

# TASK-005 — wait --until (erken dönüş, eklentisiz uyandırma)

## Amaç

Ajan, ara sinyallerde (stall/dep/OOM) polling yapmadan uyandırılabilmeli;
mekanizma yalnızca bloklanan çağrının dönüşü (process-model), harness
eklentisi yok. Polling yeteneği (`status`/`log_tail`) korunur.

## Kapsam

- UDS `wait`: opsiyonel `until` listesi; ilk eşleşmede `woke_on` ile dön
- `until` yoksa bugünkü davranış (yalnızca terminal state'ler)
- CLI: `--until done,dep_missing,stall_suspect`
- RFC 5.2.2'ye dürüst semantik notu (senkron bekleme, async push değil)

- Yapılmayacaklar: gerçek async push, harness eklentisi

## Uygulama Planı

1. `protocol.rs`: `Request.until`
2. `daemon.rs`: `wait_match` + döngü + unit testler
3. `cli/wait.rs`: `--until` + exit mapping
4. Integration: dep erken-dönüş + stall erken-dönüş
5. RFC notu + `cargo test -j1` + CI

## Etkilenen Dosyalar

- `src/ipc/protocol.rs`, `src/daemon/daemon.rs`, `src/cli/wait.rs`
- `tests/integration.rs`, `HBMON-RFC.md`

## Doğrulama

- `cargo test -j1` yeşil + CI yeşil
