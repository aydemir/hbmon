---
id: TASK-048
title: "TASK-047 incelemesinin kalan bulguları (exec stdout taraması, watch linger, log_tail belleği, exit kodları, no-op bayraklar, ölü kod, drift)"
status: done
priority: P1
created: 2026-09-18
updated: 2026-09-18
environment: both
labels: [review, correctness, perf, cli, hygiene]
depends_on: [TASK-047]
---

# TASK-048 — TASK-047 incelemesinin kalan bulguları

## Amaç

TASK-047 kaydındaki "Takipte" listesini kapatmak: her bulgu ya düzeltilir
ya da gerekçeli won't-fix olarak `decisions.md`'ye yazılır. Yarım bırakılan
bir bulgu, ajanın exit kodu / context maliyeti hesabını bozar.

## Kapatılan Bulgular

| # | Bulgu | Çözüm |
|---|---|---|
| S3b | `exec` dep taraması yalnız stderr (watch ile eşitsizlik) | stdout tee → bayt-aynı stdout'a + dep taraması (parite) |
| S3c | non-detach `watch`: çıktı görünmez, +60 s linger | non-detach modda linger atlanır + `--help`/README notu |
| S3d | `log_tail` tüm dosyayı belleğe alır (100 MB cap) | sondan-artan pencere; seyrek filtre için O(n) akış |
| S4a | `kill`/`shutdown`/`cleanup` her durumda exit 0; ölü build'de `killed:true` | `ok`/`killed` denetimi → exit 1; cleanup `failed` sayacı |
| S4b | `wait --poll-ms` clamp'siz; `--pid`/`--label` no-op | sunucu tarafı clamp; `--pid`→`parent_pid`, `--label`→`label` (ready + status) |
| S4d | ölü kod: `metrics/{mem,fds,io,net}`, `proc/metrics`, `platform/{linux,macos,windows}` shim'leri, `pidfile::read_pidfile`, `EventLogger::open_append`, cgroup `io_*bytes` | silinir (RFC §9 ağacı ve README/RFC notları güncellenir) |
| S5 | `index.json` 0.1.0; log/pidfile symlink kontrolü "O_EXCL" iddiası (TOCTOU) | sürüm 0.1.1; `O_NOFOLLOW` gerçekten uygulanır (RFC iddiası koda uydurulur); RFC Windows test sayısı bayatlığı kaldırılır |

## Kapsam

- Yapılmayacaklar: yeni IPC op, yeni bağımlılık, TUI/Web, plugin, daemon
  mimarisi değişikliği (`linger` parametresi hariç).
- Dondurulmuş yüzey: handshake alanları, exit eşlemesi (build), `wait --until`
  adları, `status --compact` alan seti, IPC op'ları **değişmez**; eklemeler
  (full status `label`/`parent_pid`) geriye uyumlu.

## Uygulama Planı

1. `eventlog::tail_filter` → sondan pencere (64 KiB → 4 MiB) + O(n) akış
   fallback; birim testleri (büyük dosya, seyrek filtre, kısmi ilk satır)
2. `exec`: stdout tee thread (bayt-aynı yönlendirme) + dep taraması;
   integration testi (stdout dep satırı → exit 2)
3. `run_daemon(cfg, linger)`; non-detach `watch` linger'sız; integration
   testi (foreground watch hızlı döner)
4. `handler`: `poll_ms` clamp; `kill` → `killed:false`; `Shared.label`;
   `status_map` `label`/`parent_pid`; `cli`: kill/shutdown/cleanup exit 1
   denetimi; `watch --pid` → cfg
5. Ölü kod silme + `O_NOFOLLOW` (log/pidfile/out/truncate)
6. Doküman: PROTOCOL (§5 control exit), README (EN/TR), RFC §9 ağacı +
   symlink ifadesi + Windows test sayısı, AGENTS gotcha, decisions girdisi
7. Kapılar + canlı doğrulama + TASK-047 "Takipte" listesinin kapatılması

## Etkilenen Dosyalar

- `src/eventlog/mod.rs`, `src/cli/exec.rs`, `src/cli/watch.rs`,
  `src/cli/{kill,shutdown}.rs`, `src/cli/mod.rs`, `src/daemon/{daemon,handler}.rs`,
  `src/ipc/protocol.rs`, `src/platform/perm.rs`, `src/metrics/{mod,cgroup}.rs`,
  `src/proc/mod.rs`, `src/platform/mod.rs`
- silinen: `src/metrics/{mem,fds,io,net}.rs`, `src/proc/metrics.rs`,
  `src/platform/{linux,macos,windows}.rs`
- `tests/integration.rs`
- `README.md`, `README.tr.md`, `PROTOCOL.md`, `AGENTS.md`, `HBMON-RFC.md`,
  `HBMON-RFC-EN.md`, `index.json`, `tasks/decisions.md`
- `tasks/KANBAN.md`, `tasks/index.json`

## Doğrulama

- `cargo fmt --check` + clippy + `cargo test --locked -j2` yeşil
- Canlı: stdout dep satırı exec'te exit 2; foreground watch < 10 s;
  ölü build'de kill → `killed:false` + exit 1; `status` hızlı kalır
