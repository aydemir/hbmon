---
id: TASK-049
title: "Proje simgesi (GitHub README + crates vitrini)"
status: done
priority: P3
created: 2026-09-19
updated: 2026-09-19
environment: both
labels: [branding, docs, hygiene]
depends_on: []
---

# TASK-049 — Proje simgesi (GitHub README + crates vitrini)

## Amaç

`hbmon`'un GitHub README ve crates.io vitrininde kullanacağı simge:
kullanıcının verdiği CRT + nabız-göz + devre-gülümseme konseptinin
sadeleştirilmiş, küçük boyutta okunur vektör karşılığı.

## Kapsam

- Yapılacaklar: `assets/logo.svg` (ana), `assets/logo.png` (512),
  `assets/logo-128.png` (README önizleme doğrulaması), iki README'de
  header logo bloğu (crates.io uyumu için mutlak URL).
- Yapılmayacaklar: crate içi gömülü ikon, docs.rs CSS, yeni bağımlılık.

## Uygulama Planı

1. SVG el-işçiliği (tek renk yeşil nabız + gri çerçeve, koyu zemin).
2. `cairosvg` ile PNG export (512 + 128), 128px okunurluk kontrolü.
3. README.md / README.tr.md header'a `<img>` bloğu.
4. `Cargo.toml` değişikliği yok — `exclude` listesi `assets/`'i
   kapsamıyor, dosyalar registry paketine otomatik giriyor.

## Etkilenen Dosyalar

- `assets/logo.svg`, `assets/logo.png`, `assets/logo-128.png`
- `README.md`, `README.tr.md`

## Doğrulama

- PNG render gözle doğrulandı (512 + 128).
- `git status` yalnızca beklenen dosyaları gösteriyor.
