---
id: TASK-007
title: "exec handshake dürüstlüğü (ephemeral sözleşme)"
status: done
priority: P1
created: 2026-09-09
updated: 2026-09-09
environment: both
labels: [cli, contract, exec]
depends_on: []
---

# TASK-007 — exec handshake dürüstlüğü (ephemeral sözleşme)

## Amaç

`hbmon exec` stdout satır 1'e `sock`/`log` basıyor ama daemon olmadığı için
bu dosyalar hiç oluşmuyor (B3). LLM bu sock'a `status`/`wait` denerse
`connect` hatası alır. Handshake, varolmayan monitörü ilan etmemeli.

## Kapsam

- Handshake JSON'a `"ephemeral":true` + `"note":"no daemon; status/wait unavailable"` eklenir
- `uuid`/`sock`/`log` alanları korunur (ileri-uyumluluk; yollar rezerve
  adlardır, canlı monitör değildir)
- README `exec` satırına ephemeral notu (tek satır)
- Integration test: `ephemeral==true` assert + sock dosyasının oluşmadığı
  assert (sözleşme teste kilitlenir)

- Yapılmayacaklar: exec için gerçek JSONL yazma, daemon ekleme,
  handshake alan adı değişikliği, RFC şema değişikliği (o TASK-008'de)

## Uygulama Planı

1. `src/cli/exec.rs`: handshake `println!` formatına 2 alan ekle
2. `tests/integration.rs::exec_handshake_and_exit_zero`: `ephemeral`
   assert + `sock` yokluğu assert
3. `README.md`: exec satırına ephemeral notu
4. `cargo test --locked -j2` + CI

## Etkilenen Dosyalar

- `src/cli/exec.rs`
- `tests/integration.rs`
- `README.md`

## Doğrulama

- `cargo test --locked -j2` yeşil + CI yeşil
- Manuel: `hbmon exec -- echo hi` çıktısının 1. satırı JSON parse edilir,
  `ephemeral==true`, belirtilen sock dosyası diskte yoktur
