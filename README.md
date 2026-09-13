**English** | [Türkçe](README.tr.md)

# hbmon — Harness-Independent Build Monitor

Single-binary, zero-runtime-dependency build watcher for LLM coding agents.
Works with any harness (OpenCode, Claude Code, Aider, …) on any OS — full
monitoring on Linux (CPU/RSS/IO/FD), process supervision + RSS/FD/path on
macOS (CPU best-effort), full monitoring on Windows (Toolhelp+RSS/IO/handles;
net best-effort, cmdline = exe path).

## How it works

Three-layer separation: **lifecycle** (setsid + double-fork → reparent to init),
**communication** (sidecar JSONL + pull-based queries over UDS, no log leaks into context),
**discovery** (env var + `/tmp/hbmon-*.sock` convention). Limitation: setsid
escapes the session/process-group but not the cgroup; host/harness cgroup
policy is outside HBMon's control (see [HBMON-RFC-EN.md](HBMON-RFC-EN.md) §4.2.1; Turkish original: [HBMON-RFC.md](HBMON-RFC.md)).

## Platform support

| Area | Linux | macOS | Windows |
|---|---|---|---|
| supervision | setsid + double-fork | setsid + double-fork | DETACHED_PROCESS + Job Object |
| CPU / RSS / IO / FD | full via `/proc` | CPU best-effort; RSS/FD/path via libproc | CPU/RSS/IO/handles via Toolhelp |
| network | TCP count via `/proc` | — | best-effort |
| cmdline | full | full | exe path only |
| OOM suspicion | dmesg | best-effort (dmesg format differs) | none (documented gap) |
| transport | UDS, files `0600` | UDS, files `0600` | named pipe (no ACL lockdown) |
| stall / dep-missing / timeout | yes (heuristic) | yes (heuristic) | yes (heuristic) |

## Install

```bash
cargo install hbmon   # crates.io, v0.1.0+
# or from source: cargo build --release  # stripped binary ~2.4MB (v0.1.0, <5MB goal; CI `size` job watches it)
# Windows: same → target\release\hbmon.exe (named-pipe transport)
```

## Usage

```bash
# Watch in background, return immediately
hbmon watch --detach -- make -j8
# {"v":1,"ev":"ready","uuid":"...","sock":"/tmp/hbmon-....sock","log":"..."}

# Query status
hbmon status --sock /tmp/hbmon-<uuid>.sock

# Block until done
hbmon wait --sock /tmp/hbmon-<uuid>.sock --timeout 600

# Run in foreground (first line is handshake JSON; ephemeral — no daemon,
# no status/wait, no sock/log files created on exit)
hbmon exec -- make -j8   # exit: 0 ok, 1 fail, 2 dep-missing
hbmon exec --format json -- make -j8  # last stderr line: JSON summary

# Kill / shut down
hbmon kill --sock ... --signal TERM
hbmon shutdown --sock ...
```

## For agents (automation fast path)

This section is for LLM agents; the full contract is in [HBMON-RFC-EN.md](HBMON-RFC-EN.md)
(Turkish original: [HBMON-RFC.md](HBMON-RFC.md)) — don't skip it. One-page English protocol summary:
[PROTOCOL.md](PROTOCOL.md).

1. `hbmon watch --detach -- <cmd>` → stdout line 1 = handshake JSON
   `{v,ev:"ready",uuid,sock,log}`. Parse line 1, keep `sock`.
2. Poll: `hbmon status --sock $SOCK` (cheap probe: `--compact`), or block:
   `hbmon wait --sock $SOCK --until done,failed,dep_missing,timeout,stall_suspect,oom_suspect`
   (aliases: `stalled`, `oom_killed`; unknown name → `INVALID_UNTIL`, exit 3).
3. Exit: `0 done / 1 failed / 2 dep-missing / 124 timeout / 137 oom / 3 internal error`.
   On `2`, install the missing package + retry.
4. `exec` is ephemeral: `sock`/`log` in the handshake are reserved names, no files
   are created — don't try `status`/`wait`.
5. Discovery order: `--sock > $HBMON_SOCK > /tmp/hbmon-*.sock` (newest);
   see all: `hbmon list` (read-only; `--state running` / `--live-only` filter).
   For events: `hbmon log --sock $SOCK --tail N` (don't cat the whole `.jsonl`;
   `--event metric` returns only matching events).
   Stale files: a crashed daemon leaves `.sock/.pid/.jsonl/.out` behind.
   `hbmon cleanup` removes files older than `--older-than` (default 86400s)
   but never touches a live daemon's siblings. If `status` fails with a
   connect error, the socket is stale — `watch` again, don't reuse it.
6. Suspect signals are heuristics, not proof — act, don't just wait:
   | signal | meaning | recommended action |
   |---|---|---|
   | `stall_suspect` / `stalled` | no IO/CPU/child-spawn past threshold | `status --compact` + `log --event metric` to confirm; `kill` if truly stuck, else keep waiting |
   | `oom_suspect` / `oom_killed` | OOM-killer trace matched | don't retry as-is — reduce memory usage, then retry |
   | `dep_missing` | missing-dependency pattern (exit 2) | install the package + retry |

## Stability & SemVer

`0.x`: no breaking change to the frozen surface without a minor bump and a
`decisions.md` entry. Frozen: handshake JSON (`v`, `ev:"ready"`, `uuid`,
`sock`, `log`; `exec` adds `ephemeral:true`), exit-code mapping (0 done /
1 failed / 2 dep-missing / 124 timeout / 137 oom / 3 internal error),
`wait --until` canonical names (`done failed dep_missing timeout
stall_suspect oom_suspect`, aliases `stalled oom_killed`),
`status --compact` field set, IPC ops (`status metrics log_tail wait kill
shutdown`). Experimental (may change): metric fields, stall
thresholds/scores, `metrics`/`log_tail` output details. Locked by
`tests/drift.rs`.

## Status

v1 MVP: daemon, UDS JSON-RPC (`status`/`metrics`/`wait`/`kill`/`log_tail`/`shutdown`),
JSONL event log, adaptive stall detection, OOM suspicion (dmesg), dependency-missing
pattern matching, timeout watchdog. Full design in [HBMON-RFC-EN.md](HBMON-RFC-EN.md) (separate document;
Turkish original: [HBMON-RFC.md](HBMON-RFC.md)).
