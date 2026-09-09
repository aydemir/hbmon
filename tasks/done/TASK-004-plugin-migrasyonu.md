---
id: TASK-004
title: "opencode-plugins migrasyonu"
status: done
priority: P2
created: 2026-09-09
updated: 2026-09-09
environment: both
labels: [migration, opencode-plugins]
depends_on: []
---

# TASK-004 — opencode-plugins migrasyonu

## Amaç

`build-mon.sh` + cpu-liveness yerine hbmon: tek binary, harness-agnostic
hedefin kendi ekosisteminde kanıtı (RFC Senaryo 5).

## Kapsam

- opencode-plugins tarafında hbmon tabanlı izleme yolu
- Eski shell yolunun arşivlenmesi
- Yapılmayacaklar: opencode-plugins'a yeni özellik

## Uygulama Planı

1. Mevcut build-mon akışını tespit et
2. hbmon karşılığını yaz + test et
3. Eski yolu arşivle
