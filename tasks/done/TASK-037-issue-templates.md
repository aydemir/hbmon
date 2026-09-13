---
id: TASK-037
title: "GitHub issue template + known-limitations kontrol listesi"
status: done
priority: P3
created: 2026-09-13
updated: 2026-09-13
environment: both
labels: [docs, operability, github]
depends_on: [TASK-032]
---

# TASK-037 — GitHub issue template + known-limitations kontrol listesi

## Amaç

3. göz (v2) §3 doğrulandı (daraltılmış): Windows named-pipe ACL yokluğu
README matrix'te belgeli (`no ACL lockdown`), ama `.github/`'da issue
template yok — "çalışmıyor" issue'ları platform bilgisi olmadan gelecek.
`src/platform/perm.rs` Windows'ta no-op; kod değişikliği yok, sadece
rapor kalitesi.

## Kapsam

- Yapılacaklar:
  1. `.github/ISSUE_TEMPLATE/bug_report.md`: OS / sürüm / `status
     --compact` çıktısı / sock keşif yolu alanları.
  2. Bilinen limitation onay kutuları: Windows named-pipe ACL yok,
     cgroup reaping HBMon kontrolünde değil (RFC §4.2.1), macOS CPU
     best-effort, Windows cmdline = exe yolu. Her kutu README matrix'e
     link verir.
  3. `config.yml` (boş issue engelleme notu, `contact_links` yok).
- Yapılmayacaklar: kod/ACL implementasyonu, discussion template'i.

## Uygulama Planı

1. İki dosyayı yaz.
   - [done] `bug_report.md` (OS/sürüm/komut/compact/logtail alanları +
     4 known-limitation kutusu) + `config.yml`.
2. Kutu metinlerinin README matrix satırlarıyla aynı olduğunu gözle
   doğrula (drift yok — test etkilenmez).
   - [done] `no ACL lockdown` / best-effort / exe yolu / cgroup —
     matrix satırlarıyla aynı. Drift 3/3 yeşil.

## Etkilenen Dosyalar

- `.github/ISSUE_TEMPLATE/bug_report.md` (yeni)
- `.github/ISSUE_TEMPLATE/config.yml` (yeni)

## Doğrulama

- `cargo test --locked -j2 --test drift` yeşil (doküman işi, tripwire)
- Markdown link hedefleri mevcut dosyalara işaret eder.
