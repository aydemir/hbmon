---
id: TASK-044
title: "Windows test borcu (paths unit + ignored + boyut notu)"
status: done

## Doğrulama Kaydı (2026-09-14, Windows makine)

- `paths.rs` unit testleri eklendi (uuid_from_artifact_names,
  default_addrs_match_convention, env_value_maps_to_sock,
  explicit_windows_forms_recover_uuid) — yeşil.
- `--ignored` 2 test GEÇTİ (önce `list_shows_live_monitors_by_uuid`
  patladı — kök neden: `--uuid` ile `--sock` gövdesi uyuşmuyordu,
  tüm platformlarda bozuktu; `--log` da izole dizine verilmiyordu.
  İkisi de düzeltildi; `cleanup` testine gerçek guard iddiası eklendi).
- Release boyut (Windows): **2.67 MiB** (2.800.640 bayt) — 5 MiB
  bütçenin altında.
priority: P2
created: 2026-09-14
updated: 2026-09-14
environment: windows
labels: [windows, tests]
depends_on: [TASK-006]
---

# TASK-044 — Windows test borcu (paths unit + ignored + boyut notu)

## Amaç

- `src/platform/paths.rs`: sıfır unit test. Windows kolları
  (`from_explicit_windows`, `stem_uuid`, `from_env`, `default_sock`)
  yalnızca canlı koşuda doğrulanıyor.
- `tests/integration.rs` 2 `#[ignore]` test bu Windows makinede hiç
  koşmadı (`-- --ignored`).
- Windows release ikilik boyutu bilinmiyor (size job ubuntu-only).

## Kapsam

- `paths.rs` altına cfg-gated unit testler (win + unix kolları).
- Bu makinede `cargo test --locked -j2 -- --ignored` yeşil kaydı.
- `cargo build --locked --release` (Windows) boyut notu bu dosyaya.

## Uygulama Planı

1. paths unit testleri
2. `--ignored` koşusu (Windows makine)
3. Release boyut ölçümü + not

## Etkilenen Dosyalar

- `src/platform/paths.rs` (yalnızca test modülü)

## Doğrulama

- `cargo fmt/clippy/test` (Win + WSL1)
