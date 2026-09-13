---
id: TASK-014
title: "daemon dispatch split (status_map/wait_match/dispatch)"
status: done
priority: P3
created: 2026-09-13
updated: 2026-09-13
environment: both
labels: [daemon, refactor, readability]
depends_on: [TASK-011]
---

# TASK-014 — daemon dispatch split

## Amaç

`src/daemon/daemon.rs` hâlâ ~955 satır; TASK-011 `poll_once`'u çıkardı ama
`dispatch` (~140 satır) + `status_map` + `wait_match` aynı dosyada.
Okunabilirlik ve merge-sürtünme için davranış korunarak bölmek.
Kullanıcı = LLM ajanı (kodu okuyan da ajan).

## Kapsam

- `dispatch`/`status_map`/`status_snapshot`/`wait_match` → `src/daemon/handler.rs` (yeni modül); `daemon.rs` sadece lifecycle + poll
- İmza değişikliği yok, IPC şema değişikliği yok
- Yapılmayacaklar: davranış değişikliği, yeni op, Shared mutex yapısı değişikliği (TASK-011 gerekçesi geçerli)

## Uygulama Planı

1. `handler.rs` modülü çıkar, küçük adımlarla taşı (her adımda derle)
2. `cargo fmt` + `cargo clippy --locked --all-targets -- -D warnings`
3. `cargo test --locked -j2` (mevcut 44 unit + integration yeşil kalmalı)

## Etkilenen Dosyalar

- `src/daemon/daemon.rs`
- `src/daemon/handler.rs` (yeni)
- `src/daemon/mod.rs`

## Doğrulama

- `cargo test --locked -j2` yeşil + CI yeşil
- `daemon.rs` <700 satır, davranış farkı yok

## Gerçekleşme Notu (2026-09-13)

- `daemon.rs` 1027 → 658 satır; `handler.rs` 392 satır (Shared +
  dispatch/status_map/status_snapshot/status_compact/wait_match +
  5 wait_match testi; dep_scan testi daemon tarafında).
- `Shared` + alanları `pub(crate)`; imza/şema değişikliği yok.
- Arazi notları: `mod.rs`'e yanlışlıkla çift mod satırı eklendi (yakalandı,
  düzeltildi); script dilimi ilk testin `#[test]` özniteliğini düşürdü
  (clippy `dead_code` ile yakalandı); `paths`/`ProcessInspector` importları
  handler'da gereksiz çıktı (trait'siz dyn çağrı).
- Kapılar: fmt + clippy + windows-gnu check temiz; tam süit 49+3+10
  yeşil (1 gerekçeli-ignore).
