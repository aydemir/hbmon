# Dogfooding: hbmon kendini izledi (release derlemesi)

Tarih: 2026-09-09. `cargo clean` sonrası full `cargo build --release -j1`
(~40 bağımlılık + LTO link), Termux/Android cihazda, tek çekirdek.

## Komutlar

```bash
hbmon watch --detach --timeout-sec 1200 -- cargo build --release -j1
# → {"ev":"ready","uuid":"c0d8e0fd",...}  (189ms'de döndü)

hbmon status --sock /tmp/hbmon-c0d8e0fd.sock   # ara yoklamalar
hbmon wait --sock /tmp/hbmon-c0d8e0fd.sock --timeout 900
# → {"state":"done","code":0,"duration_sec":210.2}
```

## Zaman çizelgesi (status yoklamaları)

| elapsed | state | cpu% | rss MB | io_w MB | faz |
|---|---|---|---|---|---|
| 64s | running | 97.3 | 370 | – | bağımlılık derlemesi (rustc) |
| 122s | running | 97.6 | 265 | 170 | bağımlılık derlemesi |
| 178s | running | 96.7 | 233 | – | hbmon crate + LTO link |
| 210s | done | – | – | – | exit 0 |

## JSONL (43 satır: 1 ready + 41 metric + 1 exit)

```json
{"ev":"ready","uuid":"c0d8e0fd","root_pid":1956,"cmd":"cargo build --release -j1","workdir":"/root/hbmon",…}
{"ev":"metric","uuid":"c0d8e0fd","cpu":97.3,"rss_mb":370,…}
…
{"ev":"exit","uuid":"c0d8e0fd","pid":1956,"code":0,"duration_sec":210.2,"state":"done"}
```

Stall/OOM/dep olayı çıkmadı (temiz derleme — beklenen). Ajanın
context'ine giren toplam build log'u: **0 bayt** (2 status + 1 wait JSON'u hariç).
