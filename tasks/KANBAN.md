# hbmon — Roadmap & Kanban (2026-09-09)

`hbmon` deposu için planlama, takip ve önceliklendirme çalışma alanı.

## Hedef Kilidi (değişmez)

> **Kullanıcı LLM ajanıdır.** Tek binary, sıfır runtime bağımlılık, küçük boyut,
> context ekonomisi, pull-tabanlı, harness-bağımsız. İnsana-göz (TUI/Web),
> doğrulanamaz platform (Windows), ilke-delen eklenti (plugin sistemi) bu hedefin
> dışındadır. Dış öneriler (LLM code-review dahil) bu kilide çarpıp elenir;
> kararlar `decisions.md`'ye yazılır.

## Board Snapshot

| ID | Başlık | Status | Priority |
|----|--------|--------|----------|
| TASK-001 | Custom dep-missing patterns | done | P1 |
| TASK-002 | Release + crates.io yayını | done | P1 |
| TASK-003 | Dogfooding + demo | done | P2 |
| TASK-004 | opencode-plugins migrasyonu | done | P2 |
| TASK-005 | wait --until erken dönüş | done | P1 |
| TASK-006 | Windows portu (named pipe) | done | P2 |
| TASK-007 | exec ephemeral handshake | done | P1 |
| TASK-008 | Dead-dep + RFC drift temizligi | done | P2 |
| TASK-009 | Incremental dep-scan (B1 perf) | done | P1 |
| TASK-010 | DepMatch birlestirme | done | P3 |
| TASK-011 | run_daemon bolme | done | P3 |
| TASK-012 | test uuid cakisma sertlestirme | done | P3 |
| TASK-013 | Pull vs wrapper-push canlı deney + sidecar | done | P2 |
| TASK-014 | daemon dispatch split | done | P3 |
| TASK-015 | wait --until sinyal normalizasyonu | done | P1 |
| TASK-016 | status context ekonomisi (compact) | done | P1 |
| TASK-017 | sock keşif + gc (list/prune) | done | P2 |
| TASK-018 | health sağlamlık (oom+stall) | done | P2 |
| TASK-019 | drift kilidi + boyut kapısı | done | P2 |
| TASK-020 | release unblock hazırlığı | done | P1 |
| TASK-021 | dep-offset yarışı | done | P1 |
| TASK-022 | cleanup canlı-daemon koruması | done | P1 |
| TASK-023 | hbmon log CLI (log_tail op) | done | P2 |
| TASK-024 | out log cap (uzun build disk güvenliği) | done | P2 |
| TASK-025 | dep pattern genişletme (yeni ekosistemler) | done | P2 |
| TASK-026 | README ajan hızlı yolu güncelleme | done | P3 |
| TASK-027 | UUID validasyon + 64-bit | done | P1 |
| TASK-028 | JSON sözleşme kilidi | done | P1 |
| TASK-029 | log --event + list --state filtresi | done | P2 |
| TASK-030 | Crate hijyeni + EN vitrin (v0.1.1) | done | P2 |
| TASK-031 | HBMON-RFC bayatlık temizliği (Node portu + v0.1.1 senkronu) | done | P2 |
| TASK-032 | Tag öncesi sözleşme dokümanı (Stability + matrix + suspect) | done | P1 |
| TASK-033 | crates.io publish prosedürü + release.yml publish | done | P1 |
| TASK-034 | Gerçek build smoke testi + stale/orphan politika dokümanı | done | P2 |
| TASK-035 | HBMON-RFC İngilizce çevirisi (HBMON-RFC-EN.md) | done | P2 |
| TASK-036 | Tüketici desenleri belgesi (salt CLI / skill / plugin+MCP) | done | P2 |
| TASK-037 | GitHub issue template + known-limitations kontrol listesi | done | P3 |
| TASK-038 | Örnek skill paketi (skills/hbmon/SKILL.md) | done | P3 |
| TASK-039 | Windows clippy sıfırlama (Rust 1.98 lintleri) | done | P1 |
| TASK-040 | cleanup Windows live-guard deliği | done | P1 |
| TASK-041 | Named-pipe ACL kilidi (current-user DACL) | done | P2 |
| TASK-042 | Windows graceful TERM yoklaması (CTRL_BREAK) | done | P2 |
| TASK-043 | Windows net sayımı (GetTcpTable) + cmdline/OOM araştırma | done | P3 |
| TASK-044 | Windows test borcu (paths unit + ignored + boyut notu) | done | P2 |
| TASK-045 | Milestone-dili doc drift temizliği | done | P3 |
| TASK-046 | hbmon events (istemci-taraflı olay akışı) | done | P2 |
| TASK-047 | 2./3. göz inceleme kaydı + P1 düzeltmeleri (sock guard, wait terminal, exec timeout) | done | P1 |
| TASK-048 | TASK-047 incelemesinin kalan bulguları (exec stdout, watchdog UX, log_tail belleği, exit kodları, ölü kod) | done | P1 |
| TASK-049 | Proje simgesi (GitHub README + crates vitrini) | done | P3 |
| TASK-050 | Exit olayına özet satırı (bg_logs önizleme) | done | P2 |
| TASK-051 | v0.2.1 release (patch + binary + crates.io) | done | P1 |
| TASK-052 | macOS clippy sıfırlama (cfg artefaktı) | done | P1 |
| TASK-053 | macOS ignored sock yolu (UDS 104) + serve körlüğü | done | P1 |
| TASK-054 | Windows CI spawn borçları (7 test) | done | P1 |
| TASK-055 | v0.2.2 release (patch + binary + crates.io) | todo | P1 |
