---
id: TASK-015
title: "wait --until sinyal normalizasyonu"
status: done
priority: P1
created: 2026-09-13
updated: 2026-09-13
environment: both
labels: [ipc, wait, protocol]
depends_on: [TASK-005]
---

# TASK-015 — wait --until sinyal normalizasyonu

## Amaç

`wait_match` bugün `stall_suspect|stalled` ve `oom_suspect|oom_killed` alyanslarını
kabul ediyor ama bilinmeyen adları sessizce yok sayıyor (`_ => false`).
Ajan yanlış sinyal adıyla sonsuz bekliyor — context ve zaman kaybı.

## Kapsam

- Bilinmeyen `until` adında `INVALID_UNTIL` hatası dön (sessiz yoksayma kalkar)
- Kanonik adlar dokümante edilir: `done,failed,dep_missing,timeout,stall_suspect,oom_suspect`; alyanslar korunur
- `wait --list-signals` veya `--help` metnine sinyal listesi
- Yapılmayacaklar: push/async uyandırma, plugin, protokol v2 (kilit dışı)

## Uygulama Planı

1. `wait_match` + `dispatch(wait)` hata yolu + CLI help metni
2. Unit test: bilinmeyen ad hata verir, boş `until` eski davranış (terminal-only)
3. RFC ilgili satır + README ajan hızlı yolu güncellenir

## Etkilenen Dosyalar

- `src/daemon/daemon.rs` (veya TASK-014 sonrası `handler.rs`)
- `src/cli/wait.rs`
- `HBMON-RFC.md`, `README.md`

## Doğrulama

- `cargo test --locked -j2` yeşil (yeni kilitli test: unknown-until)
- Art arda koşularda yeşil (TASK-012 kuralı)

## Gerçekleşme Notu (2026-09-13)

- `protocol.rs`: `WAIT_SIGNALS` + `WAIT_ALIASES` + `validate_until` (tek sözlük, CLI+daemon ortak).
- Daemon `dispatch(wait)`: bilinmeyen ad → `INVALID_UNTIL` (`ok:false`).
- CLI: istekten önce hızlı-doğrulama (exit 3, stderr'de geçerli liste) + `--help` sinyal sözlüğü.
- Test: 2 unit (validate accept/reject) + 1 unit (alyas eşleşme) + 1 canlı integration (`bogus` → exit 3).
- Canlı doğrulama: ham UDS `until:["bogus"]` → `INVALID_UNTIL`; tam süit 47 unit + 9 integration yeşil.
- RFC `until` paragrafı kanonik liste + alyas + INVALID_UNTIL ile güncellendi.
