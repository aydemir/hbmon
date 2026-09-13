---
id: TASK-026
title: "README ajan hızlı yolu güncelleme"
status: done
priority: P3
created: 2026-09-13
updated: 2026-09-13
environment: both
labels: [docs, agents]
depends_on: [TASK-015, TASK-016, TASK-017]
---

# TASK-026 — README ajan hızlı yolu güncelleme

## Amaç

TASK-015/016/017 sonrası README'nin "Ajanlar için" bölümü bayatladı:
kanonik sinyal sözlüğü + INVALID_UNTIL, `status --compact`, `hbmon list`
yok. Ajan eski sözleşmeyle çalışır. (Borç benden.)

## Kapsam

- README hızlı yol: sinyal sözlüğü + compact + list + keşif sırası güncellemesi
- RFC zaten güncel (015/016 işledi); eksik kalan mini not varsa eklenir
- Yapılmayacaklar: davranış değişikliği, yeni bölüm şişirme

## Uygulama Planı

1. README "Ajanlar için" + keşif satırı düzenle
2. Gözden geçir (RFC ile çelişki yok)

## Etkilenen Dosyalar

- `README.md`
- `HBMON-RFC.md` (gerekirse mini)

## Doğrulama

- `cargo fmt --check` etkilenmez; RFC↔README çelişki taraması (göz)

## Gerçekleşme Notu (2026-09-13)

- README hızlı yol: kanonik sinyal + alyas + INVALID_UNTIL, `--compact`,
  `list`, `log --tail` işlendi (2 satır değişti, şişirme yok).
- RFC mini: CLI checklist'ine list/log + yeni bayraklar eklendi.
- Kod değişikliği yok — kapı gerekmez (fmt etkilenmez).
