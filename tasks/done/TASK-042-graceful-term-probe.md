---
id: TASK-042
title: "Windows graceful TERM yoklaması (CTRL_BREAK)"
status: done

## Sonuç (2026-09-14): won't-fix, kanıtlı

- `GenerateConsoleCtrlEvent` console'suz detached mimaride hedefe ulaşamaz;
  console'lu spawn detach'ı deler (kilit ihlali). `decisions.md` girdisi +
  `signal.rs` yorumu yazıldı, bug template'e kutu eklendi. Kod değişikliği
  yok (bilinçli).
priority: P2
created: 2026-09-14
updated: 2026-09-14
environment: windows
labels: [windows, daemon, signals]
depends_on: [TASK-006]
---

# TASK-042 — Windows graceful TERM yoklaması (CTRL_BREAK)

## Amaç

`signal.rs:101-102`: Term de Kill de `TerminateJobObject`. TASK-006/M3-13
`TERM→GenerateConsoleCtrlEvent(CTRL_BREAK)` planlamış ama gerçekleşme
terminate'e düşmüş. Kapatılabilir mi, yoksa mimari olarak imkânsız mı?
Karar kanıtla verilir.

## Kapsam

- Yoklama: daemon `DETACHED_PROCESS` + console yok; child console
  devralmaz → `GenerateConsoleCtrlEvent` hedefe ulaşamaz (MSDN:
  yalnızca console'lu grup). Gerçekleşirse kod, gerçekleşmezse
  `decisions.md` + kod yorumu ile won't-fix kaydı.
- Yapılmayacaklar: console'lu spawn'a dönüş (detach'ı deler),
  unix değişikliği.

## Uygulama Planı

1. Mimari kanıtı yaz (detach bayrakları + console yokluğu)
2. Sonuç: kod VEYA decisions.md girdisi + `windows_kill` yorumu

## Etkilenen Dosyalar

- `src/platform/signal.rs` (yorum) ve/veya `tasks/decisions.md`

## Doğrulama

- Karar yazılı, `cargo clippy/test` yeşil
