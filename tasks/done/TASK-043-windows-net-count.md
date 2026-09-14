---
id: TASK-043
title: "Windows net sayımı (GetTcpTable) + cmdline/OOM araştırma"
status: done

## Doğrulama Kaydı (2026-09-14, Windows makine)

- Unit: `loopback_tcp_counted_for_self` (listener+accept+connect ≥3 satır,
  olmayan pid → 0, panik yok) yeşil.
- Canlı: `watch t043net` → `status` → `shutdown` tam tur yeşil.
- cmdline/OOM: won't-fix, gerekçe `decisions.md`'de. Docs: README EN+TR,
  RFC TR+EN güncellendi.
priority: P3
created: 2026-09-14
updated: 2026-09-14
environment: windows
labels: [windows, metrics, research]
depends_on: [TASK-006]
---

# TASK-043 — Windows net sayımı (GetTcpTable) + cmdline/OOM araştırma

## Amaç

`net_tcp/net_udp` Windows'ta sabit 0; `cmdline` exe yolu; OOM boş.
Kapatılabilir olanı kapat, fragile olanı gerekçesiyle ele.

## Kapsam

- Yapılacaklar:
  1. `net_tcp`: `GetTcpTable2`/`GetExtendedTcpTable` (iphlpapi, ham FFI,
     sıfır-dep) ile izlenen pid'lerin TCP sayımı; best-effort, hata=0.
  2. `cmdline` (PEB okuma) + OOM (EventLog) için araştırılabilirlik
     notu → `decisions.md`: PEB fragile (WOW64/protected), EventLog
     ağır → ikisi de won't-fix (belgeli eksik kalır).
- Yapılmayacaklar: WMI (COM maliyeti, kilit ruhuna aykırı),
  unix değişikliği, şema değişikliği.

## Uygulama Planı

1. `winffi.rs`: iphlpapi bildirimleri
2. `proc/windows.rs`: `net_tcp_for(pids)` + metrics kablolaması
3. Unit test (kendi pid'i için >=0 sayım, panik yok)
4. decisions.md araştırma girdisi (cmdline/OOM)

## Etkilenen Dosyalar

- `src/platform/winffi.rs`
- `src/proc/windows.rs`
- `tasks/decisions.md`

## Doğrulama

- `cargo fmt/clippy/test` (Win + WSL1)
