---
id: TASK-002
title: "Release + crates.io yayını"
status: done
priority: P1
created: 2026-09-09
updated: 2026-09-13
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
- `cargo publish --dry-run`, sonra gerçek yayın (kullanıcı: henüz erken — bekliyor;
  2026-09-13: token alındı, `cargo login` ile saklandı — yayın adımı hazır, komut bekleniyor)
- GitHub prebuilt: `release.yml` eklendi (tag `v*` veya manuel → 4 platform
  asset'i + sha256; ilk tag crates.io ile aynı gün kesilecek)
- Yapılmayacaklar: Homebrew/Nix (talep gelirse ayrı task)

## Uygulama Planı

1. Release derle + boyut + duman testi
2. Metadata kontrolü (description, license, repository)
3. Publish + README güncelle

## Etkilenen Dosyalar

- `Cargo.toml`, `README.md`

## Hazırlık Notu (TASK-020, 2026-09-13 — token bekleniyor)

- `cargo build --locked --release`: 33s, ikilik **2.38 MB** (<5MB hedef ✓, README'deki ~2.4MB doğrulandı)
- Duman testi (release ikilik): watch → status --compact → wait(done, code 0) → wait --until bogus (exit 3 INVALID_UNTIL) ✓
- `cargo publish --dry-run --allow-dirty`: 92 dosya paketlendi, verify temiz, upload yapılmadı ✓
- Metadata (description/license/repository) paketlemede sorunsuz
- Token gelince tek adım kalır: `cargo publish` + `v*` tag (release.yml manuel de tetiklenebilir)

## Hazırlık Notu 2 (2026-09-13 — TASK-027/028/029 sonrası)

- `cargo build --locked --release`: ikilik **2.560.848 B (~2.44 MB)** (<5MB hedef ✓)
- `cargo publish --dry-run --allow-dirty`: verify temiz, upload yapılmadı ✓
- Release ikilik duman testi: watch → status --compact → log --event metric → wait done (code 0) ✓
- Kalan: `cargo publish` + `git tag v0.1.0` (kullanıcı komutu bekleniyor)

## Yayın Denemesi (2026-09-13)

- Commit `a1c23be` push'landı, `git tag v0.1.0` push'landı (release.yml 4 platform derlemesi tetiklendi)
- `cargo publish` BAŞARISIZ: crates.io 400 — "A verified email address is required"
  (https://crates.io/settings/profile adresinde e-posta doğrulaması gerekli)
- E-posta doğrulandıktan sonra aynı commit'ten `cargo publish` yeterli (tekrar tag gerekmez)

## Kapanış (2026-09-13)

- `cargo publish` BAŞARILI — hbmon v0.1.0 crates.io'da
- Not: tag `v0.1.0` → `a1c23be`, yayın → `f460817` (yalnızca task notu farkı, kod aynı)
