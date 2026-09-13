---
id: TASK-027
title: "UUID validasyon + 64-bit"
status: done
priority: P1
created: 2026-09-13
updated: 2026-09-13
environment: both
labels: [security, uuid, contract]
depends_on: []
---

# TASK-027 — UUID validasyon + 64-bit

## Amaç

`--uuid` sock/out/log/pid yolu türetir (`hbmon-<uuid>.sock`). `../`, `/`
giren uuid dosya dışına yazar. Üretim uuid'si 4 byte (32-bit) — paralel
daemon'da çakışma payı dar.

## Kapsam

- Yapılacaklar:
  - `generate_uuid()` 8 byte (16 hex char)
  - `validate_uuid()`: `1..=64` char, `[A-Za-z0-9_-]`, yoksa red
  - `watch --uuid` açık değerde validate (hızlı fail, exit 3)
  - Unit + integration kilit testleri
- Yapılmayacaklar (out-of-scope):
  - El yapımı PRNG (OS entropisinde kalınır, `rand` korunur)
  - `uuid_from_base` gevşetmesi (mevcut dosyaları okumaya devam)

## Uygulama Planı

1. `src/util/uuid.rs`: 8 byte + `validate_uuid` + unit test
2. `src/util/mod.rs`: export
3. `src/cli/watch.rs`: açık uuid'de validate
4. `tests/integration.rs`: `../evil` hızlı red testi
5. Kalite kapıları

## Etkilenen Dosyalar

- `src/util/uuid.rs`, `src/util/mod.rs`, `src/cli/watch.rs`
- `tests/integration.rs`

## Doğrulama

- `cargo fmt && cargo clippy --locked --all-targets -- -D warnings && cargo test --locked -j2`
