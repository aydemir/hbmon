# AGENTS.md — hbmon ajan talimatı

Bu depo LLM ajanları içindir. Kod yazmadan önce `tasks/KANBAN.md:7`'deki
**hedef kilidini** ve `tasks/decisions.md`'yi oku; dış öneri kilide çarparsa elenir.
Detaylı sözleşme `HBMON-RFC.md`'dedir. Ajan hızlı yolu `README.md`'dedir.

## Kalite kapıları (her Rust düzenlemesinden sonra, sırayla)

```bash
cargo fmt
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked -j2
```

- Sıfır uyarı + sıfır format farkı olmadan bırakma (self-correct).
- CI aynı üçünü koşar (`fmt --check`, `clippy -D warnings`, build+test;
  4 OS). CI'da patlayacak kodu push'lama.
- LSP/MCP yokluğunda terminal döngüsü kanonik yöntemdir.

## Task disiplini

- Yeni iş: `tasks/todo/TASK-0XX-kisa-ad.md` (`tasks/_template.md` şablon),
  `tasks/KANBAN.md` satırı + `tasks/index.json` girdisiyle birlikte.
- Biten iş: `tasks/done/`'a taşı, frontmatter `status: done`, KANBAN + index güncelle.
- ID'ler benzersiz olmalı (TASK-006 çakışması örneği: çift başlık yasaktır).

## Test kuralları

- Integration `uuid()` pid+nanos üretir (TASK-012); kısaltma/sadeleştirme yapma.
- Yeni davranış = yeni/kilitli test; `cargo test` art arda koşularda yeşil kalmalı.

## Commit

- Sadece açık istekle commit/push yap. Mesaj dili: `TASK-0XX done: <kısa>` veya `fix(kapsam): <kısa>`.
