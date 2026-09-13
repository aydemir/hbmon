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
