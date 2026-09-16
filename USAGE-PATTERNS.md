# hbmon Consumer Patterns (one page)

`hbmon` core will never ship a plugin, MCP server, or skill (target lock:
single binary, zero runtime dependencies, harness-independent). But *you*,
the LLM consumer, can wrap it however your harness likes. Three patterns:

## 1. Bare CLI (default — zero setup)

If your harness has a shell, you need nothing else. Paste the contract
([PROTOCOL.md](PROTOCOL.md)) and run:

```bash
# 1. start, parse line 1 of stdout, keep sock
hbmon watch --detach -- make -j8
# {"v":1,"ev":"ready","uuid":"...","sock":"/tmp/hbmon-....sock","log":"..."}

# 2a. cheap poll while doing other work
hbmon status --sock $SOCK --compact

# 2b. or block with early return
hbmon wait --sock $SOCK --until done,failed,dep_missing,timeout,stall_suspect,oom_suspect

# 3. act on exit: 0 continue / 1 read logs / 2 install+retry /
#    124 timeout / 137 oom / 3 internal error
hbmon log --sock $SOCK --tail 30 --event metric
```

- Plus: no setup, works on every harness, you control context spend
  (when to `--compact`, how many `--tail` lines).
- Minus: you carry the contract in your prompt (`exec` has no
  status/wait; stale socket means re-`watch`, don't reuse).

## 2. As a skill

Same commands, but the instructions ship as a skill package instead of a
pasted prompt. A minimal `hbmon/SKILL.md` skeleton:

```markdown
---
name: hbmon
description: Monitor long builds in the background (watch/status/wait/exec).
---
# hbmon
1. `watch --detach -- <cmd>` → stdout line 1 = `{v,ev:"ready",uuid,sock,log}`.
2. Poll: `status --compact`. Block: `wait --until <signals>`. Stream: `events --event exit,dep_missing &` (push hissi; çekirdek hâlâ pull).
3. Exit: 0 done / 1 failed / 2 dep-missing (install+retry) / 124 / 137 / 3.
4. `exec` is ephemeral: NO status/wait. `stall_suspect` → confirm then kill;
   `oom_suspect` → don't retry, reduce memory; stale socket → re-watch.
```

- Plus: no prompt bloat, versioned instructions, reusable across projects.
- Minus: one-time install per harness. (Body is ~done: README agent
  section + [PROTOCOL.md](PROTOCOL.md) + suspect table.)

## 3. Via plugin + MCP

Someone *outside* core writes a thin layer mapping ops to tools:

```
hbmon_watch(cmd) → {uuid, sock}        hbmon_log(sock, tail, event?) → [...]
hbmon_status(sock, compact?) → {...}   hbmon_kill(sock) / hbmon_shutdown(sock)
hbmon_wait(sock, until[], timeout) → {state, code, woke_on}
```

Your side (no shell, JSON only):

```json
// tool call: hbmon_wait
{"sock": "/tmp/hbmon-abc.sock", "until": ["done","dep_missing"], "timeout": 600}
// result: {"state":"dep_missing","code":2,"woke_on":"dep_missing"}
```

- Plus: no shell parsing, typed args, the only road on shell-less harnesses.
- Minus (all real): setup + server lifecycle (who keeps it up?); tool
  results auto-inject into context — the `--compact`/`--tail` discipline
  slips and logs leak back in; someone must maintain the harness×version
  matrix. Prior art on the OpenCode side: `aydemir/opencode-plugins`
  (`build-mon.mjs` + `hbmon-build-mon.mjs` adapter + settle-noticer).

## Which one?

| Pattern | Setup | Context control | Pick when |
|---|---|---|---|
| Bare CLI | none | fully yours | shell exists — default |
| Skill | once | fully yours | same contract across many projects |
| Plugin+MCP | server+pkg | partly host's | no shell, or typed tools required |

Rule of thumb: shell → 1, laziness → 2, no shell → 3.
