---
id: TASK-040
title: "cleanup Windows live-guard deliği"
status: done

## Doğrulama Kaydı (2026-09-14, Windows makine)

- Unit: `live_probe_addr_platform_rules` yeşil.
- Canlı: `watch --uuid t040guard` → `cleanup --older-than 0` →
  `{"removed":134}`, `.jsonl/.out/.pid` sağ kaldı → `status --compact`
  `running` → `shutdown` ok. Eski kod `.pid`'i silerdi.
priority: P1
created: 2026-09-14
updated: 2026-09-14
environment: windows
labels: [windows, cli, gc, correctness]
depends_on: [TASK-022]
---

# TASK-040 — cleanup Windows live-guard deliği

## Amaç

`run_cleanup` (`src/cli/mod.rs:104-110`) canlı setini SADECE `.sock`
girdilerinden kuruyor. Windows'ta pipe dosya değildir, `%TEMP%`'te
`.sock` hiç oluşmaz → `live` her zaman boş → canlı daemonun `.pid`
(ve yazılmayan `.jsonl`/`.out`) yaş eşiğini aşınca silinebilir.
TASK-022 koruması Windows'ta fiilen ölü.

## Kapsam

- Yapılacaklar: live-set kurulumunu probe-fn enjekte edilebilir hale
  getir (`live_uuids(entries, probe)`); Windows kolu uuid başına
  `can_connect(default_sock(uuid))` yoklar; unix davranışı birebir korunur.
- Unit test: stub probe ile canlı/ölü ayrımı (iki platformda da derlenir).
- Canlı doğrulama: bu makinede `watch` → `cleanup --older-than 0` →
  `.pid` sağ kalır → `shutdown`.
- Yapılmayacaklar: gc politikası değişikliği, yaş eşiği değişikliği.

## Uygulama Planı

1. `live_uuids` helper + cfg kolları
2. Unit test (stub probe)
3. Canlı CLI turu (Windows makine)
4. `cargo fmt/clippy/test`

## Etkilenen Dosyalar

- `src/cli/mod.rs`

## Doğrulama

- `cargo test --locked -j2` yeşil (Win + WSL1)
- Canlı turda `.pid` sağ kalır, stale silinir
