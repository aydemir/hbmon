---
id: TASK-047
title: "2./3. göz inceleme kaydı + P1 düzeltmeleri (sock guard, wait terminal, exec timeout)"
status: done
priority: P1
created: 2026-09-18
updated: 2026-09-18
environment: both
labels: [review, safety, cli, correctness, daemon]
depends_on: []
---

# TASK-047 — 2./3. göz inceleme kaydı + P1 düzeltmeleri

## Amaç

Kod tabanının 2. göz (analiz) + 3. göz (doğrulama) incelemesi
sonucunda bulunan sözleşme/güvenlik hatalarının kaydı ve P1
sınıfının düzeltilmesi. Kullanıcı = LLM ajanı: hatalı exit kodu ve
sessiz veri kaybı ajan kararlarını doğrudan bozar.

## İnceleme Kaydı (2026-09-18)

Yöntem: 7.660 satır Rust + RFC/PROTOCOL/README/tasks okuması; 7 canlı
deney (A–G) ile iddiaların ampirik doğrulaması; `fmt`/`clippy`/`test`/
`build --release` kapıları. Kapılar taban durumda yeşil (82 unit + 3
drift + 18 integration + 1 smoke; release 2.44 MiB).

| # | Sınıf | Bulgu | Kanıt | Hüküm (3. göz) |
|---|---|---|---|---|
| S1 | veri kaybı | `--sock <var olan normal dosya>` dosyayı **siliyor** (stale-sock temizliği socket-olmayan yola da uygulanıyor) | canlı: probe dosyası socket'e dönüştü | DOĞRULANDI |
| S2 | liveness | `wait --until` yalnız terminal-olmayan sinyalle bitmiş build'de: timeout < linger → 124; timeout > linger (60 s) → `empty request` + **exit 3** | canlı ×2 (124@3.1s; exit 3@58s) | DOĞRULANDI (2. gözün "hep 124" genellemesi düzeltildi) |
| S3a | correctness | `exec --timeout-sec` kabul edilip **yok sayılıyor** | canlı: 2 s bayrağıyla 6.10 s sürdü, exit 0 | DOĞRULANDI |
| S3b | tutarlılık | dep taraması asimetrisi: `exec` yalnız stderr, `watch` stdout+stderr | canlı ×2 (exec stdout→0, stderr→2; watch stdout→dep_missing) | DOĞRULANDI |
| S3c | UX tuzağı | `watch` (`--detach`'sız): handshake yok, build çıktısı görünmez, +60 s linger | canlı: 1 s iş → 61 s, 0 bayt çıktı | DOĞRULANDI |
| S3d | perf | `log --tail` tüm dosyayı belleğe alıyor (100 MB cap'te daemon RSS sıçraması) | kod okuması (ölçülmedi) | KISMEN |
| S4a | tutarlılık | `kill`/`shutdown`/`list`/`cleanup` her durumda exit 0; ölü build'de `killed:true` | canlı: build bitmişken kill → killed:true | DOĞRULANDI (nit'e düşürüldü) |
| S4b | girdi | `wait --poll-ms 0` clamp'siz; `watch --pid`, `--label` no-op | kod | DOĞRULANDI |
| S4c | DX | daemon çekilince hata metni `empty request` (gerçek: EOF) | canlı | DOĞRULANDI |
| S4d | hijyen | ölü kod: `metrics/{mem,fds,io,net}`, `proc/metrics`, `platform/{linux,macos,windows}` shim'leri, `pidfile::read_pidfile` | grep: tanım dışı referans 0 | DOĞRULANDI |
| S5 | drift | `index.json` 0.1.0 ↔ Cargo 0.1.1; RFC test sayıları (70/14 → 82/18); `EventLogger::create` yorumu "create_new" der ama `create(true)` kullanır | karşılaştırma | DOĞRULANDI |
| — | bilgi | Kilit sırası inversiyonu şüphesi: guard'lar tek-ifade, döngü yok → **reddedildi**; yalnız mutex-poisoning notu kaldı | kod | REDDEDİLDİ |
| — | bilgi | Windows ACL/`GetTcpTable` ve macOS libproc davranışı bu ortamda **kanıtlanamaz** (CI yalnız derler) | — | kanıtsız iddia |

Kilit filtresi: tüm öneriler hedef kilidi içinde (bug fix + hijyen);
TUI/Web, plugin, Prometheus, ML-ETA kapsam dışı — değişmedi.

## Kapsam (bu görev)

- **S1**: stale-sock temizliği yalnız gerçek socket dosyası için
  (socket-olmayan/symlink yolda hata; veri kaybı yok) + unit/integration kilit
- **S2**: `wait` terminal state'te listede olmasa da döner (dürüst terminal
  sonuç) + EOF hata metni düzeltmesi (S4c) + unit/integration kilit
- **S3a**: `exec --timeout-sec` watchdog (TERM → 5 s grace → KILL, exit 124) +
  integration kilit
- Doküman: `decisions.md` (S2 davranış değişikliği), RFC TR/EN `wait`
  erken-dönüş paragrafı, README/PROTOCOL notları
- Yapılmayacaklar: S3b/S3c/S3d/S4a/S4b/S4d/S5 (aşağıda takipte),
  yeni IPC op, yeni bağımlılık, daemon mimarisi değişikliği

## Uygulama Planı

1. `paths::sock_remove_stale` (unix: `is_socket` değilse `Err`) +
   `spawn_watch` çağrısı; unit testler (`tempfile`)
