---
id: TASK-019
title: "drift kilidi + boyut kapısı (RFC test + CI)"
status: done
priority: P2
created: 2026-09-13
updated: 2026-09-13
environment: both
labels: [docs, deps, rfc, ci]
depends_on: [TASK-008]
---

# TASK-019 — drift kilidi + boyut kapısı

## Amaç

TASK-008 drift'i bir kez temizledi ama kapı yok: `protocol.rs` ↔ RFC
tablosu ve `Cargo.toml` bağımlılıkları yeniden kayabilir. TASK-002'nin
<5MB hedefi de CI'da ölçülmüyor.

## Kapsam

- Kilitli test: IPC op listesi (`status/metrics/wait/kill/log_tail/shutdown`) ↔ RFC tablosu; `Cargo.toml` bağımlılık allow-listi (clap/serde/serde_json/regex/once_cell/libc/rand dışına çıkarsa test kızarır)
- CI job: `cargo build --release` + boyut raporu + 5MB eşiği (uyarı, fail değil — ilk ölçüm)
- Yapılmayacaklar: Homebrew/Nix, crates.io publish (TASK-002'de)

## Uygulama Planı

1. `tests/` kilitli drift testi (protocol op'ları + dep allow-list)
2. `ci.yml`'e release-size job (sadece boyut yazdır + eşik kontrolü)
3. README'ye boyut satırı (ölçülen gerçek değer)

## Etkilenen Dosyalar

- `tests/`
- `.github/workflows/ci.yml`
- `README.md`

## Doğrulama

- `cargo test --locked -j2` yeşil + CI yeşil

## Gerçekleşme Notu (2026-09-13)

- Tek doğruluk kaynağı: `protocol::IPC_OPS` (RFC tablo karşılığı; yeni op =
  burası + dispatch + RFC + test el ele).
- `tests/drift.rs`: op kilidi + typed roundtrip + Cargo.toml allow-list
  (runtime 7 + dev 2; clippy `manual_pattern_char_comparison` uyarısı
  yakalandı, dizi deseniyle düzeltildi).
- CI `size` job: release derle + raporla + 5MB üstünde `::warning::`
  (fail yok); workflow gözle doğrulandı (yaml modülü yok).
- README: `2.38MB (v0.1.0, <5MB hedefi; CI size job'u izler)`.
- Windows-gnu check temiz; tam süit 49+3+10 yeşil (1 gerekçeli-ignore).
