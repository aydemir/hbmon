---
id: TASK-051
title: "v0.2.1 release (patch: exit summary + binary + crates.io)"
status: done
priority: P1
created: 2026-09-23
updated: 2026-09-23
environment: both
labels: [release, distribution]
depends_on: [TASK-050]
---

# TASK-051 — v0.2.1 release

## Amaç

TASK-050 (`exit.summary`, eklemeli/non-breaking) sonrası patch sürümü çık:
`0.2.0` → `0.2.1`, GitHub binary release + crates.io publish.

## Kapsam

- Yapılacaklar
  - `Cargo.toml`/`Cargo.lock` 0.2.1, README install satırı, RFC senkron
    satırı + changelog 11. madde (TR+EN), `docs/RELEASE.md` karar güncelleme
  - `cargo publish --dry-run` + testler yeşil
  - commit + push, `v0.2.1` tag + push (release.yml binary basar)
  - `cargo publish` (yerel credentials) + doğrulama
- Yapılmayacaklar
  - Breaking değişiklik, yeni özellik, otomatik publish'e bağlama

## Uygulama Planı

1. Sürüm bump + doküman senkronu
2. `cargo fmt` / clippy / test / dry-run
3. Commit + push, tag + push
4. `cargo publish`, `gh release view` + crates.io doğrulama

## Etkilenen Dosyalar

- `Cargo.toml`, `Cargo.lock`, `README.md`, `README.tr.md`
- `HBMON-RFC.md`, `HBMON-RFC-EN.md`, `docs/RELEASE.md`

## Doğrulama

- `cargo test --locked -j2` yeşil + `cargo publish --dry-run` temiz
- GitHub Release'de v0.2.1 + 4 asset, crates.io'da hbmon 0.2.1
