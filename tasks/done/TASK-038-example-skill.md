---
id: TASK-038
title: "Örnek skill paketi (skills/hbmon/SKILL.md)"
status: done
priority: P3
created: 2026-09-13
updated: 2026-09-13
environment: both
labels: [docs, agents, harness]
depends_on: [TASK-036]
---

# TASK-038 — Örnek skill paketi

## Amaç

3. göz (v2) §7: `USAGE-PATTERNS.md`'deki skill iskeleti minimal; kopyala-
çalıştır bir örnek yok. Core'a skill alınmaz (hedef kilidi), ama dışarıda
paketlenebilir bir *örnek* tüketici belgesinin doğal devamıdır.

## Kapsam

- Yapılacaklar:
  1. `skills/hbmon/SKILL.md`: frontmatter (`name`, `description`) +
     sözleşme (handshake/discovery/`until`/exit/frozen `until` adları) +
     suspect aksiyon tablosu + stale politika — tamamı README +
     `PROTOCOL.md`'den alıntı, yeni bilgi yok.
  2. Terimler `PROTOCOL.md` ile birebir (çeviri yok, kopya).
- Yapılmayacaklar: core kodu, harness'e özel kurulum script'i, TR sürüm.

## Uygulama Planı

1. `SKILL.md` yaz (tek dosya, ~60 satır).
   - [done] `skills/hbmon/SKILL.md`: frontmatter + handshake/discovery/
     `until`/exit/suspect/stale — hepsi README+PROTOCOL alıntısı.
2. Komut/sinyal adlarını `PROTOCOL.md`'ye karşı `grep` ile doğrula.
   - [done] 6 kanonik + 2 alyas + INVALID_UNTIL + exit kodları iki
     belgede de mevcut.
3. `cargo package --list` ile pakete girdiğini doğrula (md dosyaları
   varsayılan paketlenir).
   - [done] Pakette. Drift 3/3 yeşil.

## Etkilenen Dosyalar

- `skills/hbmon/SKILL.md` (yeni)

## Doğrulama

- `cargo test --locked -j2 --test drift` yeşil
- `until` adları/exit kodları `PROTOCOL.md` ile aynı.
