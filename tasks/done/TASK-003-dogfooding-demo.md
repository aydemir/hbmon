---
id: TASK-003
title: "Dogfooding + demo"
status: done
priority: P2
created: 2026-09-09
updated: 2026-09-09
environment: both
labels: [demo, adoption]
depends_on: []
---

# TASK-003 — Dogfooding + demo

## Amaç

Gerçek büyük derlemede stall/OOM avını kanıtlayan kayıt + adoption malzemesi.

## Kapsam

- Uzun bir derlemeyi hbmon ile izleyip stall/metrik olaylarını kaydetme
- Kısa demo senaryosu (README veya ayrı örnek)
- Yapılmayacaklar: video prodüksiyonu

## Uygulama Planı

1. Hedef derleme seç (örn. büyük bir crate)
2. watch → status → wait turu + JSONL arşivi
3. Sonucu docs/examples altına ekle
