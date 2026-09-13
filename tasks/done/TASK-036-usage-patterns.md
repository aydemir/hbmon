---
id: TASK-036
title: "Tüketici desenleri belgesi (salt CLI / skill / plugin+MCP)"
status: done
priority: P2
created: 2026-09-13
updated: 2026-09-13
environment: both
labels: [docs, agents, harness]
depends_on: [TASK-032]
---

# TASK-036 — Tüketici desenleri belgesi

## Amaç

`hbmon` core'a plugin/MCP/skill alınmaz (hedef kilidi), ama LLM
tüketicisi kendi harness'i için yol seçmek ister: salt CLI, skill paketi,
yoksa plugin+MCP katmanı. Seçimi kolaylaştıran tek belge yok — her
harness kullanıcısı aynı deneyi baştan yapıyor.

## Kapsam

- Yapılacaklar:
  1. Kökte `USAGE-PATTERNS.md` (EN, `PROTOCOL.md` emsali): üç desen —
     salt CLI komut dizisi, SKILL.md iskeleti, plugin+MCP tool haritası
     + karar tablosu (kurulum / context kontrolü / ne zaman).
  2. OpenCode öncel-sanatı referansı: `aydemir/opencode-plugins`
     (`build-mon.mjs` + `hbmon-build-mon.mjs` adapter + settle-noticer)
     plugin+MCP kanadında örnek olarak anılır (vendörlenmez).
  3. `README.md` + `README.tr.md` ajan bölümünden tek satır link.
- Yapılmayacaklar: core'a plugin/MCP/skill kodu, harness eklentisi
  yazımı, TR çeviri (gerekirse ayrı task).

## Uygulama Planı

1. `USAGE-PATTERNS.md` yaz (komutlar README ile tutarlı olmalı).
   - [done] Kökte EN belge: salt CLI dizisi + SKILL.md iskeleti +
     plugin+MCP tool haritası + karar tablosu + opencode-plugins öncel-sanatı.
2. README EN+TR'ye birer satır link.
   - [done]
3. `cargo package --list` ile pakete girdiği doğrula.
   - [done] `USAGE-PATTERNS.md` pakette.

## Etkilenen Dosyalar

- `USAGE-PATTERNS.md` (yeni)
- `README.md`, `README.tr.md` (birer satır)

## Doğrulama

- `cargo package --list --allow-dirty` içinde `USAGE-PATTERNS.md` var
- Komut örnekleri README ajan bölümüyle aynı
