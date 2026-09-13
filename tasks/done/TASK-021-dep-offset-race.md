---
id: TASK-021
title: "dep-offset yarışı (Poll::new spawn sonrası örnekliyor)"
status: done
priority: P1
created: 2026-09-13
updated: 2026-09-13
environment: both
labels: [daemon, dep-missing, flake, race]
depends_on: [TASK-009]
---

# TASK-021 — dep-offset yarışı

## Amaç

`wait_until_dep_missing_returns_early` paralelde aralıklı patlıyor
(TASK-011'de 2/5 olarak kayıtlı tarihsel flake; TASK-016 üçlüsünde
tekrar yakalandı: 30.8s + `woke_on: done`).

Kök neden: `Poll::new`, stale-prefix guard için `.out` uzunluğunu
`cmd.spawn()`'dan SONRA örnekliyor. Child (`sh -c 'echo dep...; ...'`)
çoğunlukla yarışı kaybeder (offset=0, dep bulunur) ama paralel yükte
daemon gecikirse echo önce yazılır → dep satırı "stale" sayılıp hiç
taranmaz. Deterministik repro: `.out`'a dep satırı önceden yaz + watch
→ `woke_on: done` (dep_missing beklenirken).

## Kapsam

- Offset, `cmd.spawn()`'dan ÖNCE örneklenir (`Poll::new` imzası
  `out: &Path` → `dep_offset: u64`); niyet (stale guard) korunur,
  örnekleme noktası düzelir
- Yapılmayacaklar: tarama mantığı değişikliği, yeni pattern, IPC değişikliği

## Uygulama Planı

1. `run_daemon`: out open sonrası / spawn öncesi `dep_offset` yakala
2. Deterministik repro artık `dep_missing` dönmeli
3. `cargo fmt` + clippy + `cargo test --locked -j2` (üçlü tekrarlı yeşil)

## Etkilenen Dosyalar

- `src/daemon/daemon.rs`

## Doğrulama

- Stale guard korunur: spawn öncesi yazılan satır hâlâ atlanır (`done`)
- Child'ın spawn sonrası yazdığı dep satırı artık kaçırılamaz (sınır spawn öncesi)
- Dörtlü art arda 3× yeşil + tam süit yeşil

## Gerçekleşme Notu (2026-09-13)

- Düzeltme doğru ama repro yorumu düzeltildi: spawn öncesi yazılan satır
  tanım gereği stale'dir (korunur, `done` doğru). Asıl yarış (child echo,
  spawn ile örnekleme arası) program sırasıyla kapandı: örnekleme artık
  spawn'dan önce, aynı thread'de — child baytı sınırı geçemez.
- `Poll::new(timeout, out, cg)` → `Poll::new(timeout, dep_offset, cg)`;
  tek çağrı noktası. `cargo fmt` + clippy temiz.
- Kanıt: dörtlü (compact+watch_without+dep+unknown) 3× ~1s yeşil
  (erken-dönüş, 30s stall-out yok) + tam süit 47 unit + 10 integration yeşil.
