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
