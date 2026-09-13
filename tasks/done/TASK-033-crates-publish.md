---
id: TASK-033
title: "crates.io publish prosedürü + release.yml publish"
status: done
priority: P1
created: 2026-09-13
updated: 2026-09-13
environment: both
labels: [release, distribution]
depends_on: [TASK-002, TASK-030]
---

# TASK-033 — crates.io publish prosedürü + release.yml publish

## Amaç

3. göz §5 doğrulandı: `exclude` iyi (`Cargo.toml:10-17`), ama
`release.yml` yalnızca GitHub binary basıyor (`publish` job'u release
asset), `cargo publish` yok; README'de `cargo install hbmon # crates.io,
v0.1.0+` yazıyor ama publish kanıtı (`--dry-run`) yok; ilk tag numarası
(`v0.1.1` vs `v0.2.0`) netleşmemiş.

## Kapsam

- Yapılacaklar:
  1. `cargo publish --dry-run` çıktısını al, hataları kapat.
  2. `release.yml`'ye `cargo publish` job'u ekle VEYA gerekçesiyle manuel
     prosedürü `docs/RELEASE.md`'ye yaz (token/ownership dahil).
  3. README install satırını netleştir (`--git` vs crates.io).
  4. Tag kararını `decisions.md`'ye yaz (`v0.1.2`; `0.1.1` zaten yayında).
  5. PROTOCOL.md paket kararı (kök taşıma).
- Yapılmayacaklar: crates.io'ya gerçek publish (açık istek olmadan),
  token'ı repoya gömme.

## Uygulama Planı

1. `cargo publish --dry-run` koş, çıktıyı göreve ekle.
   - [done] Temiz: 67 dosya, verify+compile OK (`--allow-dirty`; ağaçta
     commitlenmemiş TASK-032 dokümanları vardı).
   - [done] Bulgu: `hbmon@0.1.1 already exists` — yeniden publish yok,
     sonraki `v0.1.2`.
2. `release.yml` veya `docs/RELEASE.md`'yi güncelle.
   - [done] `docs/RELEASE.md` (manuel prosedür) + `publish-crate.yml`
     (manuel tetik, tag↔Cargo.toml eşleşme korumalı) eklendi; otomatik
     release'e bağlanmadı (geri alınamaz işlem).
3. README install bölümünü düzelt.
   - [done] Doğrulandı, değişiklik yok: `cargo install hbmon`, `--git`
     karışıklığı yok.
4. `decisions.md`'ye tag kararı.
   - [done] `v0.1.2` kaydı eklendi.
5. PROTOCOL.md paket kararı.
   - [done] `PROTOCOL.md` köke taşındı (67 dosya pakette doğrulandı);
     `README.md`/`README.tr.md` linkleri `./PROTOCOL.md` oldu.

## Etkilenen Dosyalar

- `.github/workflows/release.yml`
- `.github/workflows/publish-crate.yml` (yeni, manuel tetik)
- `PROTOCOL.md` (kök taşıma: `docs/` → kök)
- `docs/RELEASE.md` (gerekirse yeni)
- `README.md`
- `tasks/decisions.md`

## Doğrulama

- `cargo publish --dry-run` temiz
- `cargo fmt` + `cargo clippy --locked --all-targets -- -D warnings` temiz
