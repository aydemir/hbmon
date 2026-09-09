---
id: TASK-001
title: "Custom dep-missing patterns"
status: done
priority: P1
created: 2026-09-09
updated: 2026-09-09
environment: both
labels: [health, dep-missing]
depends_on: []
---

# TASK-001 — Custom dep-missing patterns

## Amaç

Yeni dil/build sistemi kullanan ajan, kendi eksik-bağımlılık desenini
harness'e dokunmadan ekleyebilmeli (dosya-tabanlı = harness-bağımsız).

## Kapsam

- `$XDG_CONFIG_HOME/hbmon/patterns.json` (yoksa `~/.config/hbmon/…`):
  `[{"id","category","regex"}]`; custom desenler built-in'lerden önce bakılır
- Geçersiz regex sessizce atlanır (daemon asla paniklemez)
- RFC'deki `patterns.toml` yerine JSON: yeni crate bağımlılığı yok
  (tek-binary/sıfır-dep ilkesi)

- Yapılmayacaklar: hot-reload, TOML desteği, desen öncelik puanı

## Uygulama Planı

1. `dep_missing.rs`: `load_from(path)` + `custom_path()` + custom-öncelikli `match_line`
2. Unit testler (tempfile ile): geçerli/geçersiz regex, öncelik, eksik dosya
3. `cargo test -j1` + CI

## Etkilenen Dosyalar

- `src/health/dep_missing.rs`

## Doğrulama

- `cargo test -j1` yeşil + CI yeşil
