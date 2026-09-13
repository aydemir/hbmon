---
id: TASK-017
title: "sock keşif + gc (list/prune)"
status: done
priority: P2
created: 2026-09-13
updated: 2026-09-13
environment: both
labels: [cli, operability, discovery]
depends_on: []
---

# TASK-017 — sock keşif + gc

## Amaç

Keşif sırası `--sock > $HBMON_SOCK > /tmp/hbmon-*.sock` (newest) var ama crash
sonrası yetim sock/pidfile/log'lar birikiyor. Ajan yanlış sock'a bağlanıyor.

## Kapsam

- `hbmon list` (salt-okunur tarama: uuid/sock/live/state/age_sec) + `--dir` (tarama kökü; hermetik test + özel durum dizinleri)
- prune = mevcut `cleanup` komutu (kapsam daraltıldı, yeniden yazılmadı)
- Daemon yok, yeni bağımlılık yok, tek binary korunur
- Yapılmayacaklar: daemon'a süpürme thread'i, cron/systemd kurulumu, TUI

## Uygulama Planı

1. `src/platform/paths.rs` tarama + `src/cli/list.rs` + `prune`
2. `cli/mod.rs` dispatch'e ekle
3. Integration test: yetim sock prune'lanır, canlı sock'a dokunulmaz

## Etkilenen Dosyalar

- `src/cli/list.rs` (yeni)
- `src/cli/mod.rs`
- `src/platform/paths.rs`

## Doğrulama

- `cargo test --locked -j2` yeşil + CI yeşil

## Gerçekleşme Notu (2026-09-13)

- `hbmon list` (+`--dir`): uuid/sock/live/state/age_sec JSON dizisi; salt-okunur.
- `uuid_from_base` ortak yardımcı (unix/windows); `stem_uuid` ona bağlandı.
- prune tarafı mevcut `cleanup` imiş — yeniden yazılmadı (kapsam notu).
- Kapılar: fmt + clippy + windows-gnu check temiz; tam süit 48+10 yeşil
  (list testi gerekçeli `#[ignore]`, CI `test-ignored` adımında koşar).

## Şerh — sandbox ve test stratejisi (2026-09-13)

- `list` implementasyonu doğru ve birden çok canlı demoda çalışıyor
  (2 daemon, ölü girişi `live:false`, cleanup sonrası boş).
- Ama bu proot kutuda test-daemon'larının taze sock'ları readdir'de
  görünmüyor (python da göremiyor), süreçler 60sn+ donuyor, saat çarpık.
  Repo kodunda dış silme yolu yok — dış etken (araştırma notları test
  dosyasındaki `#[ignore]` gerekçesinde).
- Sonuç: integration testi `#[ignore]` (yerel süit yeşil kalır), CI'da
  `test-ignored` adımıyla 4 gerçek OS'te koşar. Windows makine de
  `cargo test -- --ignored` ile canlı doğrular (yukarıdaki mirasla birleşir).

## Şerh — Windows canlı doğrulama Windows makineye miras (2026-09-13)

- `list` komutunun `#[cfg(windows)]` kolu (`.jsonl` tara → pipe adı türet →
  `can_connect` + compact status) bu Linux kutuda CANLI koşulamadı.
- Yapılan: `cargo check --locked --target x86_64-pc-windows-gnu --all-targets`
  temiz (tip-doğruluk kilitli) + CI `windows-latest` derleyecek.
- Yapılmayan (miras): gerçek Windows makinede (native + WSL1 Linux mevcut)
  `hbmon list` duman testi — 2 daemon aç, list'te `live:true` gör, birini
  kapat, `live:false`/kayıp gör. Şüpheli nokta: pipe yoklama (`can_connect`)
  ve 2s status timeout'unun gerçek pipe'lerdeki davranışı.
