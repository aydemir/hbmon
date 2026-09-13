---
id: TASK-034
title: "Gerçek build smoke testi + stale/orphan politika dokümanı"
status: done
priority: P2
created: 2026-09-13
updated: 2026-09-13
environment: both
labels: [tests, operability, docs]
depends_on: [TASK-022]
---

# TASK-034 — Gerçek build smoke testi + stale/orphan politika dokümanı

## Amaç

3. göz §3-4 doğrulandı (daraltılmış): integration testler yalnızca
`sleep`/`echo` (`tests/integration.rs:27-31` `sleep_cmd`, 890 satırda
gerçek toolchain yok); stale dosyalar için `cleanup` + canlı-guard VAR
(`src/cli/mod.rs:85-110`, TASK-022) ama otomatik reclaim/orphan state
yok ve agent'ın stale sock gördüğünde ne yapacağı dokümante değil.
§6'nın "herkese açık socket" iddiası unix için YANLIŞ (`0600`:
`src/platform/perm.rs`, `src/daemon/pidfile.rs:16`,
`src/eventlog/mod.rs:31`); gerçek eksik Windows ACL no-op + ownership/
auth yok — bu görevde yalnızca dokümante edilir, kod yok.

## Kapsam

- Yapılacaklar:
  1. En az bir gerçek toolchain smoke testi: `cargo build --locked`
     (veya `make -j`) `watch` altında, CI'da Linux'ta koşar; flake'e karşı
     timeout geniş, `uuid()` kuralı korunur (TASK-012).
  2. Stale/orphan politika dokümanı (README veya `docs/`): crash sonrası
     `.sock/.pid/.jsonl` durumu, `list` ile tespit, `cleanup --older-than`
     kullanımı, canlı-guard garantisi, Windows notu.
- Yapılmayacaklar: otomatik reclaim/orphan-state kodu, socket auth/token
  implementasyonu, Windows ACL kodu (yalnızca not).

## Uygulama Planı

1. `tests/` altına `smoke_real_build.rs` (veya integration'a ek case):
   küçük `cargo build` / `make` koşusu, `wait --until done` ile bitir.
   - [done] `tests/smoke_real_build.rs`: bağımlıksız mini crate
     (`tempfile`), `watch --detach` + `wait --timeout 300` → `done`/`0`,
     sonunda `shutdown`. Tek başına 3.85s.
2. CI'da ek süreye dikkat (`-j2` korunur).
   - [done] Ek süre ~4s; `-j2` korundu.
3. Politika dokümanını yaz, `cleanup --help` ile tutarlı tut.
   - [done] README EN+TR ajan bölümüne bayat-dosya paragrafı
     (`cleanup --older-than` + canlı-guard + stale'da yeniden `watch`);
     ayrı dosya açılmadı (crate paketine giren link sorunu olmasın).
4. `cargo test --locked -j2` art arda 2 koşuda yeşil.
   - [done] 2x: 72 unit + 3 drift + 14 integration + 1 smoke yeşil.

## Etkilenen Dosyalar

- `tests/smoke_real_build.rs` (yeni) veya `tests/integration.rs`
- `README.md` ve/veya `docs/OPERATIONS.md` (yeni)
- `.github/workflows/ci.yml` (gerekirse timeout)

## Doğrulama

- `cargo fmt`
- `cargo clippy --locked --all-targets -- -D warnings`
- `cargo test --locked -j2` (2x art arda yeşil)
