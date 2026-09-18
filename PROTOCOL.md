# hbmon Protocol Reference (one page)

English summary of the agent-facing contract. Normative detail (in Turkish)
is in [HBMON-RFC.md](HBMON-RFC.md); frozen-vs-experimental promises are in
[README.md](README.md) ("Stability & SemVer").

## 1. Handshake

`hbmon watch --detach -- <cmd>` prints exactly one JSON line on stdout:

```json
{"v":1,"ev":"ready","uuid":"<hex>","sock":"/tmp/hbmon-<uuid>.sock","log":"/tmp/hbmon-<uuid>.jsonl"}
```

Parse line 1, keep `sock`. `hbmon exec -- <cmd>` prints the same shape plus
`"ephemeral":true` — `sock`/`log` are reserved names, no files are created,
`status`/`wait` do not exist for it. `exec` exits `0/1/2`, and
`--timeout-sec N` adds a watchdog (`124`; TERM → 5 s grace → KILL; `0` = off).
Both stdout and stderr are tee'd and scanned for dep-missing patterns
(`watch` scans both streams via `.out` — parity, TASK-048/S3b).

## 2. Discovery

`--sock` flag wins, then `$HBMON_SOCK`, then newest `/tmp/hbmon-*.sock`.
`hbmon list` shows all known monitors (read-only).

## 3. Operations (UDS JSON-RPC, `{"v":1,"op":...,"id":...}`)

| op | purpose |
|---|---|
| `status` | full snapshot (state, metrics, tree, health, log_tail) |
| `status` + `compact:true` | cheap poll: `state uuid elapsed_sec health last_event code?` |
| `metrics` | counters only |
| `wait` | block until a signal or `--timeout` (see §4; terminal states return even if unlisted) |
| `log_tail` | last N `.jsonl` lines, optional `event` filter |
| `kill` | TERM/KILL the build process group |
| `shutdown` | stop the daemon (kills the build) |

## 4. `wait --until` signals

Canonical: `done failed dep_missing timeout stall_suspect oom_suspect`
(aliases: `stalled` = `stall_suspect`, `oom_killed` = `oom_suspect`).
Empty `--until` = wait for terminal states only. Unknown name →
`INVALID_UNTIL`, exit 3. Early return adds `woke_on:<name>`.

A terminal state always returns, even when it is not in `--until`
(`woke_on` = canonical name; OOM → `oom_suspect`) — a finished build never
waits out the 60 s linger.

## 5. Exit codes

`0` done · `1` failed · `2` dep-missing (install + retry) · `124` timeout
(`wait --timeout`, `exec --timeout-sec`, `events --timeout`) · `137` oom ·
`3` internal error. `stall_suspect`/`oom_suspect` are heuristics:
confirm with `status --compact` + `log --event metric` before acting.

## 6. Files & permissions

Per monitor: `.sock` `.pid` `.jsonl` `.out` under `/tmp` (unix) —
created `0600`. `hbmon cleanup` removes stale files but never touches a
live daemon's siblings. Windows uses named pipes (current-user-only DACL).
