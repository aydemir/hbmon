---
name: hbmon
description: Run long builds in the background and poll them cheaply (watch / status / wait / exec / log).
---

# hbmon — background build monitor

Single binary, zero setup. Full contract: `PROTOCOL.md`. Keep polling cheap.

## Start

`hbmon watch --detach -- <cmd>` → stdout line 1 is the handshake, parse it:

```json
{"v":1,"ev":"ready","uuid":"...","sock":"/tmp/hbmon-....sock","log":"..."}
```

Keep `sock`. Discovery if lost: `--sock` flag > `$HBMON_SOCK` > newest
`/tmp/hbmon-*.sock`. `hbmon list` shows all monitors (read-only).

## Poll or block

- Cheap poll: `hbmon status --sock $SOCK --compact`
- Block with early return:
  `hbmon wait --sock $SOCK --until done,failed,dep_missing,timeout,stall_suspect,oom_suspect`
  (aliases: `stalled`, `oom_killed`; unknown name → `INVALID_UNTIL`, exit 3)
- Events, never full logs: `hbmon log --sock $SOCK --tail N [--event metric]`

## Exit codes and actions

| Exit | State | Do this |
|---|---|---|
| 0 | done | continue |
| 1 | failed | read `log --tail`, fix, retry |
| 2 | dep_missing | install the package, retry |
| 124 | timeout | raise `--timeout` or split the build, retry |
| 137 | oom | reduce memory, retry (never as-is) |
| 3 | internal | report with `--compact` output |

Suspect signals are heuristics: `stall_suspect` → confirm with
`status --compact` + `log --event metric`, `kill` only if truly stuck.

## Never do this

- `status`/`wait` on an `exec` handshake (`ephemeral:true` — no daemon).
- Reuse a socket after a connect error (stale) — `watch` again.
- `cat` the whole `.jsonl` — always `--tail N`.
