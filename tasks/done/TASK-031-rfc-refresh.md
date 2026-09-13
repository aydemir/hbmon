---
id: TASK-031
title: "HBMON-RFC bayatlık temizliği (Node portu + v0.1.1 senkronu)"
status: done
priority: P2
created: 2026-09-13
updated: 2026-09-13
environment: both
labels: [docs, rfc, drift]
depends_on: [TASK-030]
---

# TASK-031 — HBMON-RFC bayatlık temizliği

## Amaç

RFC v0.1 draft'ında kaldı; gerçekleşme tersine döndü:
- Referans `build-mon.sh` → Node portu (`build-mon.mjs`, opencode-plugins TASK-127), `.sh` arşivde
- Windows "v2'ye" yazıyor, oysa TASK-006 ile v0.1.0'da geldi
- `log_tail` `event` filtresi, `exec` ephemeral dosyasızlığı, uuid charset kilidi belgede yok
- Test sayıları, §15.9 TASK eşleşmesi, durum/tarih başlığı eski

## Kapsam

- Yapılacaklar: HBMON-RFC.md'de 11 cerrahi düzeltme (davranış değişikliği yok)
- Yapılmayacaklar: bölüm taşıma, RFC çevirisi, kod değişikliği

## Uygulama Planı

1. Başlık + referans + OS hedefleri
2. Adapter/örnek bölümleri (.sh → .mjs)
3. Protokol ekleri (log_tail event, exec ephemeral, uuid charset)
4. Sayılar + kapanış notları (§14.1, §15.9, son satır)
5. `cargo test` (drift tripwire) — doküman editi kodu değiştirmez

## Etkilenen Dosyalar

- `HBMON-RFC.md`

## Doğrulama

- `cargo test --locked -j2` yeşil (özellikle `tests/drift.rs`)
