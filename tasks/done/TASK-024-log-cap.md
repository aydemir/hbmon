---
id: TASK-024
title: "out log cap (uzun build disk güvenliği)"
status: done
priority: P2
created: 2026-09-13
updated: 2026-09-13
environment: both
labels: [daemon, robustness, disk]
depends_on: [TASK-009]
---

# TASK-024 — out log cap

## Amaç

`.out` append ile sınırsız büyür; saatler süren build'lerde disk riski.
Rotasyon yok. `scan_out_for_dep` kesik dosyayı yönetebiliyor
(`len < offset` → reset), yani budama güvenli tarafta.

## Kapsam

- `watch --max-log-mb N` (opt-in; varsayılan sınırsız — davranış korunur):
  tick'te boyut kontrolü, aşınca son N bayt tutulur, dep-offset resetlenir
- Yapılmayacaklar: external logrotate bağımlılığı, sıkıştırma, `.jsonl` budama

## Uygulama Planı

1. `run_daemon`/`poll_once` boyut kontrolü + budama yardımcısı
2. Unit test: budama sonrası dep taraması toparlanır (offset reset)
3. `cargo fmt` + clippy + `cargo test --locked -j2`

## Etkilenen Dosyalar

- `src/daemon/daemon.rs`

## Doğrulama

- Yeni kilitli test yeşil + tam süit yeşil

## Gerçekleşme Notu (2026-09-13)

- `watch --max-log-mb N` (opt-in) → `MonitorConfig::max_log_bytes`;
  tick'te aşımda son yarıyı tut + `secure_fix` (0600 korunur).
- Arazi: eklediğim `dep_offset.min(keep)` satırı scan-reset'i bozuyordu
  (tutulan kuyruğu atlardı) — yazarken yakalandı, kaldırıldı; scan'in
  kendi `len < offset → 0` mantığı yeterli (`dep_found` guard çifte olayı önler).
- Canlı duman: 3MB çıktı + `--max-log-mb 1` → final 512KB (cap/2) ✓.
  (İlk duman bayat ikilikle koşmuş — flag'i tanımamasından yakalandı.)
- 3 unit (suffix/short/recovery) + tam süit 55+3+11 yeşil.
