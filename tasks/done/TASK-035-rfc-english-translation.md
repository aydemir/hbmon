---
id: TASK-035
title: "HBMON-RFC İngilizce çevirisi (HBMON-RFC-EN.md)"
status: done
priority: P2
created: 2026-09-13
updated: 2026-09-13
environment: both
labels: [docs, rfc, i18n]
depends_on: [TASK-031, TASK-032]
---

# TASK-035 — HBMON-RFC İngilizce çevirisi

## Amaç

`README.md` (EN) Türkçe `HBMON-RFC.md`'ye link veriyor (942 satır);
EN okuyan ajan/kullanıcı sözleşmenin tamamına erişemiyor. TASK-031
çeviriyi bilinçli dışarıda bıraktı; kullanıcı kararıyla tam çeviri
ayrı görev olarak açılıyor. TASK-032'deki EN `docs/PROTOCOL.md` özeti
baki kalır (1 sayfa hızlı yol), bu görev normatif belgenin tamamıdır.

## Kapsam

- Yapılacaklar:
  1. `HBMON-RFC-EN.md`: bölüm/alt-bölüm numaraları ve sırası birebir,
     kod blokları/JSON şemaları/op adları/exit kodları aynen korunur.
  2. Terminoloji TASK-032 Stability bölümüyle tutarlı (frozen terimler
     çevrilmez: `dep_missing`, `stall_suspect`, `oom_suspect`, op adları).
  3. `README.md`'deki 3 RFC linki EN belgeye çevrilir;
     `README.tr.md` Türkçe belgede kalır.
- Yapılmayacaklar: içerik değişikliği/yeniden yapılandırma, kod
  değişikliği, TR belgede oynama (numara kayarsa iki belge birlikte
  güncellenir — drift sayılır).

## Uygulama Planı

1. Bölüm bölüm çevir (§1-16 + ekler), her bölümde başlık sayısını TR ile
   karşılaştır.
   - [done] Alt ajan çevirdi; bağımsız doğrulandı: `grep -c '^#'` 73/73,
     `grep -c '```'` 68/68, kod blokları `diff` ile bayt-ayni, frozen
     terim sayımları eşit (`dep_missing` 18/18 vb.).
2. Kod/JSON/komut bloklarını diff ile aynılık kontrolü.
   - [done] `CODE_IDENTICAL`.
3. README.md linklerini `./HBMON-RFC-EN.md`'ye çevir.
   - [done] 3 link EN belgeye; TR aslına parantez içi referans eklendi.
     `README.tr.md` TR belgede.
4. `cargo test` (doküman editi kodu değiştirmez; drift tripwire yeşil).
   - [done] Full süit yeşil (72 + 3 + 14 + 1 smoke).

## Etkilenen Dosyalar

- `HBMON-RFC-EN.md` (yeni)
- `README.md` (link hedefleri)
- `tasks/decisions.md` (EN normatif eşdeğer kaydı, gerekirse)

## Doğrulama

- `cargo test --locked -j2` yeşil
- Başlık sayısı paritesi: `grep -c '^#' HBMON-RFC.md` ==
  `grep -c '^#' HBMON-RFC-EN.md`
- Kod blok sayısı paritesi: `grep -c '```' ...` eşit.
