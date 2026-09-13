---
id: TASK-020
title: "release unblock hazırlığı (tokensuz adımlar)"
status: done
priority: P1
created: 2026-09-13
updated: 2026-09-13
environment: both
labels: [release, distribution]
depends_on: [TASK-002]
---

# TASK-020 — release unblock hazırlığı

## Amaç

TASK-002 `blocked_on: crates.io token` ile duruyor. Token gelene kadar
tokensuz adımları bitirip yayını tek komuta indirmek.

## Kapsam

- `cargo build --release` + boyut ölçümü + duman testi (`watch/status/wait` happy-path)
- `cargo publish --dry-run` + metadata kontrolü (description/license/repository)
- `release.yml` manuel tetikleme ile kuru prova (tag kesmeden)
- Yapılmayacaklar: gerçek `cargo publish`, gerçek tag, Homebrew/Nix

## Uygulama Planı

1. Release derle + boyut + duman testi, sonucu dosyaya işle
2. `publish --dry-run` çıktısını temizle
3. TASK-002'ye hazırlık notu düş, token gelince tek adım kalır

## Etkilenen Dosyalar

- `Cargo.toml`
- `README.md`
- `tasks/in-progress/TASK-002-release-cratesio.md` (not)

## Doğrulama

- `cargo build --locked --release` + `cargo publish --dry-run` temiz
- Duman testi: watch → status → wait(done) ucuca çalışır

## Gerçekleşme Notu (2026-09-13)

- Release build: 33s (hbmon ile izlendi), ikilik **2.38 MB** (<5MB ✓)
- Duman (release ikilik): watch → compact → wait done/code 0 → bogus exit 3 ✓
- `publish --dry-run --allow-dirty`: 92 dosya, verify temiz (çıplak dry-run
  kirli ağaçta durur — commit politikası gereği `--allow-dirty` ile prova)
- Bulgular TASK-002 dosyasına hazırlık notu olarak işlendi; token sonrası
  tek adım: `cargo publish` + tag.
