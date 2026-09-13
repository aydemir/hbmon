---
id: TASK-032
title: "Tag öncesi sözleşme dokümanı (Stability + platform matrix + suspect aksiyonu)"
status: done
priority: P1
created: 2026-09-13
updated: 2026-09-13
environment: both
labels: [docs, contract, agents]
depends_on: [TASK-028, TASK-031]
---

# TASK-032 — Tag öncesi sözleşme dokümanı

## Amaç

3. göz §1-2-7-8 doğrulandı: handshake/exit-code/`until` adları/compact
alanları de-facto public API (`src/health/mod.rs:33-44` exit mapping,
`src/daemon/handler.rs:308-333` `wait_match`, TASK-028 drift kilidi) ama
README'de SemVer garantisi yok; platform farkları tek cümle
(`README.md:8-9`), tablo yok; `stall_suspect`/`oom_suspect`'te ajanın ne
yapacağı yazmıyor (`README.md:60-61` sadece exit 2'yi anlatır); İngilizce
tek-sayfalık protokol özeti yok (RFC Türkçe).

## Kapsam

- Yapılacaklar (yalnızca doküman, kod değişikliği yok):
  1. README'ye "Stability & SemVer" paragrafı: frozen (handshake v1,
     exit mapping 0/1/2/124/137/3, `WAIT_SIGNALS`, compact alan seti,
     IPC_OPS) vs experimental (metric alanları, stall eşiği).
  2. Platform matrix tablosu: Linux full / macOS CPU best-effort /
     Windows Toolhelp+Job Object, net best-effort, cmdline=exe yolu
     (`src/proc/windows.rs:8`), unix sock `0600` (`src/platform/perm.rs`)
     vs Windows ACL no-op.
  3. Suspect aksiyon tablosu: `stall_suspect` → `status --compact` +
     `log --event metric` ile teyit, sonra `kill`/devam; `oom_suspect` →
     retry değil, bellek küçült; `dep_missing` → kur + retry.
  4. İngilizce mini Protocol Reference (handshake JSON + exit + discovery
     sırası) — RFC çevirisi değil, 1 sayfa.
- Yapılmayacaklar: kod/şema değişikliği, RFC çevirisi, TUI/Web.

## Uygulama Planı

0. [done] RFC atıfları linklendi (`README.md` + `README.tr.md`, 3'er nokta).
1. `README.md` + `README.tr.md`'ye Stability + matrix + suspect tablosu.
2. `docs/PROTOCOL.md` (EN, 1 sayfa) ekle, README'den linkle.
3. Drift etkilenmez (`tests/drift.rs` yeşil kalmalı).

## Etkilenen Dosyalar

- `README.md`
- `README.tr.md`
- `docs/PROTOCOL.md` (yeni)

## Doğrulama

- `cargo test --locked -j2` yeşil (özellikle `tests/drift.rs`)
- Gözle: matrix'te her hücre "full / best-effort / yok" diyor, boş yok.
