---
id: TASK-008
title: "Ölü bağımlılık + RFC drift temizliği"
status: done
priority: P2
created: 2026-09-09
updated: 2026-09-09
environment: both
labels: [docs, deps, rfc]
depends_on: [TASK-007]
---

# TASK-008 — Ölü bağımlılık + RFC drift temizliği

## Amaç

Kod-doküman uyumsuzluklarını kapatmak (B2 + B4). Davranış değişikliği yok;
sadece doküman ve bağımlılık temizliği.

## Kapsam

- `crossbeam-channel` bağımlılığı `Cargo.toml` + `Cargo.lock`'tan çıkarılır
  (src'de sıfır kullanım; `wait` channel'a taşınmayacak — karar: sil)
- RFC §14.1 test sayıları güncellenir (40 unit + 7 integration, tarihli)
- RFC §14.2'den gerçekleşenler düşülür (cgroup v2 + custom patterns v1'de;
  "v1'e çekildi" notu eklenir)
- RFC §5.2.3 exit tablosu düzeltilir: `147` satırı kaldırılır/düzeltilir
  (stall exit'e yansımaz, yalnızca state+event), `130/143` satırlarına
  "implement edilmedi" notu, `ALREADY_SHUTDOWN` satırına "yok" notu
- RFC `eta_sec`: "opsiyonel/gelecek" olarak işaretlenir (şemadan silinmez)
- `HBMON-RFC.md` §15'e TASK-007/008/009 referansı eklenir (tek satır)

- Yapılmayacaklar: davranış değişikliği, `rand` temizliği (uuid için
  yaşıyor, tolere), TUI/Windows/Prometheus/plugin (hedef kilidi dışı)

## Uygulama Planı

1. `Cargo.toml`'dan `crossbeam-channel` satırını sil → `cargo build`
   (lock yeniden üretilir) → `cargo test --locked -j2` ile kilitli doğrulama
2. RFC düzenlemeleri (yukarıdaki 5 madde, satır numaraları PR'da belirtilir)
3. CI yeşil

## Etkilenen Dosyalar

- `Cargo.toml`, `Cargo.lock`
- `HBMON-RFC.md`

## Doğrulama

- `cargo build --locked` + `cargo test --locked -j2` yeşil + CI yeşil
- `grep -rn crossbeam src/ Cargo.toml` boş döner
