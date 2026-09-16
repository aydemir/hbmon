---
id: TASK-046
title: "hbmon events: istemci-taraflı olay akışı (push hissi, çekirdeğe dokunmadan)"
status: done
priority: P2
created: 2026-09-16
updated: 2026-09-16
environment: both
labels: [cli, events, experiment, sidecar]
depends_on: [TASK-013, TASK-023]
---

# TASK-046 — hbmon events: istemci-taraflı olay akışı

## Amaç

LLM ajan uzun işin olaylarını "push" ile alsın: `hbmon events` komutunu
arka planda çalıştıran ajana `exit/dep_missing/...` olayları stdout'a
düştükçe ulaşır. TASK-013'ün "pull + olay-bazı sidecar" hükmünün yerleşik
hâli; shell prototipi (`tail -F | jq`) yerine tek binary'de komut.

## Kapsam

- `hbmon events --sock --event exit,dep_missing --tail 5 --timeout 300 --poll-ms 500`
- Ham `.jsonl` satırları aynen akar (`{"ts","v":1,"ev",...}`); `exit`
  olayında build koduyla çıkış; süre aşımında exit 124 (wait ile aynı dil)
- `shutdown` ile kapanan daemon'da `exit` olayı basılmaz → emniyet supabı:
  yeni satır yokken her 10 poll'da compact status, terminal state'te code
- Yapılmayacaklar: daemon/IPC değişikliği yok (`IPC_OPS` aynı, drift
  yeşil), kalıcı bağlantı yok, server-side push yok, TUI/Web yok

## Uygulama Planı

1. `src/cli/events.rs` (saf yardımcılar birim testli) + `mod.rs` wiring
2. 3 entegrasyon testi (akış+kod, filtre+replay, timeout 124)
3. `cargo fmt/clippy/test` kapıları + canlı demo (arka plan akışı)

## Etkilenen Dosyalar

- `src/cli/events.rs` (yeni), `src/cli/mod.rs` (wiring)
- `tests/integration.rs` (3 test)
- `tasks/done/TASK-046-events-stream.md` (bu dosya)
- `tasks/KANBAN.md`, `tasks/index.json`, `tasks/decisions.md`

## Doğrulama

- `cargo fmt` temiz, `clippy -- -D warnings` sıfır uyarı
- `cargo test --locked -j2`: 82 unit + 18 integration (4 yeni) + smoke yeşil
- Canlı: `watch --detach -- sleep 4` + `events` → ready/metric/exit satırları + exit 0

## 2./3. göz incelemesi (2026-09-16, aynı gün)

- 2. göz (kırılma): `stream_start` bayt-aynı komşu satırda sınır-kopyayı
  yutuyordu → `StreamPos` blok sayacına çevrildi (bitişik tekrar korunur;
  kalıntı teori: araya farklı satır giren bayt-aynı çift — pratikte
  imkânsız, kod yorumunda belgeli). Ölü soket/`shutdown`-ortası akış
  exit 3'e kilitlendi (test).
- 3. göz (sözleşme): RFC TR/EN komut listeleri + USAGE-PATTERNS `events`'siz
  kalmıştı (TASK-045 sınıfı drift) → üç dosya güncellendi, EN ağaç satırı
  bayt-aynı doğrulandı. Kilit hükmü değişmedi: yeni IPC op yok, drift yeşil.
