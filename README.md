# hbmon — Harness-Bağımsız Build Monitor

LLM kodlama ajanları için tek ikilik, sıfır-runtime-bağımlılık derleme izleyici.
Hangi harness kullanılırsa kullanılsın (OpenCode, Claude Code, Aider, …),
hangi OS olursa olsun çalışır — Linux'ta tam izleme (CPU/RSS/IO/FD),
macOS'ta süreç gözetimi + RSS/FD/yol (CPU best-effort), Windows'ta tam
izleme (Toolhelp+RSS/IO/handle; net best-effort, cmdline = exe yolu).

## Nasıl çalışır

Üç katmanlı ayrıştırma: **yaşam döngüsü** (setsid + double-fork → init'e reparent),
**iletişim** (sidecar JSONL + UDS üzerinden çekme-tabanlı sorgu, context'e log sızmaz),
**keşif** (env var + `/tmp/hbmon-*.sock` convention).

## Kurulum

```bash
cargo install --git https://github.com/aydemir/hbmon
# veya kaynaktan: cargo build --release  # strip'li ikilik ~2.4MB
# Windows: aynısı → target\release\hbmon.exe (named pipe transport)
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

# Ön planda çalıştır (ilk satır handshake JSON; ephemeral — daemon yok,
# status/wait kullanılamaz, çıkışta sock/log oluşmaz)
hbmon exec -- make -j8   # exit: 0 ok, 1 fail, 2 dep-missing
hbmon exec --format json -- make -j8  # stderr son satır: JSON özet

# Öldür / kapat
hbmon kill --sock ... --signal TERM
hbmon shutdown --sock ...
```

## Ajanlar için (otomat hızlı yolu)

Bu bölüm LLM ajanları içindir; detaylı sözleşme `HBMON-RFC.md`'dedir, orayı ıskalamayın.

1. `hbmon watch --detach -- <cmd>` → stdout satır 1 = handshake JSON
   `{v,ev:"ready",uuid,sock,log}`. Satır 1'i parse et, `sock`'u sakla.
2. Poll: `hbmon status --sock $SOCK`, veya bloklan:
   `hbmon wait --sock $SOCK --until done,failed,dep_missing,timeout`.
3. Exit: `0 done / 1 failed / 2 dep-missing / 124 timeout / 137 oom / 3 iç hata`.
   `2` ise eksik paketi kur + yeniden dene.
4. `exec` ephemeral'dır: handshake'teki `sock`/`log` rezerve addır, dosya
   oluşmaz — `status`/`wait` deneme.
5. Keşif sırası: `--sock > $HBMON_SOCK > /tmp/hbmon-*.sock` (newest).

## Durum

v1 MVP: daemon, UDS JSON-RPC (`status`/`metrics`/`wait`/`kill`/`log_tail`/`shutdown`),
JSONL olay günlüğü, adaptif stall tespiti, OOM şüphesi (dmesg), dependency-missing
desen eşleme, timeout watchdog. Tasarımın tamamı için `HBMON-RFC.md` (ayrı doküman).
