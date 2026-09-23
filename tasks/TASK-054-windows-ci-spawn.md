---
id: TASK-054
title: "Windows CI spawn borçları (7 entegrasyon testi: daemon never came up)"
status: todo
priority: P1
created: 2026-09-23
updated: 2026-09-23
environment: windows
labels: [windows, tests, ci]
depends_on: []
---

# TASK-054 — Windows CI spawn borçları

## Amaç

CI `build-test (windows-latest)` kırmızı: 7 entegrasyon testi
`daemon never came up / never served handshake` modunda düşüyor.
Windows makinede debug edilip yeşile döndürülecek. (Linux'tan repro
yok — bu görev bilinçli olarak Windows makineye bırakıldı.)

## Kapsam

- Düşen testler (`tests/integration.rs`, CI `test` adımı):
  `events_filter_and_replay`, `events_streams_exit_and_returns_build_code`,
  `exit_event_carries_out_summary`, `kill_terminates_build`,
  `wait_until_terminal_not_listed_returns_state`,
  `watch_status_wait_full_cycle`,
  `watch_without_uuid_handshake_matches_daemon`
- Ortak mod: `wait_for_ready` 15 sn boyunca `status`'a bağlanamıyor
  (named pipe `\\.\pipe\hbmon-...`); 14 test geçiyor.
- Kapsam dışı OLMAYAN not: TASK-050/051/052 diff'i bu yolu etkilemez
  (düşenler arasında handshake testi de var; fail `exit.summary`
  kodundan önce, startup/bind katmanında). Regresyon avı değil,
  ortam/spawn sorunu varsayımıyla başla.
- Hipotezler (sırayla ele):
  1. Yavaş runner + `-j2` paralel pipe çekişmesi → `wait_for_ready`
     15 sn yetmiyor (önce timeout'u büyütüp izle, kalıcı çözüm değil).
  2. Stale pipe kalıntısı (`TASK-012` uuid pid+nanos; hızlı re-run'da
     çakışma).
  3. Detach/`CreateProcessW` + Job Object yolunda CI-spesifik fark
     (TASK-042 graceful TERM yoklamasına bak).
- Yapılmayacaklar: Linux/macOS tarafına dokunma, `uuid()` kısaltma
  (TASK-012 yasağı), bilinçsiz retry/timeout şişirme (kayıt + gerekçe şart).

## Uygulama Planı

1. Windows makinede `cargo test --locked -j2` + `-- --ignored` koş, düşen
   listeyi CI ile karşılaştır (yerel ≠ CI ise ortam notu düş).
2. Bir düşen testte daemon log/pipe durumunu yakala (handshake basıldı mı,
   pipe oluştu mu, `list` ne görüyor).
3. Kök nedene hedefli fix + `decisions.md` girdisi.
4. Kalite kapıları (Win): `cargo fmt`, `clippy -- -D warnings`,
   `cargo test --locked -j2` yeşil; CI windows yeşil.

## Etkilenen Dosyalar

- `tests/integration.rs` (muhtemel), `src/daemon/*`, `src/platform/*`
  (bulguya göre)

## Doğrulama

- CI `build-test (windows-latest)` yeşil (fmt + clippy + test +
  test-ignored)
