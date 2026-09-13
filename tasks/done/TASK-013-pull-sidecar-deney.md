---
id: TASK-013
title: "Pull vs wrapper-push canlı deney + sidecar"
status: done
priority: P2
created: 2026-09-10
updated: 2026-09-10
environment: both
labels: [experiment, sidecar, pull, validation]
depends_on: [TASK-005]
---

# TASK-013 — Pull vs wrapper-push canlı deney + sidecar

## Amaç

Kullanıcı tezi: `hbmon` yerine geçen tokio+reqwest wrapper her stdout/stderr
satırını `is_target_event` filtresinden geçirip `localhost:3000`'e POST'lasın.
Karşı tez (kilide uygun): wrapper yok, `watch --detach` + `wait --until` +
`status` pull yeter; push şartsa core'a dokunmayan JSONL-tail sidecar ile
olay-bazı yapılır. Canlıda ölç, hangisi context/performans/kırılganlık
açısından kazanıyor göster.

## Kapsam

- Canlı `watch --detach -- <cmd>` → `wait --until done,failed,dep_missing,stall_suspect,oom_suspect` → `status` turu
- JSONL olay sayımı: toplam satır vs `ev in (exit,dep_missing,stall_suspect,oom_suspect,timeout)` vs wrapper filtresinin yakalayacağı satır sayısı (`error/fail/event/state` contains)
- Shell sidecar prototipi: `tail -F $LOG | jq` ile olay-bazı filtre, batch/debounce taslağı (kod yok, komut satırı)
- Yapılmayacaklar: `hbmon-core` rename yok, wrapper binary derleme yok, `tokio/reqwest` bağımlılık yok, `src/**` Rust değişikliği yok, TUI/Web yok, plugin sistemi yok

## Uygulama Planı

1. Kısa ve uzun iki derleme seç (örn. `cargo build` + yapay yavaş iş)
2. Her biri için watch/wait/status turu + JSONL arşivi (`uuid/sock/log` handshake kaydı)
3. Üç sayıyı raporla: toplam log satırı, wrapper-POST sayısı (alt sınır tahmini), sidecar-POST sayısı (gerçek olay sayısı)
4. Sonucu `decisions.md`'ye bir satır hüküm olarak yaz + bu dosyayı `done/`'a taşı

## Etkilenen Dosyalar

- `tasks/todo/TASK-013-pull-sidecar-deney.md` (bu dosya)
- `tasks/KANBAN.md`, `tasks/index.json`
- Deney artefaktı: `/tmp` altında handshake + JSONL kopyası (repoya commit yok)

## Doğrulama

- `cargo` değişikliği yok → `fmt/clippy/test` gerekmez
- `hbmon status --format json` içinde `state,last_event,log_tail` dolu
- `hbmon wait` `woke_on` ile erken/bitiş dönüşü kanıtlı
- Wrapper-hipotez sayaç >> sidecar sayaç (tez/sentez farkı sayıyla görünür)

## Canlı Sonuç (2026-09-10, uuid=bd8e75b3)

Komut: `hbmon watch --detach -- bash -c 'echo state...; echo event...; echo hello; echo FAILED; echo linking failed; sleep 3; exit 1'`

- `wait --until done,failed,dep_missing,stall_suspect,oom_suspect` → `woke_on=dep_missing`, `state=dep_missing`, `code=2` (exit 124/137/0/1/2/3 sözleşmesi korundu; `link_fail` paterni `error: linking with cc failed` satırını yakaladı)
- `.out` 5 satır → wrapper `is_target_event` 4 POST atardı (`state update tick`, `event fired` dahil — ikisi zararsız bilgi satırı, gürültü)
- `.jsonl` 4 olay (`ready,metric,dep_missing,exit`) → sidecar 2 PUSH (`dep_missing`, `exit`); `ready/metric` SKIP
- Hüküm: 5 satırlık oyuncak işte bile wrapper 2× gürültü üretti; gerçek derlemede (binlerce satır `state/event` geçen log) fark katlanır. Wrapper tezi çürütüldü, pull + olay-bazı sidecar doğrulandı.