2. `wait_match`: terminal state → kanonik sinyal adıyla dönüş
   (`done/failed/dep_missing/timeout/oom_suspect`); `codec` EOF ayrımı
3. `exec`: stderr tee thread + `recv_timeout` ile dep sonucu; watchdog
   (`timeout_sec > 0`), TERM→grace→KILL, exit 124
4. Testler: `wait_until_terminal_not_listed_returns_state` (integration),
   `exec_timeout_exits_124` (integration), codec EOF unit, paths unit,
   wait_match unit
5. `cargo fmt` + `clippy -- -D warnings` + `cargo test --locked -j2`
6. Canlı yeniden-doğrulama: A (exec timeout 124), B (wait terminal),
   F (dosya korunur)

## Etkilenen Dosyalar

- `src/platform/paths.rs`, `src/daemon/daemon.rs`
- `src/daemon/handler.rs`, `src/ipc/codec.rs`
- `src/cli/exec.rs`
- `tests/integration.rs`
- `tasks/decisions.md`, `HBMON-RFC.md`, `HBMON-RFC-EN.md`, `README.md`,
  `README.tr.md`, `PROTOCOL.md`, `AGENTS.md`, `skills/hbmon/SKILL.md`
- `tasks/KANBAN.md`, `tasks/index.json`

## Doğrulama

- Üç kapı yeşil (fmt/clippy/test) + yeni kilitli testler
- Canlı A/B/F deneyleri beklenen sonucu verir (aşağıda)

## Takipte (ayrı TASK adayı, bu görev dışı)

- **S3b** exec stdout dep taraması (veya PROTOCOL'da asimetri belgesi)
- **S3c** non-detach `watch` linger/çıktı politikası
- **S3d** `log_tail` sondan-okuma (RSS sıçraması)
- **S4a/S4b** exit-code tutarlılığı, clamp'ler, no-op bayraklar
- **S4d** ölü kod/duplikasyon temizliği
- **S5** `index.json` sürümü, RFC test sayıları, `create_new` yorumu

## Gerçekleşme Notu (2026-09-18)

- **S1:** `paths::sock_non_socket` (unix `symlink_metadata` +
  `FileTypeExt::is_socket`; symlink reddedilir, yok olan yol çakışma
  değil) + `sock_remove_stale` (yalnız gerçek socket; NotFound no-op).
  `spawn_watch` stale dalı ve `watch::run` handshake-öncesi ön-uçuş bunu
  kullanır; daemon çıkış temizliği `sock_remove` olarak kaldı.
- **S2:** `wait_match` → `terminal_signal` fallback'i (kanonik ad; OOM →
  `oom_suspect`); terminal dalı `code`/`exit_event` ile döndüğü için
  ajan 60 s linger'ı beklemez ve exit 3 sınıfı ortadan kalkar.
  `codec::read_message` EOF'u `connection closed` diye ayırır (S4c).
- **S3a:** `exec` stderr tee thread'e taşındı (dep sonucu
  `mpsc::recv_timeout(500 ms)` ile sınırlı toplanır); watchdog
  `timeout_sec > 0` iken TERM → 5 s grace → KILL → exit 124, kapalıyken
  doğrudan `wait()` (poll gecikmesi yok). Grup kill bilinçli yok:
  stdin inherit / Ctrl-C semantiği korunur (grup için `watch`).
- **S3b belgelendi:** `exec` dep taraması yalnız stderr — PROTOCOL,
  README (EN/TR), RFC TR/EN ve `exec.rs` başlığına yazıldı; stdout
  taraması hâlâ takipte.
- Doküman: `decisions.md` 2026-09-18 girdisi; RFC TR/EN `wait` erken
  dönüş + `exec` notu; RFC §14.1 test sayıları (88/21+1 smoke) senkron;
  README (EN+TR) Stability + ajan hızlı yolu; PROTOCOL §1/§3/§4/§5;
  AGENTS.md gotchas; `skills/hbmon/SKILL.md`.

### Doğrulama (kapılar + canlı)

- `cargo fmt --check` → 0; `clippy --locked --all-targets -- -D warnings`
  → 0; `cargo test --locked -j2` → **92 unit + 3 drift + 21 integration
  (+2 gerekçeli-ignore) + 1 smoke**, tamamı yeşil.
- Yeni kilitli testler: `paths::{non_socket_path_refused_and_kept,
  stale_socket_is_removed_and_missing_is_noop, symlink_path_is_refused}`,
  `codec::eof_is_connection_closed`, `handler::{terminal_state_wakes_
  even_when_not_listed, non_terminal_states_still_wait}`,
  `integration::{wait_until_terminal_not_listed_returns_state,
  exec_timeout_exits_124, watch_refuses_to_remove_non_socket_path}`.
- Canlı A: `exec --timeout-sec 2 -- sleep 6` → **exit 124 @2.25 s**
  (önceden 6.10 s / exit 0); `--timeout-sec 0` → exit 0 @2.11 s (kapalı).
- Canlı B: bitmiş build'de `wait --until stall_suspect` → **exit 0
  @0.10 s**, `state=done`, `woke_on=done` (önceden 124@3.1 s ya da
  linger sonrası exit 3).
- Canlı F: `--sock <normal dosya>` → **exit 3**, stdout 0 bayt,
  `refusing to use non-socket path: …`, dosya bozulmadan yerinde
  (önceden socket'e dönüşüyordu).
- Canlı C (değişmedi, artık belgeli): stdout dep satırı `exec`'te 0,
  stderr'de 2.
