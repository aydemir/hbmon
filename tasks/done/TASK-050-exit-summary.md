---
id: TASK-050
title: "Exit olayına özet satırı (bg_logs için tam .out okumasız önizleme)"
status: done
priority: P2
created: 2026-09-23
updated: 2026-09-23
environment: both
labels: [eventlog, context-economy, nabiz-consumer]
depends_on: []
---

# TASK-050 — Exit olayına özet satırı

## Amaç

Tüketici (nabız `bg_logs`) bir task'ın ne olduğunu anlamak için bugün tüm `.out`'u
(50KB cap ile) okumak zorunda. Exit olayına kısa bir `summary` alanı eklenirse
tüketici önce özeti okur, tam log'u yalnızca gerektiğinde açar. Kullanıcı = LLM
ajanı; hedef kilidi (context ekonomisi) ile doğrudan uyumlu.

İlham: minimax-code `TaskOutputStore::finalize(summary)` — biten task'a kısa özet
persist edilir, liste/okuma özeti kullanır.

## Kapsam

- Yapılacaklar
  - `ev:"exit"` satırına `summary` alanı: `.out`'un son ~2KB'ı (ya da ilk 1KB +
    son 1KB, karar verilecek), tek alan, JSON kaçar.
  - `PROTOCOL.md` + `HBMON-RFC.md` sözleşme güncellemesi (alan opsiyonel, eski
    log'lar summary'siz geçerli).
  - Sözleşme kilidi testi (TASK-028 mirası) güncellemesi.
- Yapılmayacaklar (out-of-scope)
  - Daemon içinde LLM/AI özetleme — ham kırpma, yorum yok.
  - Yeni transport, yeni dosya (`.summary` dosyası yok; özet `.jsonl`'de).
  - `status --compact` çıktısına summary gömme (ayrı TASK adayı).

## Uygulama Planı

1. Exit olayı struct'ına `summary: Option<String>` ekle; `.out`'tan son 2048
   baytı al (satır sınırında kes, başına `…` koy).
2. Eski log uyumluluğu: summary'siz `exit` parse edilmeye devam etsin (test).
3. `PROTOCOL.md` + RFC'ye alanı belgele (opsiyonel, max ~2KB).
4. Kilit testini güncelle (`cargo test -j1` yeşil + CI yeşil).

## Etkilenen Dosyalar

- `src/eventlog/*` (exit olayı yazımı)
- `src/daemon/daemon.rs` (exit yayınlayan nokta)
- `PROTOCOL.md`, `HBMON-RFC.md` (+EN karşılığı gerekiyorsa)
- Sözleşme kilit testleri

## Doğrulama

- `cargo test -j1` yeşil + CI yeşil
- Gerçek build'de `log --event exit` çıktısında `summary` görünür, eski
  `.jsonl`'ler hatasız okunur
