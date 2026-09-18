[English](README.md) | **Türkçe**

# hbmon — Harness-Bağımsız Build Monitor

LLM kodlama ajanları için tek ikilik, sıfır-runtime-bağımlılık derleme izleyici.
Hangi harness kullanılırsa kullanılsın (OpenCode, Claude Code, Aider, …),
hangi OS olursa olsun çalışır — Linux'ta tam izleme (CPU/RSS/IO/FD),
macOS'ta süreç gözetimi + RSS/FD/yol (CPU best-effort), Windows'ta tam
izleme (Toolhelp+RSS/IO/handle; iphlpapi ile TCP sayımı, cmdline = exe yolu).

## Nasıl çalışır

Üç katmanlı ayrıştırma: **yaşam döngüsü** (setsid + double-fork → init'e reparent),
**iletişim** (sidecar JSONL + UDS üzerinden çekme-tabanlı sorgu, context'e log sızmaz),
**keşif** (env var + `/tmp/hbmon-*.sock` convention). Sınır: setsid
session/process-group'dan çıkarır ama cgroup'tan çıkarmaz; host/harness
cgroup politikası HBMon'un kontrol alanı dışındadır (detay [HBMON-RFC.md](./HBMON-RFC.md) §4.2.1).

## Platform desteği

| Alan | Linux | macOS | Windows |
|---|---|---|---|
| gözetim | setsid + double-fork | setsid + double-fork | DETACHED_PROCESS + Job Object |
| CPU / RSS / IO / FD | `/proc` ile tam | CPU best-effort; RSS/FD/yol libproc ile | Toolhelp ile CPU/RSS/IO/handle |
| ağ | `/proc` ile TCP sayımı | — | iphlpapi ile TCP sayımı (UDP —) |
| cmdline | tam | tam | yalnızca exe yolu |
| OOM şüphesi | dmesg | best-effort (dmesg formatı farklı) | yok (belgeli eksik) |
| transport | UDS, dosyalar `0600` | UDS, dosyalar `0600` | named pipe (current-user DACL, `0600` eşdeğeri) |
| stall / dep-missing / timeout | evet (heuristic) | evet (heuristic) | evet (heuristic) |

## Kurulum

```bash
cargo install hbmon   # crates.io, v0.1.0+
# veya kaynaktan: cargo build --release  # strip'li ikilik 2.38MB (v0.1.0, <5MB hedefi; CI `size` job'u izler)
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
hbmon exec --timeout-sec 600 -- make -j8  # watchdog: exit 124 (0 = kapalı)

# Öldür / kapat
hbmon kill --sock ... --signal TERM
hbmon shutdown --sock ...
```

## Ajanlar için (otomat hızlı yolu)

Bu bölüm LLM ajanları içindir; detaylı sözleşme [HBMON-RFC.md](./HBMON-RFC.md)'dedir, orayı ıskalamayın.
Tek sayfalık İngilizce protokol özeti: [PROTOCOL.md](PROTOCOL.md).
Salt CLI / skill / plugin+MCP tüketim desenleri: [USAGE-PATTERNS.md](USAGE-PATTERNS.md).

1. `hbmon watch --detach -- <cmd>` → stdout satır 1 = handshake JSON
   `{v,ev:"ready",uuid,sock,log}`. Satır 1'i parse et, `sock`'u sakla.
2. Poll: `hbmon status --sock $SOCK` (ucuz yoklama: `--compact`), veya bloklan:
   `hbmon wait --sock $SOCK --until done,failed,dep_missing,timeout,stall_suspect,oom_suspect`
   (alyas: `stalled`, `oom_killed`; bilinmeyen ad → `INVALID_UNTIL`, exit 3).
   Terminal state listede olmasa da döner (`woke_on` = kanonik ad) — bitmiş
   build daemon linger'ını beklemez.
3. Exit: `0 done / 1 failed / 2 dep-missing / 124 timeout / 137 oom / 3 iç hata`.
   `2` ise eksik paketi kur + yeniden dene.
4. `exec` ephemeral'dır: handshake'teki `sock`/`log` rezerve addır, dosya
   oluşmaz — `status`/`wait` deneme. `--timeout-sec N` watchdogludur
   (TERM → 5 s grace → KILL, exit 124; `0` = kapalı) ve dep taraması
   yalnız stderr'dedir (`watch` iki akışı `.out`'tan tarar).
5. Keşif sırası: `--sock > $HBMON_SOCK > /tmp/hbmon-*.sock` (newest);
   hepsini gör: `hbmon list` (salt-okunur; `--state running` / `--live-only` filtreler).
   Olaylar için `hbmon log --sock $SOCK --tail N` (tüm `.jsonl`'u cat'leme;
   `--event metric` yalnızca eşleşen olayları döndürür).
   Bayat dosyalar: çöken daemon `.sock/.pid/.jsonl/.out` bırakır.
   `hbmon cleanup`, `--older-than`'den eski dosyaları siler (varsayılan
   86400s) ama canlı daemon'un kardeş dosyalarına dokunmaz. `status`
   bağlantı hatası verirse socket bayattır — yeniden `watch` aç, eskisini
   kullanma.
