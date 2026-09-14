---
id: TASK-041
title: "Named-pipe ACL kilidi (current-user DACL)"
status: done

## Doğrulama Kaydı (2026-09-14, Windows makine)

- Unit: `pipe_acl_security_binds` (bind + aynı-kullanıcı) yeşil.
- Canlı: `watch t041b` → `status/list/wait/kill/shutdown` tam tur yeşil.
- Bulgu: el-yapımı absolute SD (40 bayt doğru olsa bile) `CreateNamedPipeW`'da
  1305 verdi; SDDL `"D:(A;;GA;;;OW)"` yolu çalışıyor. `serve` hatası yutuluyor
  (`daemon.rs:554 let _ =`) — başsız daemon tanısı zor; ayrı task'ta ele alınmalı.
- Docs: README EN+TR, RFC TR+EN §13, PROTOCOL.md, bug template güncellendi.
priority: P2
created: 2026-09-14
updated: 2026-09-14
environment: windows
labels: [windows, security, ipc]
depends_on: [TASK-006, TASK-034]
---

# TASK-041 — Named-pipe ACL kilidi (current-user DACL)

## Amaç

Windows pipe `CreateNamedPipeW sec=NULL` + `secure_fix` no-op
(`src/platform/perm.rs:37-40`): multi-user makinede başka kullanıcı
`status` okuyabilir / `kill` gönderebilir. Unix `0600` eşdeğeri yok.
TASK-034 bunu "yalnızca not" bırakmıştı; bug-report template'te
belgeli eksik olarak duruyor.

## Kapsam

- Yapılacaklar: ham FFI ile current-user-only DACL
  (`OpenProcessToken` → `TokenUser` SID → `SetEntriesInAcl` →
  `InitializeSecurityDescriptor` → `SECURITY_ATTRIBUTES` →
  `CreateNamedPipeW`); sıfır yeni bağımlılık; başarısızlık = fail-closed
  (`serve` Err, açık mesaj). Client tarafı aynı kullanıcı → etkilenmez.
- Cross-user izleme artık mümkün değil (deliğin kapanması = tasarım).
- Docs: README platform satırı + RFC §13 + bug template güncellenir.
- Yapılmayacaklar: token/auth protokolü, unix değişikliği, wire değişikliği.

## Uygulama Planı

1. `winffi.rs`: ACL/security API bildirimleri + struct'lar
2. `transport/windows.rs`: `serve` içinde SECURITY_ATTRIBUTES kurulumu
3. Bu makinede roundtrip + cross-check (`status`/`wait` entegrasyon)
4. Docs güncellemesi (README EN+TR, RFC TR+EN, bug template)

## Etkilenen Dosyalar

- `src/platform/winffi.rs`
- `src/ipc/transport/windows.rs`
- `README.md`, `README.tr.md`, `HBMON-RFC.md`, `HBMON-RFC-EN.md`
- `.github/ISSUE_TEMPLATE/bug_report.md`

## Doğrulama

- `cargo fmt/clippy/test` (Win + WSL1 derleme etkilenmez)
- Aynı-kullanıcı `watch/status/wait` turu yeşil
