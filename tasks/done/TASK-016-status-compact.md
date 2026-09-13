---
id: TASK-016
title: "status context ekonomisi (compact/fields)"
status: done
priority: P1
created: 2026-09-13
updated: 2026-09-13
environment: both
labels: [ipc, status, context-economy]
depends_on: []
---

# TASK-016 — status context ekonomisi

## Amaç

`status` her çağrıda full snapshot dönüyor: tree + metrics + health +
5 satır log_tail + last_event. Poll eden ajan her tur aynı vergiyi ödüyor.
Hedef kilidinin çekirdeği context ekonomisidir.

## Kapsam

- `status --compact` (veya `fields=state,code,woke_on,health.stall_score`): varsayılan davranış DEĞİŞMEZ, opt-in küçültme
- `log_tail n` parametresi zaten var; compact'ta `log_tail` atlanır
- Yapılmayacaklar: push, streaming, GraphQL tarzı sorgu dili, şema v2

## Uygulama Planı

1. `protocol.rs` + `dispatch(status)` + `status_map` opt-in filtre
2. CLI `--compact` / `--fields` bayrağı
3. Boyut karşılaştırma testi (compact < full, zorunlu alanlar korunur)

## Etkilenen Dosyalar

- `src/ipc/protocol.rs`
- `src/daemon/daemon.rs`
- `src/cli/status.rs`

## Doğrulama

- `cargo test --locked -j2` yeşil + yeni test: compact payload full'un alt kümesi, `state` her zaman var

## Gerçekleşme Notu (2026-09-13)

- Daemon: `status_compact` (state/uuid/elapsed/health{stall_score,threshold_sec}/last_event/+code);
  tree/metrics-detay/log_tail/root_cmd/cgroup yok, inspector/log okuma yok.
- `dispatch(status)`: `compact:true` opt-in; varsayılan full değişmez.
- CLI `status --compact`, `protocol::Request::compact`, RFC status paragrafı.
- Kilitli canlı test `status_compact_is_subset_and_smaller` (alan + bayt kazancı).
- Yanda bulunan pre-existing yarış TASK-021'e ayrıldı ve düzeltildi;
  tam süit 47 + 10 yeşil.
