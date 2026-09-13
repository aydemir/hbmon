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
policy is outside HBMon's control (see `HBMON-RFC.md` §4.2.1 — in Turkish).

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

This section is for LLM agents; the full contract is in `HBMON-RFC.md`
(in Turkish) — don't skip it.

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

## Status

v1 MVP: daemon, UDS JSON-RPC (`status`/`metrics`/`wait`/`kill`/`log_tail`/`shutdown`),
JSONL event log, adaptive stall detection, OOM suspicion (dmesg), dependency-missing
pattern matching, timeout watchdog. Full design in `HBMON-RFC.md` (separate document,
in Turkish).
