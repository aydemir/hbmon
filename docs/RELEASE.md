# Release prosedürü (hbmon)

İki artefakt, iki ayrı kapı. İkisi de geri alınamaz işleme çıkar
(tag + crates.io publish), o yüzden otomasyon bilinçli olarak
asgari tutulur (TASK-033).

## 1. GitHub binary release (otomatik)

`v*` tag push'lanınca `.github/workflows/release.yml` 4 hedefte
derler + checksum'lı asset'lerle GitHub Release açar. Değişiklik yok.

## 2. crates.io publish (manuel)

`release.yml` bilerek `cargo publish` içermez. Publish geri alınamaz
(sürüm numarası yeniden kullanılamaz); insan kapısı şart.

```bash
# 0. Sürüm artışı (Cargo.toml + Cargo.lock). Kural:
#    - dondurulmuş yüzeyde breaking yoksa patch (0.1.x)
#    - breaking varsa minor + decisions.md girdisi (README Stability)
cargo publish --dry-run          # paket temiz olmalı
cargo test --locked -j2          # yeşil
git tag v0.1.x && git push origin v0.1.x   # binary release tetiklenir
cargo publish                    # crates.io (token: ~/.cargo/credentials)
```

Alternatif: Actions → `publish-crate` workflow'unu tag adıyla manuel
çalıştır (`CRATES_IO_TOKEN` secret'ı gerekir).

## 3. Sürüm kararları

- `v0.1.1` crates.io'da yayında (2026-09-13 dry-run ile doğrulandı:
  `hbmon@0.1.1 already exists`). Aynı numaraya yeniden publish yok.
- `v0.2.0` yayında (2026-09-18, TASK-047/048 breaking-adjacent düzeltmeler).
- `v0.2.1` yayında (2026-09-23, TASK-050/051: `exit.summary`).
- Sonraki: `v0.2.2` (TASK-053/054/055: detach linger + UDS helper +
  `serve_error`; hepsi eklemeli/non-breaking → patch).
- Token/ownership: maintainer `~/.cargo/credentials`'ta; CI secret'ı
  `CRATES_IO_TOKEN` (yoksa workflow çalıştırılmaz, binary release
  etkilenmez).
