# hbmon — Harness-Bağımsız Build Monitor

LLM kodlama ajanları için tek ikilik, sıfır-runtime-bağımlılık derleme izleyici.
Hangi harness kullanılırsa kullanılsın (OpenCode, Claude Code, Aider, …),
hangi OS olursa olsun çalışır — v1: Linux'ta tam izleme (CPU/RSS/IO/FD),
macOS'ta süreç gözetimi + RSS/FD/yol (CPU best-effort).

## Nasıl çalışır

Üç katmanlı ayrıştırma: **yaşam döngüsü** (setsid + double-fork → init'e reparent),
**iletişim** (sidecar JSONL + UDS üzerinden çekme-tabanlı sorgu, context'e log sızmaz),
**keşif** (env var + `/tmp/hbmon-*.sock` convention).

## Kurulum

```bash
cargo install --path .   # veya: cargo build --release
```

## Kullanım

```bash
# Arka planda izle, hemen dön
hbmon watch --detach -- make -j8
# {"v":1,"ev":"ready","uuid":"...","sock":"/tmp/hbmon-....sock","log":"..."}

# Durum sorgula
hbmon status --sock /tmp/hbmon-<uuid>.sock

# Bitene kadar bekle (bloklamalı)
hbmon wait --sock /tmp/hbmon-<uuid>.sock --timeout 600

# Ön planda çalıştır (ilk satır handshake JSON)
hbmon exec -- make -j8   # exit: 0 ok, 1 fail, 2 dep-missing
hbmon exec --format json -- make -j8  # stderr son satır: JSON özet

# Öldür / kapat
hbmon kill --sock ... --signal TERM
hbmon shutdown --sock ...
```

## Durum

v1 MVP: daemon, UDS JSON-RPC (`status`/`metrics`/`wait`/`kill`/`log_tail`/`shutdown`),
JSONL olay günlüğü, adaptif stall tespiti, OOM şüphesi (dmesg), dependency-missing
desen eşleme, timeout watchdog. Tasarımın tamamı için `HBMON-RFC.md` (ayrı doküman).