6. Şüphe sinyalleri heuristic'tir, kanıt değil — bekleme, aksiyon al:
   | sinyal | anlamı | önerilen aksiyon |
   |---|---|---|
   | `stall_suspect` / `stalled` | eşik aşımında IO/CPU/child-spawn yok | `status --compact` + `log --event metric` ile teyit; gerçekten takıldıysa `kill`, yoksa beklemeye devam |
   | `oom_suspect` / `oom_killed` | OOM-killer izi eşleşti | aynen retry yapma — bellek kullanımını azalt, sonra retry |
   | `dep_missing` | eksik-bağımlılık deseni (exit 2) | paketi kur + retry |

## Kararlılık & SemVer

`0.x`: dondurulmuş yüzeye minor artışı ve `decisions.md` girdisi olmadan
breaking change yok. Dondurulmuş: handshake JSON (`v`, `ev:"ready"`,
`uuid`, `sock`, `log`; `exec` ek olarak `ephemeral:true`), exit-code
eşlemesi (0 done / 1 failed / 2 dep-missing / 124 timeout / 137 oom /
3 iç hata), `wait --until` kanonik adları (`done failed dep_missing
timeout stall_suspect oom_suspect`, alyaslar `stalled oom_killed`),
`status --compact` alan seti, IPC op'ları (`status metrics log_tail wait
kill shutdown`). Deneysel (değişebilir): metrik alanları, stall
eşikleri/skorları, `metrics`/`log_tail` çıktı detayları. Kilit:
`tests/drift.rs`.

TASK-047 (eklemeli, testle kilitli): `wait --until` terminal state'i listede
olmasa da döner (`woke_on` kanonik ad; OOM → `oom_suspect`),
`exec --timeout-sec` → `124` (TERM → 5 s grace → KILL; `0` = kapalı) ve
`--sock` socket-olmayan yolu silmek yerine reddeder, `exec` hem stdout hem
stderr'i tarar (`watch` ile parity).
TASK-048 (eklemeli, testle kilitli): `kill`/`shutdown`/`cleanup`
exit 1 döner başarısızlıkta (`killed:false` / `ok_shutdown:false` /
`failed>0`), `wait --poll-ms` sunucu tarafında en az 50 ms'ye clamplanır,
foreground `watch` bitişte anında döner (linger yok).

## Durum

v1 MVP: daemon, UDS JSON-RPC (`status`/`metrics`/`wait`/`kill`/`log_tail`/`shutdown`),
JSONL olay günlüğü, adaptif stall tespiti, OOM şüphesi (dmesg), dependency-missing
desen eşleme, timeout watchdog. Tasarımın tamamı için [HBMON-RFC.md](./HBMON-RFC.md) (ayrı doküman).
