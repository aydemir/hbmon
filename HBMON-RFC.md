# HBMon — Harness-Bağımsız, OS-Bağımsız Build Monitor
## Tasarım ve Referans Implementasyon Planı (RFC v0.1)

| Alan | Değer |
|---|---|
| **Durum** | Draft |
| **Tarih** | 2026-09-09 |
| **Yazar** | (kullanıcı) — orijinal mimari tasarım; Rust implementasyon planı Mavis tarafından |
| **Hedef kitle** | LLM harness geliştiricileri, build-orchestration yazanlar, derleme süresi yüksek projelerde ajan kullananlar |
| **Referans** | `aydemir/opencode-plugins` — DHS PTC + `build-mon.sh` + `opencode-settle-noticer` (varlık kanıtı) |
| **Dil** | Rust (stable, edition 2021) |
| **Hedef OS (v1)** | Linux + macOS |
| **Hedef OS (v2)** | Windows (named pipe) |
| **Lisans** | MIT OR Apache-2.0 |

---

## İçindekiler

1. [Abstract / Özet](#1-abstract--özet)
2. [Motivasyon ve Problem](#2-motivasyon-ve-problem)
3. [Tasarım — Üç Katmanlı Ayrıştırma](#3-tasarım--üç-katmanlı-ayrıştırma)
4. [Süreç Yaşam Döngüsü Ayrıştırması (Katman 1)](#4-süreç-yaşam-döngüsü-ayrıştırması-katman-1)
5. [Context-Penetrasyonsuz İletişim (Katman 2)](#5-context-penetrasyonsuz-iletişim-katman-2)
6. [Keşif Mekanizmaları (Katman 3)](#6-keşif-mekanizmaları-katman-3)
7. [Sağlık ve Canlılık Tespiti](#7-sağlık-ve-canlılık-tespiti)
8. [LLM Konuşma Protokolü — Tam Şema](#8-llm-konuşma-protokolü--tam-şema)
9. [Rust Krate Yapısı ve Mimari](#9-rust-krate-yapısı-ve-mimari)
10. [OS-Bağımsızlık — `ProcessInspector` Trait Sözleşmesi](#10-os-bağımsızlık--processinspector-trait-sözleşmesi)
11. [Kullanım Senaryoları](#11-kullanım-senaryoları)
12. [Harness-Bağımsızlık Kanıt Matrisi](#12-harness-bağımsızlık-kanıt-matrisi)
13. [Güvenlik, Sınırlamalar ve Yarış Koşulları](#13-güvenlik-sınırlamalar-ve-yarış-koşulları)
14. [Gelecek Çalışmalar (v1.5 / v2 / v3)](#14-gelecek-çalışmalar-v15--v2--v3)
15. [Açık Sorular ve Tartışma](#15-açık-sorular-ve-tartışma)
16. [Referanslar](#16-referanslar)

---

## 1. Abstract / Özet

LLM tabanlı kodlama ajanlarının en büyük operasyonel darboğazı **uzun derleme süreleridir**. Mevcut çözümler ya harness'a sıkı sıkıya bağımlıdır (plugin/hook sistemi, MCP, output contract), ya LLM context'ini kirletir (sürekli log stream), ya da OS'a bağımlıdır (`/proc` varsayımı, sadece Linux).

Bu RFC, **üç katmanlı ayrıştırma** (süreç yaşam döngüsü, iletişim, keşif) ile **harness ve OS'tan bağımsız** bir build monitor önerir. Hedef implementation Rust ile tek statik ikiliktir; Linux ve macOS v1'de desteklenir.

**Ana katkı:** LLM'in build sürecini **pasif izleme + çekme (pull) tabanlı sorgu** ile yönetebilmesi için sıfır-harness-bağımlılık sözleşmesi. Mevcut harness'ların (OpenCode, Claude Code, Aider, Cursor BG, Codex CLI, raw shell) hiçbir değişiklik olmadan HBMon'u kullanabilmesi.

**Referans varlık kanıtı:** `aydemir/opencode-plugins` reposundaki `build-mon.sh` + `opencode-settle-noticer` kombinasyonu bu felsefenin plugin/shim seviyesinde çalışan ispatlanmış halidir. Bu RFC, onu OS-bağımsız, harness-bağımsız, first-class bir Rust aracına yükseltir.

---

## 2. Motivasyon ve Problem

### 2.1 Problem — Harness-Sıkışmış Build Orchestration

Mevcut LLM harness'ları build süreçlerini şu şekillerde yönetir:

```
┌──────────────────────────────────────┐
│  LLM Harness (OpenCode, Claude…)    │
│  ├─ session timeout (örn. 5dk)      │
│  ├─ context window (örn. 200K tok)  │
│  ├─ tool allowlist                   │
│  ├─ child reaper (SIGHUP on exit)   │
│  └─ output contract (JSON schema)    │
└──────────────────────────────────────┘
        │
        ▼ (kısıtlar)
┌──────────────────────────────────────┐
│  Build process (make, cargo, npm)   │
└──────────────────────────────────────┘
```

Tipik sonuçlar:

- 4 dakikalık derleme ortasında LLM session kapanıyor
- Context sürekli log akışıyla doluyor
- Child süreçler session bitince ölüyor
- Kullanıcı "build bitti mi?" diye tekrar tekrar sormak zorunda kalıyor

### 2.2 Gözlemler — `aydemir/opencode-plugins` Vaka Çalışması

Gerçek deneyimlerden çıkarılan bulgular:

1. **`build-mon.sh` + `opencode-settle-noticer` kombinasyonu gerçek fayda sağlıyor.** Arka planda derleme olurken LLM boşa düşmüyor, başka işle meşgul olabiliyor.
2. **DHS PTC (context-saver) pattern'i çalışıyor.** Build çıktısı context'e sızmıyor; olay bitince özet push'lanıyor.
3. **Plugin/shim seviyesi, build monitoring için maksimumu veriyor — ama mimari minimumun biraz üstünde.** Asıl doğru yer harness core'unda first-class subsystem.
4. **Pratik kısıt:** Harness core'una PR bütçesi yok → plugin/shim seviyesinde kalmak bilinçli tercih.

### 2.3 Hedef

> **HBMon, hangi harness kullanılırsa kullanılsın, hangi OS'ta çalışırsa çalışsın (Linux/macOS), LLM'in derleme sürecini token harcamadan izlemesini, bitişinde uyarılmasını ve ihtiyaç halinde anlık durumunu sorgulamasını sağlayan tek-ikilik, sıfır-runtime-bağımlılık bir araçtır.**

### 2.4 Hedef-Dışı (Out of Scope, v1)

- v1'de Windows desteği yok (v2'ye)
- v1'de container-aware cgroup v2 ayrımı yok (v1.5)
- v1'de multi-build paralel izleme yok (v2)
- v1'de TUI/Web UI yok (v2)
- v1'de plugin sistemi (Lua/WASM) yok (v3 — over-engineering riski)

---

## 3. Tasarım — Üç Katmanlı Ayrıştırma

```
┌─────────────────────────────────────────────────┐
│                  LLM Harness                    │
│         (shell tool + file read + env)          │
└─────────────────────────────────────────────────┘
        │                              ▲
        │ spawn                        │ pull (status)
        │ env: HBMON_SOCK, HBMON_LOG   │
        ▼                              │
┌─────────────────────────────────────────────────┐
│              HBMon (daemon)                     │
│  ┌───────────────┐  ┌────────────────────────┐  │
│  │ Katman 1:     │  │ Katman 2:              │  │
│  │ Yaşam Döngüsü │  │ İletişim               │  │
│  │ (daemonize,   │  │ (UDS + JSONL + exit)   │  │
│  │  setsid)      │  │                        │  │
│  └───────────────┘  └────────────────────────┘  │
│  ┌─────────────────────────────────────────────┐ │
│  │ Katman 3: Keşif (env, ps scan, handshake)  │ │
│  └─────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────┘
        │
        ▼ (süreç ağacı izleme, OS API)
┌─────────────────────────────────────────────────┐
│  Build Process Tree                             │
│  (make → cc, cc, cc → ld)                      │
└─────────────────────────────────────────────────┘
```

| Katman | Problem | Çözüm |
|---|---|---|
| **1. Yaşam Döngüsü** | Harness kapanınca child ölüyor | Double-fork + setsid → init'e reparent |
| **2. İletişim** | Log LLM context'ini dolduruyor | Sidecar JSONL + UDS (pull) + exit code |
| **3. Keşif** | LLM monitor'ü nasıl bulur? | Env var + `/tmp/hbmon-*.sock` convention |

**Sonuç:** Monitor, harness'ın görüş alanının dışında yaşar; LLM ona standart yollarla (env, file, socket) ulaşır.

---

## 4. Süreç Yaşam Döngüsü Ayrıştırması (Katman 1)

### 4.1 Problem

LLM harness session kapanınca (kullanıcı cancel, network kesilmesi, session timeout) **child süreçlere SIGHUP gönderilir.** Bu, `nohup` veya `&` ile bile tam olarak önlenemez çünkü:

- Bazı harness'lar process group'u öldürür
- Bazıları cgroup bazlı reaping yapar
- Terminal kapatılınca controlling terminal'a bağlı süreçler SIGHUP alır

### 4.2 Çözüm — Daemonization Pattern

LLM'in shell tool'u HBMon'u şu şekilde spawn eder:

```bash
hbmon watch --pid $$ --detach -- make -j8
```

`hbmon` içinde (`src/daemon/mod.rs`):

```
1. fork()
   ↓
2. setsid()                          ← yeni session
   ↓
3. fork()                            ← bir kez daha
   ↓
4. close(STDIN), close(STDOUT), close(STDERR)
   ↓
5. /tmp/hbmon-<uuid>.pid yaz
   ↓
6. /tmp/hbmon-<uuid>.sock aç (UDS)
   ↓
7. /tmp/hbmon-<uuid>.jsonl oluştur
   ↓
8. exit(0) (parent LLM'e döner)
```

**Daemon HBMon:**

- Artık PID 1'e reparent edilmiş (init)
- Yeni session, controlling terminal yok
- STDIN/STDOUT/STDERR kapalı
- LLM exit code 0 alır, kendi işine devam edebilir
- Build bittiğinde daemon kendini temizler (pid/sock/log unlink + exit)

### 4.3 OS Davranış Matrisi

| OS | setsid | Double-fork | Notlar |
|---|---|---|---|
| Linux | `setsid(2)` | ✅ | Standart POSIX pattern |
| macOS | `setsid(2)` | ✅ | Aynı, `libc`'de mevcut |
| Windows | yok (Job Object + self re-spawn) | ✅ (ham `CreateProcessW`) | `DETACHED_PROCESS` + `CREATE_NEW_PROCESS_GROUP` + `BREAKAWAY`; stdio NUL; handle devralma yok (`bInheritHandles=FALSE`, TASK-006) |

### 4.4 Rust Implementation İskeleti

```rust
// src/daemon/mod.rs
pub fn daemonize() -> Result<()> {
    // İlk fork
    match unsafe { fork() } {
        -1 => return Err(Error::Fork),
        0 => {}, // child devam
        _ => std::process::exit(0), // parent çık
    }

    // setsid
    if unsafe { libc::setsid() } == -1 {
        return Err(Error::Setsid);
    }

    // İkinci fork
    match unsafe { fork() } {
        -1 => return Err(Error::Fork),
        0 => {}, // grandchild devam
        _ => std::process::exit(0), // first child çık
    }

    // stdio kapat
    unsafe {
        libc::close(0);
        libc::close(1);
        libc::close(2);
    }

    // Working directory güvenli yere
    std::env::set_current_dir("/").ok();

    // umask temizle
    unsafe { libc::umask(0o022); }

    Ok(())
}
```

### 4.5 Süreç Öldürme — `kill` Subcommand

LLM veya kullanıcı isterse:

```bash
hbmon kill --sock /tmp/hbmon-abc.sock --signal SIGTERM
# veya
hbmon kill --pid 12345 --signal SIGTERM
```

Bu UDS üzerinden `{"op":"kill","signal":15}` gönderir; daemon `kill(-pgid, sig)` ile **tüm süreç grubunu** sonlandırır.

**Windows notu (TASK-006):** graceful grup sinyali yok — `Term` de `Kill` de
`TerminateJobObject` ile sonlanır (build `exit 1` → `failed` eşlenir).
Job atanamazsa fallback: Toolhelp ağaç yürüyüşü + yaprak-önce
`TerminateProcess` (`kill_tree`).

---

## 5. Context-Penetrasyonsuz İletişim (Katman 2)

### 5.1 Tasarım İlkesi

> **HBMon, LLM'e asla doğrudan veri göndermez (push). LLM, ihtiyacı olduğunda çeker (pull).**

Bu iki nedenle:

1. LLM context'i değerli — her log satırı token demek
2. Harness'ların çoğu push event'i kabul etmez (subprocess model)

### 5.2 Üç Kanal

#### 5.2.1 Sidecar JSONL Event Log

```
/tmp/hbmon-<uuid>.jsonl
```

**Yapı:** Her satır tek bir olay, newline-delimited JSON. Append-only.

**Örnek:**

```jsonl
{"ts":"2026-09-09T00:00:00Z","v":1,"ev":"ready","uuid":"abc","sock":"/tmp/hbmon-abc.sock","log":"/tmp/hbmon-abc.jsonl","root_pid":1234,"cmd":"make -j8"}
{"ts":"2026-09-09T00:00:01Z","v":1,"ev":"metric","uuid":"abc","cpu":1.2,"rss_mb":8,"io_r":0,"io_w":0,"fds":5}
{"ts":"2026-09-09T00:00:05Z","v":1,"ev":"child_spawn","uuid":"abc","pid":1235,"ppid":1234,"cmd":"cc -c foo.c"}
{"ts":"2026-09-09T00:00:30Z","v":1,"ev":"stall_suspect","uuid":"abc","reason":"no_io_no_cpu","idle_sec":28.0,"threshold_sec":30.0}
{"ts":"2026-09-09T00:01:42Z","v":1,"ev":"exit","uuid":"abc","pid":1234,"code":0,"duration_sec":102.3,"state":"done"}
```

**LLM tüketimi:**

```bash
tail -n 20 /tmp/hbmon-abc.jsonl
```

**Avantajlar:**

- Kalıcı (build sonrası incelenebilir)
- Append-only, yarış koşulu yok
- LLM herhangi bir file read tool'uyla okur
- Format standart (JSONL, jq ile parse)

**Rotasyon:** 100MB üzerine çıkarsa eski satırlar FIFO silinir (max 100MB cap).

#### 5.2.2 Unix Domain Socket — JSON-RPC Over UDS

```
/tmp/hbmon-<uuid>.sock
```

**Bağlantı modeli:** LLM her sorguda bağlanır, cevabı alır, kapatır. Persistent connection yok (state machine basit kalsın).

**Protokol:** UTF-8 newline-delimited JSON request, newline-delimited JSON response.

##### Operation: `status`

**Request:**
```json
{"v":1,"op":"status","id":"req-1"}
```

**Response:** (tam şema Section 8'de.)

##### Operation: `metrics` (Hafif)

Sadece anlık metrikleri döner (tree yok, hızlı):

```json
{"v":1,"op":"metrics","id":"req-2"}
→ {"v":1,"id":"req-2","ok":true,"cpu":12.3,"rss_mb":412,"io_r":88,"io_w":21,"fds":14,"net_tcp":3}
```

##### Operation: `wait` (Bloklamalı)

Build bitene kadar bloklanır. LLM'in "şimdi sorma, hazır olunca söyle" demesinin yolu.

**Request:**
```json
{"v":1,"op":"wait","id":"req-3","timeout_sec":300,"poll_ms":500}
```

**Response (timeout'a kadar bloklanır, sonra):**
```json
{
  "v": 1,
  "id": "req-3",
  "ok": true,
  "state": "done",
  "code": 0,
  "duration_sec": 47.3,
  "exit_event": { "...": "..." }
}
```

**Davranış:**

- `timeout_sec` dolunca son snapshot'ı döner (state = "running" veya "timeout")
- Build erken biterse hemen döner
- 500ms poll aralığı ile süreç ağacı kontrol edilir

**Erken dönüş (`until`):** `until` listesi verilirse (`done`, `dep_missing`,
`stall_suspect`, `oom_suspect`, …) çağrı ilk eşleşen sinyalde `woke_on`
alanıyla döner; polling ve harness eklentisi gerekmez. Bu senkron
çoğullamalı beklemedir, async push değil (`until` yoksa yalnızca terminal
state'lerde dönülür).

##### Operation: `kill`

```json
{"v":1,"op":"kill","id":"req-4","signal":15}
→ {"v":1,"id":"req-4","ok":true,"killed":true}
```

##### Operation: `log_tail` (Son N Satır)

```json
{"v":1,"op":"log_tail","id":"req-5","n":20}
→ {"v":1,"id":"req-5","ok":true,"lines":["...","..."]}
```

##### Operation: `shutdown`

Daemon'ı kapatır (build devam etmez):

```json
{"v":1,"op":"shutdown","id":"req-6","force":false}
→ {"v":1,"id":"req-6","ok":true}
```

`force=false` ise önce build'e `SIGTERM` gönderir, 5s bekler, hâlâ yaşıyorsa `SIGKILL`.

#### 5.2.3 Exit Code (Son Özet)

LLM `hbmon wait` veya `hbmon exec` kullanırsa, daemon LLM'in parent'ı olarak çalışır ve build bittiğinde exit code ile özet verir:

| Exit | Anlam | Tetikleyici |
|---|---|---|
| `0` | Başarı | Build process `exit(0)` |
| `1` | Build hatası | `exit(≠0)` veya pattern eşleşmedi |
| `2` | Dependency missing | Pattern match: "command not found", "ModuleNotFoundError", vb. |
| `3` | Genel süreç hatası | Beklenmeyen (crash, signal) |
| `124` | Timeout | `--timeout-sec` aşıldı |
| `130` | SIGINT (Ctrl+C) | Kullanıcı iptal — implement edilmedi |
| `137` | OOM killer | dmesg/cgroup tespiti |
| `143` | SIGTERM | Daemon tarafından öldürüldü — implement edilmedi |

> Not (v1 gerçekleşmesi): stall exit koduna yansımaz — `Stalled` yalnızca
> `state` + `stall_suspect` event'idir, build çıkış kodu aynen iletilir.
> `147` kodu bu RFC'nin eski taslağındandı, implement edilmedi.

**Avantaj:** Tüm harness'lar exit code'u anlar. Ek contract gerekmez.

### 5.3 Kanal Seçim Rehberi (LLM İçin)

```
LLM ne yapmak istiyor?                    Kanal
─────────────────────────────────────────────────────────
Derleme bitene kadar bekle, sonra devam et → exit code
Bitti mi diye uzun poll                   → UDS wait
Şu an ne aşamada, hızlıca bak            → UDS status
Son olayları log olarak oku              → JSONL tail
Build'i öldür                             → UDS kill
```

### 5.4 Wire Format — Ortak Kurallar

- **Versiyon:** Her mesaj `"v":1` ile başlar (schema evolution için)
- **Newline-delimited:** Her mesaj tek satır (`\n` ile biter)
- **UTF-8:** Tüm string alanlar UTF-8
- **Timestamp:** ISO 8601 UTC (`"2026-09-09T00:00:00Z"`)
- **Büyük sayılar:** MB cinsinden (integer)
- **PID:** Unsigned 32-bit integer
- **ID:** Request/response eşleştirmesi için client-tarafı UUID/string

---

## 6. Keşif Mekanizmaları (Katman 3)

### 6.1 Üç Katmanlı Keşif

#### Katman A — Açık Env Değişkeni (En Sağlam)

```bash
# LLM shell tool:
HBMON_SOCK=/tmp/hbmon-abc.sock \
HBMON_LOG=/tmp/hbmon-abc.jsonl \
HBMON_UUID=abc \
  hbmon watch --pid $$ --detach -- make -j8
```

LLM `HBMON_SOCK`'u sonraki sorgularda kullanır.

**Avantaj:** Deterministik, race condition yok, harness bağımsız.

**Dezavantaj:** LLM'in environment'ı saklaması gerekir (çoğu harness yapar).

#### Katman B — Convention-Based Scan (Fallback)

LLM, env'i kaybetmişse:

```bash
ls -t /tmp/hbmon-*.sock 2>/dev/null | head -1
```

En yeni socket = en son spawn edilen monitor.

**Dikkat:** Birden fazla aktif build varsa yanlış monitor seçilebilir. Bu yüzden **UUID-zorunlu** kullanım önerilir.

#### Katman C — Handshake (Sarmalama Modu)

```bash
hbmon exec -- make -j8
```

İlk satırda daemon handshake yapar (stdout'a):

```json
{"v":1,"ev":"ready","uuid":"abc","sock":"/tmp/hbmon-abc.sock","log":"/tmp/hbmon-abc.jsonl"}
```

Sonra build çıktısı gelir.

**LLM parse eder:** İlk satır handshake, kalanı build output. (LLM zaten `head -n 1` + kalan olarak ayırabilir.)

### 6.2 Hangisi Tercih Edilmeli?

| Kullanım | Önerilen |
|---|---|
| LLM shell tool'u **bir kez** komutu çalıştırıp bırakacaksa | Katman C (handshake) |
| LLM **uzun süre** başka iş yapıp sonra sorgulayacaksa | Katman A (env) |
| LLM env'i kaybetmişse / multi-build | Katman B (scan) — ama dikkatli |

---

## 7. Sağlık ve Canlılık Tespiti

### 7.1 Durum Makinesi

```
        spawn
          │
          ▼
      ┌───────┐
      │running│◀─────────────┐
      └───┬───┘              │
          │                  │
          ├─ child exit OK ───┤
          │                  │
          ├─ child exit fail │
          │       │          │
          │       ▼          │
          │  ┌──────┐        │
          │  │failed│        │
          │  └──────┘        │
          │                  │
          ├─ idle > 3×p95 ───┤
          │       │          │
          │       ▼          │
          │  ┌───────┐       │
          │  │stalled│       │
          │  └───┬───┘       │
          │      │ resume    │
          │      └───────────┘
          │
          ├─ OOM killed
          │       │
          │       ▼
          │  ┌──────────┐
          │  │oom_killed│
          │  └──────────┘
          │
          ├─ dep pattern match
          │       │
          │       ▼
          │  ┌────────────┐
          │  │dep_missing │
          │  └────────────┘
          │
          └─ timeout
                  │
                  ▼
            ┌──────────┐
            │ timeout  │
            └──────────┘
```

### 7.2 Stall / Askı Tespiti — Adaptif Eşik

**Sorun:** Sabit eşik yanlış. 5 saniyelik `cc -E` ile 600 saniyelik `lld -flto` aynı kurala giremez.

**Çözüm:**

1. İlk **60 saniye** gözlem yapılır (warmup)
2. Bu sürede her "sessiz an" kaydedilir (CPU=0 + I/O=0 + new child=0 olduğu anlar)
3. **Rolling p95** idle süresi hesaplanır
4. Stall kuralı: `current_idle > 3 × p95_idle` VE `idle > 30s` (mutlak alt sınır)

**Edge case'ler:**

- **Network-bound build** (`go mod download`, `cargo fetch`): I/O aktif ama CPU düşük. Kural: network I/O = "alive" sinyali
- **Link aşaması** (lld, ld): CPU yüksek ama dosya yazmıyor. Kural: CPU > 0 = "alive" sinyali
- **Çok kısa build** (< 10s): Stall tespiti devre dışı (false positive önleme)

**Çıktı:**

```json
{"ts":"...","v":1,"ev":"stall_suspect","uuid":"abc","reason":"no_io_no_cpu","idle_sec":47.2,"threshold_sec":42.0,"p95_idle":14.0}
```

**Resume:**

```json
{"ts":"...","v":1,"ev":"stall_resolved","uuid":"abc","lasted_sec":47.2}
```

### 7.3 OOM Killer Tespiti

**Linux:** `/var/log/kern.log` veya `journalctl -k -n 100` üzerinden pattern:

```
Out of memory: Killed process 1234 (cc) total-vm:...
```

veya

```
oom-kill:constraint=CONSTRAINT_MEMCG,...
```

**macOS:** `log show --predicate 'eventMessage contains "jetsam"' --last 5m` (memory pressure killer).

**Tetikleme:** OOM-killed PID bizim süreç ağacımızdaysa → `state = "oom_killed"`, exit 137.

### 7.4 Dependency Missing Tespiti

**Yöntem:** Stderr pattern matching + regex set.

**Varsayılan pattern seti (v1):**

| Pattern | Kategori | Dil/Sistem |
|---|---|---|
| `command not found` | `dep_missing:command` | POSIX shell |
| `No such file or directory` | `dep_missing:path` | POSIX |
| `Cannot find module` | `dep_missing:npm` | Node.js |
| `ModuleNotFoundError` | `dep_missing:pymod` | Python |
| `error: failed to parse manifest` | `dep_missing:crates` | Cargo |
| `error: package ID specification` | `dep_missing:cabal` | Haskell |
| `fatal error: .*: No such file` | `dep_missing:header` | C/C++ |
| `Could not find .* in` | `dep_missing:lib` | ld linker |
| `Package .* was not found` | `dep_missing:pkg` | Debian/RHEL pkg-config |
| `error: linking with \S+ failed` | `dep_missing:link` | Rust |

**Tetikleme:** Pattern eşleşirse → `state = "dep_missing"`, `ev = "dep_missing"`, exit 2.

**Kullanıcı genişletmesi:** `~/.config/hbmon/patterns.json` (yoksa `$XDG_CONFIG_HOME/hbmon/patterns.json`) ile ek pattern'ler — v1'de (RFC taslağındaki `patterns.toml` yerine JSON: sıfır-dep ilkesi, TASK-001).

### 7.5 Timeout

**CLI flag:** `--timeout-sec 600`

**Davranış:** Süre aşılırsa `SIGTERM` → 5s → `SIGKILL`. State = "timeout", exit 124.

---

## 8. LLM Konuşma Protokolü — Tam Şema

### 8.1 `status` Response (Tam)

`status` yanıtı şu alanları içerir: `v`, `id`, `ok`, `state` (`running` | `stalled` | `oom_killed` | `done` | `failed` | `dep_missing` | `timeout`), `uuid`, `root_pid`, `root_cmd`, `started_at`, `elapsed_sec`, `metrics` (`cpu_pct`, `rss_mb`, `io_read_mb`, `io_write_mb`, `fds_open`, `net_tcp`, `net_udp`), `tree` (pid/cmd/cpu/rss_mb/state/children), `health` (`stall_score`, `threshold_sec`, `last_io_at`, `last_cpu_nonzero_at`, `last_child_spawn_at`), `log_tail`, `last_event`.

> Not (v1 gerçekleşmesi): `eta_sec` şemada opsiyonel olarak durur ama daemon
> şu an dönmüyor — gelecek çalışma (v1.5+).

### 8.2 Alan Açıklamaları

| Alan | Tip | Açıklama |
|---|---|---|
| `v` | u8 | Protokol versiyonu (şu an 1) |
| `id` | string | Request/response eşleştirme |
| `ok` | bool | Hata durumu (false ise `err` alanı) |
| `state` | enum | `running` \| `stalled` \| `oom_killed` \| `done` \| `failed` \| `dep_missing` \| `timeout` |
| `uuid` | string | Monitor unique ID |
| `root_pid` | u32 | İzlenen kök süreç |
| `root_cmd` | string | Kök süreç komut satırı |
| `started_at` | timestamp | Daemon başlangıç zamanı |
| `elapsed_sec` | float | `now - started_at` |
| `eta_sec` | float? | Tahmini kalan süre — v1'de dönülmüyor (gelecek) |
| `metrics.cpu_pct` | float | Süreç ağacı toplam CPU % |
| `metrics.rss_mb` | u32 | Toplam RSS (MB) |
| `metrics.io_read_mb` | u32 | Toplam okuma I/O (build başından beri) |
| `metrics.io_write_mb` | u32 | Toplam yazma I/O |
| `metrics.fds_open` | u32 | Açık dosya tanımlayıcı sayısı |
| `metrics.net_tcp` | u32 | Aktif TCP bağlantı sayısı |
| `metrics.net_udp` | u32 | Aktif UDP "bağlantı" sayısı |
| `tree` | array | Süreç ağacı (root + çocuklar) |
| `health.stall_score` | float | 0.0 (aktif) — 1.0 (tamamen askı) |
| `health.last_io_at` | timestamp | Son I/O aktivitesi |
| `health.last_cpu_nonzero_at` | timestamp | Son CPU>0 anı |
| `log_tail` | array<json string> | JSONL log'un son N satırı (parse edilmemiş) |
| `last_event` | object | En son olay (özet) |

### 8.3 Event Tipleri (Tam Liste)

| Event | Tetikleyici | Ek Alanlar |
|---|---|---|
| `ready` | Daemon handshake | `sock`, `log`, `root_pid`, `cmd` |
| `spawn` | Root süreç başladı | `pid`, `cmd`, `ppid` |
| `child_spawn` | Yeni child | `pid`, `ppid`, `cmd` |
| `child_exit` | Child bitti | `pid`, `code`, `duration_sec` |
| `exit` | Root bitti | `pid`, `code`, `duration_sec`, `state` |
| `metric` | Periyodik ölçüm | `cpu`, `rss_mb`, `io_r`, `io_w`, `fds` |
| `stall_suspect` | Stall heuristic tetiklendi | `reason`, `idle_sec`, `threshold_sec` |
| `stall_resolved` | Stall sona erdi | `lasted_sec` |
| `oom_suspect` | OOM killer | `pid`, `killed_by` |
| `dep_missing` | Pattern match | `pattern_id`, `category`, `match_text` |
| `signal` | Sürece signal gönderildi | `pid`, `sig` |
| `health_change` | State transition | `from`, `to` |
| `timeout` | Timeout aşıldı | `elapsed_sec`, `limit_sec` |
| `shutdown` | Daemon kapanıyor | `reason` |

### 8.4 Hata Response

```json
{
  "v": 1,
  "id": "req-1",
  "ok": false,
  "err": {
    "code": "BUILD_NOT_FOUND",
    "message": "root process 1234 has already exited"
  }
}
```

**Hata kodları:**

| Code | Anlam |
|---|---|
| `INVALID_REQUEST` | JSON parse / schema hatası |
| `UNKNOWN_OP` | Bilinmeyen operation |
| `BUILD_NOT_FOUND` | İzlenen süreç artık yok |
| `ALREADY_SHUTDOWN` | Daemon kapanmış — implement edilmedi (dispatch'te yok) |
| `TIMEOUT` | `wait` timeout aşıldı — implement edilmedi (`wait` bunun yerine `ok:true` + `timeout:true` döner) |
| `INTERNAL` | Beklenmeyen internal hata — implement edilmedi |

---

## 9. Rust Krate Yapısı ve Mimari

### 9.1 Crate Yapısı (Tek Crate, Modüler)

```
hbmon/
├── Cargo.toml
├── README.md
├── src/
│   ├── main.rs              ← CLI entry (clap derive)
│   ├── lib.rs               ← kütüphane re-exports
│   ├── cli/                 ← watch, status, wait, exec, kill, shutdown
│   ├── daemon/              ← daemonize, lifecycle, pidfile, signals
│   ├── ipc/                 ← UDS server, JSON-RPC dispatcher, codec
│   ├── proc/                ← ProcessInspector trait + Linux/macOS impl
│   ├── metrics/             ← cpu, mem, io, fds, net
│   ├── health/              ← state, stall, oom, dep_missing, timeout
│   ├── eventlog/            ← JSONL appender (rotation ile)
│   ├── platform/            ← cfg(target_os) dispatch
│   └── util/                ← uuid, time
├── tests/
│   ├── integration.rs       ← uçtan uca: watch/status/wait/kill
└── docs/
    └── examples/
```

### 9.2 Bağımlılık İlkeleri

- **Neden `tokio` yok:** Tek ikilik küçük olmalı (hedef < 5MB). Thread-per-task yeterli; async runtime ~2MB ekler.
- **Neden `sysinfo` yok:** Child ağacı, OOM detection, stall heuristic vermiyor; kendi trait katmanı daha testable.
- **Neden `nix` yok:** Birkaç syscall için `libc` doğrudan yeterli.

### 9.3 Modül Bağımlılık Grafiği

```
cli ──→ daemon ──→ proc ──→ platform
            │         │
            ├──→ ipc ─┤
            ├──→ eventlog
            ├──→ metrics
            ├──→ health ──→ proc
            └──→ util
```

**Kural:** Üst katman alt katmana bağımlı; tersi asla. Circular dependency yok.

---

## 10. OS-Bağımsızlık — `ProcessInspector` Trait Sözleşmesi

```rust
pub trait ProcessInspector: Send + Sync {
    fn list_children(&self, pid: u32) -> Result<Vec<u32>, String>;
    fn metrics(&self, pid: u32) -> Result<Metrics, String>;
    fn cmdline(&self, pid: u32) -> Result<String, String>;
    fn tree(&self, pid: u32) -> Result<TreeNode, String>;
    fn is_alive(&self, pid: u32) -> bool;
    fn bulk_metrics(&self, pids: &[u32]) -> Result<HashMap<u32, Metrics>, String>;
}
```

- **Linux:** doğrudan `/proc` parsing (bağımlılıksız, best-effort).
- **macOS:** `libproc` FFI (`proc_listpids`, `proc_pidinfo`); metrikler v1'de best-effort.
- **Windows (TASK-006 ile gerçeklendi):** ham Win32 FFI, yeni crate yok —
  ağaç için Toolhelp snapshot, RSS için `GetProcessMemoryInfo`, CPU ham
  zaman için `GetProcessTimes` (centisecond → `CpuTracker` aynen),
  IO sayaçları için `GetProcessIoCounters`, fd için `GetProcessHandleCount`;
  transport named pipe (`\\.\pipe\hbmon-<uuid>`), wire format v1 değişmez.
  Belgeli eksikler: `net_tcp/net_udp` = 0, `cmdline` = exe yolu (argv değil).

CPU yüzdesi her zaman delta gerektirir → `CpuTracker` (jiffies farkı / geçen süre, 100Hz varsayımı).

---

## 11. Kullanım Senaryoları

### 11.1 Senaryo 1 — Basit Spawn + Bekleme

```bash
hbmon exec -- make -j8
# → exit 0 (başarı) veya 1 (fail) veya 2 (dep missing)
```

### 11.2 Senaryo 2 — Aktif İzleme (LLM Meşgul)

```bash
hbmon watch --detach --uuid $U -- make -j8   # hemen döner
# ... LLM başka iş yapar ...
hbmon status --sock /tmp/hbmon-$U.sock
hbmon wait --sock /tmp/hbmon-$U.sock --timeout 600
```

### 11.3 Senaryo 3 — Hata Erken Tespit

LLM periyodik `log_tail` ile `dep_missing` olayını görürse derleme bitmeden müdahale eder.

### 11.4 Senaryo 4 — Birden Fazla Build (v2'de first-class, v1'de scan)

```bash
ls -t /tmp/hbmon-*.sock | head -2
```

### 11.5 Senaryo 5 — `build-mon.sh` Migrasyonu

| `build-mon.sh` | `hbmon` |
|---|---|
| `pgrep -P $PID` polling | `hbmon wait` (bloklamalı) |
| Custom event parsing | `hbmon status --format json` |
| OS-specific (pgrep, ps) | Cross-OS trait |
| Stall/OOM/dep-missing yok | Hepsi dahil |

---

## 12. Harness-Bağımsızlık Kanıt Matrisi

| Harness | Shell tool | Env var | File read | HBMon uyumlu? |
|---|---|---|---|---|
| OpenCode / Claude Code / Aider / Cursor BG / Codex CLI / Continue.dev | ✅ | ✅ | ✅ | ✅ |
| Raw LLM + bash | ✅ | ✅ | ✅ | ✅ |

Gerekli minimum: **shell komutu + dosya okuma.** İkisi de tüm modern harness'larda mevcut.

---

## 13. Güvenlik, Sınırlamalar ve Yarış Koşulları

- **Dosya izinleri:** pid/sock/jsonl/out hepsi `0600` (Windows: ACL
  varsayılanı + `%TEMP%`; pipe `\\.\pipe\hbmon-<uuid>`); symlink açılışta reddedilir (`O_EXCL` + sahiplik kontrolü).
- **Path injection:** `..` içeren path'ler reddedilir.
- **Resource limit:** daemon ~10MB RAM, idle <%1 CPU hedefi.
- **Çift spawn:** canlı socket varsa `MONITOR_ALREADY_EXISTS`.
- **Zombie:** daemon exit sonrası 60s linger (wait/status için), sonra pid/sock temizliği; JSONL + .out kalır. `hbmon cleanup` ile eski dosyalar silinir.
- **Erken ölüm (<100ms):** exit code ile bildirilir, sorguya gerek yok.
- **Bilinen sınırlar:** derin ağaç (>1000 süreç) cap'li; metrik 1Hz varsayılan; JSONL 100MB FIFO cap; container PID namespace v1.5'e.

---

## 14. Gelecek Çalışmalar (v1.5 / v2 / v3)

### 14.1 v1.0 (MVP) — Durum

- [x] Linux tam izleme + macOS süreç gözetimi
- [x] Daemonization, UDS JSON-RPC, JSONL, exit mapping
- [x] Stall/OOM/dep-missing/timeout (ETA gelecek — v1'de `eta_sec` dönülmüyor)
- [x] CLI: watch, status, wait, exec, kill, shutdown, cleanup
- [x] 41 unit + 7 integration test (2026-09-09, `cargo test --locked -j2` yeşil)

### 14.2 v1.5

- [x] Container-aware (cgroup v2) — v1'e çekildi (graceful fallback, TASK-öncesi)
- [x] Custom patterns (`~/.config/hbmon/patterns.json`; taslaktaki TOML yerine JSON — sıfır-dep, TASK-001) — v1'e çekildi
- [ ] `journalctl` OOM desteği, supervised mode
- [ ] TUI (Ratatui)

### 14.3 v2

- [x] Windows portu — TASK-006 (named pipe transport + Win32 inspector +
  Job-Object detach/kill), `windows-latest` CI'da; 45 unit + 7 integration yeşil
- [ ] Multi-build first-class (v1'de scan ile idare)
- [ ] Prometheus exporter, Web UI

### 14.4 v3

- [ ] Plugin (Lua/WASM — dikkatli), distributed monitoring, ML ETA

### 14.5 Yapılmayacaklar

- ❌ Build orchestration, cache katmanı, LLM model seçimi, IDE entegrasyonu

---

## 15. Açık Sorular ve Tartışma

1. Stall eşikleri build tipine göre (compile/link/network) özelleşmeli mi?
2. ETA: basit EMA mi, faz-ağırlıklı mı?
3. Default metric interval 1Hz yeterli mi?
4. Log rotation cap kullanıcı-tanımlı mı olmalı?
5. OOM için dmesg yeterli mi (container yetkisi)?
6. Stall skoru float mi boolean mi?
7. Lisans: dual MIT/Apache-2.0 seçildi (2026-09-09).
8. Test stratejisi: unit (mock'suz, gerçek regex/stall/CPU) + integration (gerçek daemon) benimsendi.
9. v1.0.1 bakımı 2026-09-09'da kapatıldı: TASK-006 (artımlı dep-scan), TASK-007 (exec ephemeral handshake), TASK-008 (ölü bağımlılık + bu RFC'deki drift notları).

---

## 16. Referanslar

- `aydemir/opencode-plugins` — `build-mon.sh`, `opencode-settle-noticer`, DHS PTC (varlık kanıtı)
- POSIX: `setsid(2)`, `fork(2)`; Linux: `proc(5)`, `oom(7)`; macOS: `libproc.h`, `proc_pidinfo(3)`
- Rust: `clap` 4, `serde`/`serde_json` 1, `libc` 0.2, `regex` 1, `once_cell` 1, `rand` 0.8
- Build sistemleri: Recursive Make Considered Harmful (Miller, 1997); Build Systems à la Carte (Mokhov et al., 2018)

---

**Doküman sonu. v0.1 Draft + v1 MVP gerçekleşme notları. Geri bildirim ve revizyon için açık.**
