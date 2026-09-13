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
