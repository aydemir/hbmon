---
id: TASK-025
title: "dep pattern genişletme (yeni ekosistemler)"
status: done
priority: P2
created: 2026-09-13
updated: 2026-09-13
environment: both
labels: [health, dep-missing]
depends_on: [TASK-001]
---

# TASK-025 — dep pattern genişletme

## Amaç

TASK-001 custom-pattern altyapısını kurdu ama yerleşik kapsama girmeyen
ekosistemler var (Go modülleri, pip/apt, dotnet, brew vb.). Ajanın
`exit 2 → kur → dene` döngüsü bu dillerde kör.

## Kapsam

- 5-8 yeni yerleşik desen (aday: `go: command not found`/modül hatası,
  `pip: No matching distribution`, `E: Unable to locate package`,
  `dotnet: command not found`, `brew: command not found`); her desen için
  match + no-match kilitli unit testi
- Yapılmayacaklar: pattern DSL değişikliği, custom pattern semantik değişimi

## Uygulama Planı

1. `src/health/dep_missing.rs` desen + test (desen başına çift test)
2. `cargo fmt` + clippy + `cargo test --locked -j2`

## Etkilenen Dosyalar

- `src/health/dep_missing.rs`

## Doğrulama

- Yeni kilitli testler yeşil + tam süit yeşil

## Gerçekleşme Notu (2026-09-13)

- 6 desen sona eklendi (go_module/go_sum/pip_dist/apt_pkg/nuget_pkg/
  brew_formula) — shell-missing zaten generic `cmd_not_found` ile yakalanıyor,
  o yüzden ekosisteme özgü satırlar seçildi; mevcut eşleşmeler korunur
  (sıralama değişmedi, 9 eski test aynen geçti).
- Desen başına match + no-match (12 test); canlı prova: go/apt satırları
  `exec`'te exit 2 üretti.
- Gözlem (kapsam dışı, dokunulmadı): bundler `Could not find ... in any of
  the sources` satırı `ld_lib` eşleşiyor — kategori hassasiyeti için aday,
  ayrı karar ister.
- Tam süit 67+3+11 yeşil (2 gerekçeli-ignore).
