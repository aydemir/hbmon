---
id: TASK-009
title: "Artımlı dep-scan (B1 perf kusuru)"
status: done
priority: P1
created: 2026-09-09
updated: 2026-09-09
environment: both
labels: [daemon, perf, dep-missing]
depends_on: []
---

# TASK-009 — Artımlı dep-scan (B1 perf kusuru)

## Amaç

Uzun build + verbose log durumunda daemon tick'i yavaşlatan gerçek perf
kusurunu gidermek: `scan_out_for_dep` her 500ms tick'te `.out` dosyasının
tamamını okuyor (`src/daemon/daemon.rs:566-580`). Kullanıcı = LLM ajanı;
uzun derlemede tick gecikmesi stall/OOM sinyallerini geciktirir.

## Kapsam

- `.out` için offset takibiyle artımlı tail okuma (yalnızca tick'ten beri
  eklenen baytlar taranır; bitmemiş son satır offset ilerletilmeden bir
  sonraki tick'e bırakılır; truncate/rotate'da offset sıfırlanır)
- Yeni baytların tamamı taranır (eski tüm-dosya `take(50)` koruması,
  bayat veri bir daha okunmadığı için gereksizdir)
- Daemon başlangıcında offset dosyanın o anki boyundan başlar (append-açılan
  dosyada eski run artığı yanlış pozitif üretmez)

- Yapılmayacaklar: inotify bağımlılığı, desen seti değişikliği,
  `StallDetector` Vec→VecDeque (trivial, ayrı değil — bu taska dahil
  edilebilirse tek satırlık temizlik olarak alınır, ayrı test yok)

## Uygulama Planı

1. `daemon.rs`: `scan_out_for_dep` → offset'li sürüm (imzaya `&mut u64`
   offset parametresi; caller `dep_found` yanına offset saklar)
2. Unit test: sentetik `.out` dosyasına satır ekle → yalnızca yeni satır
   taranır; truncate sonrası sıfırlanma
3. `cargo test --locked -j2` + CI

## Etkilenen Dosyalar

- `src/daemon/daemon.rs`
- `tests/` (yeni unit test; integration değişikliği yok)

## Doğrulama

- `cargo test --locked -j2` yeşil + CI yeşil
- Büyük sentetik logda (örn. 100MB) tick süresinin dosya boyutundan
  bağımsız kaldığı manuel ölçümle not düşülür
