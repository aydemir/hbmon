---
id: TASK-022
title: "cleanup canlı-daemon koruması"
status: done
priority: P1
created: 2026-09-13
updated: 2026-09-13
environment: both
labels: [cli, operability, gc, correctness]
depends_on: []
---

# TASK-022 — cleanup canlı-daemon koruması

## Amaç

`run_cleanup` yalnızca `.sock` için canlılık bakıyor (`can_connect`);
canlı daemon'un `.jsonl`/`.out`/`.pid`'i salt yaşa bakılarak silinebiliyor.
Özellikle SESSİZ uzun build'lerde (forensics'in en lazım olduğu an) olay
günlüğü kalıcı kaybolur. Gerçek risk, kodda doğrulandı.

## Kapsam

- uuid-stem'e göre grupla: sock'u canlıysa (`can_connect`) kardeş
  `.jsonl`/`.out`/`.pid` dosyalara dokunma; ölü artıkları yaşa göre temizle
- Yapılmayacaklar: gc politikası değişikliği (yaş eşiği aynı), daemon'a
  süpürme thread'i

## Uygulama Planı

1. `run_cleanup`: önce canlı sock seti, sonra kardeş koruması (+`--dir` bayrağı,
   `list --dir` ile aynı kök — hermetik test için şart)
2. Saf karar `sweep_decision` + 3 deterministik unit test (her yerde yeşil)
3. Hermetik integration test (izole dizin, `--older-than 0`): canlı korunur,
   ölü süsler süpürülür — proot-sandbox'ta `#[ignore]` (gerekçe test başında),
   CI + Windows makinede koşar
4. `cargo fmt` + clippy + `cargo test --locked -j2`

## Etkilenen Dosyalar

- `src/cli/mod.rs`
- `tests/integration.rs`

## Doğrulama

- Yeni kilitli test yeşil + tam süit yeşil

## Gerçekleşme Notu (2026-09-13)

- Koruma: canlı sock seti → kardeş dosyalar yaşa bakılmaksızın atlanır;
  `cleanup --dir` eklendi (`list --dir` ile aynı kök).
- Test stratejisi iki katman: `sweep_decision` 3 unit (deterministik) +
  hermetik integration (izole dizin). Integration bu proot kutuda
  `#[ignore]` — `touch`'lanan dosya bile 1sn sonra ENOENT verdi, arada
  hiç süreç yokken (saf FS tutarsızlığı; list testiyle aynı kök neden).
  CI + Windows makinede `-- --ignored` ile koşar.
- Kapılar: fmt + clippy temiz; tam süit 52+3+10 yeşil (2 gerekçeli-ignore).
