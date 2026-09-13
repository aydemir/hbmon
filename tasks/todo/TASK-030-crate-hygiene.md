---
id: TASK-030
title: "Crate hijyeni + EN vitrin (v0.1.1)"
status: todo
priority: P2
created: 2026-09-13
updated: 2026-09-13
environment: both
labels: [release, packaging, docs]
depends_on: [TASK-002]
---

# TASK-030 — Crate hijyeni + EN vitrin (v0.1.1)

## Amaç

- crates.io paketi `README.md`'yi vitrin yapar; v0.1.0 TR vitrinle çıktı.
  EN README v0.1.1 ile vitrine girsin.
- `AGENTS.md` (iç ajan talimatı) + `tasks/` yayınlanan `.crate`'e sızıyor;
  iç süreç dokümanı registry'de olmamalı.

## Kapsam

- Yapılacaklar:
  - `Cargo.toml`: `exclude = [AGENTS.md, tasks/, .github/, .codegraph/, docs/, index.json]`
    (`HBMON-RFC.md` kalır — README ona link veriyor)
  - version `0.1.1`, `cargo publish --dry-run` + `cargo package --list` ile sızıntı kontrolü
  - `cargo publish` + `git tag v0.1.1` + push
- Yapılmayacaklar: kod değişikliği, RFC çevirisi

## Uygulama Planı

1. `Cargo.toml` düzenle (exclude + bump)
2. Paket listesi + dry-run doğrula
3. Commit + push + publish + tag + push tag

## Etkilenen Dosyalar

- `Cargo.toml`, `Cargo.lock`

## Doğrulama

- `cargo package --list` çıktısında AGENTS.md/tasks yok, README.md var
- crates.io'da hbmon 0.1.1 + EN açıklama
