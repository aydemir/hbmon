# Karar Kaydı

## 2026-09-09 — Linux ARM doğrulandı (Termux proot Debian)

hbmon Android Termux proot Debian'da derlenip test edildi (kullanıcı
raporu). `/proc` parsing, double-fork daemon, UDS — hepsi proot altında
çalışıyor. Bu, CI'a eklenen `ubuntu-24.04-arm` job'unun ampirik karşılığı;
teorik değil, sahada doğrulanmış platform.

## 2026-09-09 — Windows koşulu kapandı (TASK-006)

`decisions.md:9` Windows'u koşullu bırakmıştı: "yalnızca CI windows job +
test edecek cihazla". İkisi de sağlandı (bu Windows makinede uçtan uca
doğrulama + CI `windows-latest`). Sıfır yeni bağımlılık ilkesi korundu
(ham `extern "system"` FFI, `macos.rs` emsali); `protocol.rs` değişmedi;
bulunan gerçek bug (handshake JSON'unda kaçışsız `\`, `exec`/`watch`)
serde_json serialize ile kapatıldı; kritik bulgu: std::Command
null-stdio + creation_flags bile capture pipe'larını devralır → detach
ham `CreateProcessW` + `bInheritHandles=FALSE` ile yapılır (yoksa
`.output()`-tarzı caller daemon ölene dek EOF göremez).

## 2026-09-09 — Dış uzman önerileri eleme (Grok review)

HBMON-RFC.md repoda yokken verilen öneriler hedef kilidine çarpıldı:

- **Yap:** custom patterns, release+crates.io, dogfooding+demo, plugin migrasyonu
  → TASK-001..004.
- **Yapma:** TUI/Web UI (ajan terminal izlemez, binary şişer), Windows
  (bu cihazdan doğrulanamaz), plugin sistemi (RFC'nin over-engineering
  uyarısı geçerli), Prometheus/distributed/ML-ETA (spekülatif).
- **Koşullu:** Windows yalnızca CI windows job + test edecek cihazla;
  plugin yalnızca 3 bağımsız ekipten aynı talep gelirse.

## 2026-09-09 — Altyapı

- Task kanban (`tasks/`), jq-sorgulanabilir `index.json`lar ve `.codegraph/`
  indexi opencode-plugins konvansiyonuyla eklendi.

## 2026-09-09 — Uyandırma kararı (TASK-005)

- Gerçek async push alıcısız imkânsız; plugin ise hedefi deler.
- Karar: `wait --until` — bloklanan çağrının izlenen sinyalde erken
  dönüşü. Polling (`status`/`log_tail`) korunur, `until` yoksa davranış
  değişmez. Dürüst adıyla senkron çoğullamalı bekleme.

## 2026-09-10 — Wrapper-push tezi çürütüldü (TASK-013)

- Öneri: `hbmon` yerine geçen tokio+reqwest wrapper, her stdout/stderr
  satırını `error/fail/event/state` contains ile `localhost:3000`'e POST'lasın.
- Canlı deney (uuid=bd8e75b3): 5 satır `.out` → wrapper 4 POST (2'si gürültü:
  `state update tick`, `event fired`); 4 olay `.jsonl` → sidecar 2 PUSH
  (`dep_missing`, `exit`). `wait --until` `woke_on=dep_missing`, `code=2`.
- Hüküm: satır-bazı wrapper-push bol keseden; kilit (tek binary, sıfır
  runtime bağımlılık, pull-tabanlı, context ekonomisi) korunur. Push şartsa
  core'a dokunmayan JSONL-tail sidecar, olay-bazı filtreyle yapılır.

## 2026-09-13 — crates.io durumu + sonraki tag (TASK-033)

- `cargo publish --dry-run` temiz (67 dosya, verify+compile OK).
- `hbmon@0.1.1` crates.io'da zaten yayında — aynı numaraya yeniden
  publish yok (registry değişmezliği).
- Sonraki tag: `v0.1.2` (README Stability/matrix/suspect + PROTOCOL.md
  non-breaking doküman ekleri). Breaking olursa minor + bu dosyaya girdi.
- `cargo publish` otomatik release'e bağlanmadı: geri alınamaz işlemde
  insan kapısı şart. Prosedür `docs/RELEASE.md`'de, manuel tetik
  `.github/workflows/publish-crate.yml`'de (`CRATES_IO_TOKEN`).
- README install satırı doğrulandı (`cargo install hbmon`) — `--git`
  karışıklığı iddiası asılsız, değişiklik yok.

## 2026-09-13 — RFC İngilizce eşdeğeri (TASK-035)
- `HBMON-RFC-EN.md`: `HBMON-RFC.md`'nin birebir çevirisi (73 başlık,
  68 ``` çiti, kod blokları bayt-ayni). Çelişmede Türkçe asıl normatiftir.
- `README.md` (EN) linkleri EN belgeye çevrildi; `README.tr.md` Türkçe
  belgede kaldı. Frozen terimler çevrilmedi (op/state adları, exit kodları).

## 2026-09-14 — Windows graceful TERM imkânsız (TASK-042, won't-fix)

- TASK-006/M3-13 `TERM→GenerateConsoleCtrlEvent(CTRL_BREAK)` planlamıştı.
  Mimari kanıt: daemon `DETACHED_PROCESS` + console yok
  (`detach.rs:windows_detach`), child console devralmaz →
  `GenerateConsoleCtrlEvent` hedefe ulaşamaz (yalnızca console'lu grup).
  Console'lu spawn'a dönüş detach'ı deler → kilit ihlali.
- Hüküm: Term=Kill=terminate kalır (RFC §4.5'te zaten belgeli).
  `signal.rs:windows_kill` yorumu kapatıldı; bug template'e
  "graceful TERM yok" kutusu eklendi.

## 2026-09-14 — Windows cmdline/OOM araştırma (TASK-043, won't-fix)

- `cmdline` tam argv: PEB okuma (`NtQueryInformationProcess` +
  `ReadProcessMemory`) WOW64/protected süreçlerde fragile, WMI ise COM
  maliyetiyle kilit ruhuna (küçük binary, sıfır bağımlılık) aykırı →
  exe yolu kalır (macOS "path, not argv" emsali).
- OOM: EventLog (`Resource-Exhaustion`) okuma ağır + yetki ister,
  "best-effort, never fatal" ilkesini yorar → `oom.rs` boş-dönüş kalır.
- Kapatılan: `net_tcp` — `GetExtendedTcpTable` (iphlpapi, ham FFI)
  ile sahip-pid sayımı eklendi; `net_udp` 0 kalır (sahip-eşlemeli
  UDP tablosu yok). README/RFC satırları güncellendi.

## 2026-09-16 — hbmon events: push çekirdeğe girmedi (TASK-046)

- Kilit "pull-tabanlı" diyor; server-side `subscribe`/kalıcı bağlantı kilidi
  delerdi → push ihtiyacı çekirdeğe değil CLI'ya verildi: `hbmon events`
  yalnızca `log_tail` + `status --compact` pull'ları stdout akışına çevirir
  (`IPC_OPS` değişmedi, drift yeşil). Ajan komutu arka planda çalıştırınca
  olaylar ona push edilmiş gibi düşer — TASK-013 hükmü (pull + olay-bazı
  sidecar) yerleşik komuta dönüştü.
- Deney kuralı: kayıp > tekrar — kuyruk penceresi dışına düşen satırda
  akış baştan basar (çift satır tolere, kayıp yok).
- Server-side gerçek push (`subscribe` op) bilinçli olarak yapılmadı;
  ihtiyaç kanıtlanırsa ayrı karar + RFC değişikliği gerekir.

## 2026-09-16 — TASK-046 2./3. göz incelemesi

- 2. göz: `stream_start` "son satırı bul" mantığı, saniye-çözünürlüklü `ts`
  yüzünden bayt-aynı komşu satırda kayıp üretebilirdi → `StreamPos`
  (son satır + bitişik blok sayacı) ile kapatıldı; karar "kayıp yok"tan
  "tekrar > kayıp + bitişik tekrar korunur"a daraltıldı. Ölü soket
  (`shutdown` sonrası dahil) exit 3'e kilitlendi.
- 3. göz: RFC TR/EN komut enumerasyonları + USAGE-PATTERNS `events`'siz
  kalmıştı → güncellendi (EN ağaç satırı bayt-aynı). Yeni IPC op yok,
  `IPC_OPS`/drift yeşil, kilit ("pull-tabanlı") bozulmadı.
