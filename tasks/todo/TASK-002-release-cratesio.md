---
id: TASK-002
title: "Release + crates.io yayını"
status: todo
priority: P1
created: 2026-09-09
updated: 2026-09-09
environment: both
labels: [release, distribution]
depends_on: []
---

# TASK-002 — Release + crates.io yayını

## Amaç

`cargo install hbmon` adoption kapısı; strip+LTO sonrası gerçek ikilik
boyutunu ölçmek (<5MB hedefi doğrulanacak).

## Kapsam

- `cargo build --release`, boyut ölçümü, README'ye kurulum satırı
- `cargo publish --dry-run`, sonra gerçek yayın
- Yapılmayacaklar: Homebrew/Nix (talep gelirse ayrı task)

## Uygulama Planı

1. Release derle + boyut + duman testi
2. Metadata kontrolü (description, license, repository)
3. Publish + README güncelle

## Etkilenen Dosyalar

- `Cargo.toml`, `README.md`
