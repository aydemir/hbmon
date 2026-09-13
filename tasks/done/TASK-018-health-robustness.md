---
id: TASK-018
title: "health sağlamlık (oom throttle + stall VecDeque)"
status: done
priority: P2
created: 2026-09-13
updated: 2026-09-13
environment: both
labels: [daemon, health, perf]
depends_on: []
---

# TASK-018 — health sağlamlık

## Amaç (revize edildi — ilk varsayım bayattı)

-Denetimde görüldü: `oom::check` her tick'te değil, `poll_once` OOM bloğu
zaten 5sn throttle'lı (`last_oom_poll`) ve cgroup `memory.events` oom_kill
öncelikli, dmesg yedek. OOM yarısı önceden yapılmış — dokunulmadı.
Gerçek iş: `StallDetector` `p95()` her tick'te clone+sort yapıyor ve
`remove(0)` O(n) kaydırıyor. Davranış aynı, maliyet düşer.

## Kapsam

- Stall: `silent_samples` → `VecDeque` + push'ta p95 önbelleği (eşik formülü
  3×p95/min 30s/<10s guard aynen); tick başına clone+sort kalktı
- OOM/cgroup: değişiklik yok (denetlendi, çalışıyor)
- dep_missing deseni YOK (TASK-001 altyapısı ayrı task ister)
- Yapılmayacaklar: ML-ETA, yeni sağlık sinyali, eşik değişikliği

## Kapsam (dürüst revizyon, 2026-09-13)

- OOM yarısı ÖNCEDEN YAPILMIŞ: `poll_once` OOM bloğu zaten 5sn throttle'lı
  (`last_oom_poll`) ve cgroup `memory.events` oom_kill öncelikli, dmesg
  yedek — denetlendi, dokunulmadı (çalışanı kurcalama).
- Stall: `silent_samples` → `VecDeque` + push'ta p95 önbelleği (eşik formülü,
  3×p95/min 30s/<10s guard aynen); tick başına clone+sort kalktı.
- Yapılmayacaklar: ML-ETA, yeni sağlık sinyali, eşik değişikliği (korundu)

## Uygulama Planı

1. `src/health/oom.rs` throttle + cgroup kontrolü
2. `src/health/stall.rs` VecDeque + testler yeşil
3. Tick maliyeti göz kararı (dmesg fork sayısı logla doğrulanır)

## Etkilenen Dosyalar

- `src/health/oom.rs`
- `src/health/stall.rs`
- `src/metrics/cgroup.rs` (sadece okuma)

## Doğrulama

- `cargo test --locked -j2` yeşil (stall/oom unit'leri + yeni throttle testi)

## Gerçekleşme Notu (2026-09-13)

- Stall: `VecDeque` + `p95_cache` (push'ta yeniden hesap); 5 mevcut test
  aynen geçti (davranış korunumu) + yeni `rolling_window_caps_length_and_tracks_p95`
  (120 cap + p95 takibi + 30s taban).
- OOM: değişiklik yok — 5sn throttle + cgroup-öncelik denetlendi.
- Tam süit: 49 unit + 10 integration yeşil (1 gerekçeli-ignore).
