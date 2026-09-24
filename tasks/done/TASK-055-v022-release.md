---
id: TASK-055
title: "v0.2.2 release (patch + binary + crates.io)"
status: done
priority: P1
created: 2026-09-24
updated: 2026-09-24
environment: both
labels: [release, distribution]
depends_on: [TASK-053, TASK-054]
---

# TASK-055 — v0.2.2 release

## Amaç

TASK-053/054 düzeltmeleri sonrası patch sürümü çık: `0.2.1` → `0.2.2`,
GitHub binary release + crates.io publish. Hepsi eklemeli/non-breaking
(detach linger, UDS helper, `serve_error`) → patch.

## Kapsam

- Yapılacaklar
  - `Cargo.toml`/`Cargo.lock` 0.2.2, README install satırı, RFC senkron
    satırı + changelog 12. madde (TR+EN), `docs/RELEASE.md` karar güncelleme
  - `cargo publish --dry-run` + testler yeşil
  - commit + push, `v0.2.2` tag + push (release.yml binary basar)
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
- GitHub Release'de v0.2.2 + 4 asset, crates.io'da hbmon 0.2.2
- Gerçekleşti (2026-09-24): CI koşusu `35939136470` (master) yeşil,
  release koşusu `35939140933` success → Release v0.2.2 + 8 asset,
  `publish-crate` koşusu `35940098219` success → crates.io 0.2.2
  (API doğrulandı: `max_version: 0.2.2`).
