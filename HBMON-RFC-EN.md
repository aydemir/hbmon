> English translation of [HBMON-RFC.md](HBMON-RFC.md). In case of divergence, the Turkish original is normative.

# HBMon — Harness-Independent, OS-Independent Build Monitor
## Design and Reference Implementation Plan (RFC v0.1)

| Field | Value |
|---|---|
| **Status** | In sync with v0.2.2 (draft period closed) |
| **Date** | 2026-09-09 (first draft) — last sync 2026-09-18 |
| **Author** | (user) — original architecture design; Rust implementation plan by Mavis |
| **Audience** | LLM harness developers, build-orchestration authors, agent users on projects with high build times |
| **Reference** | `aydemir/opencode-plugins` — DHS PTC + `build-mon.mjs` (+ `hbmon-build-mon.mjs` adapter) + `opencode-settle-noticer` (existence proof; `.sh` predecessors in `scripts/archive/`) |
| **Language** | Rust (stable, edition 2021) |
| **Target OS (v1)** | Linux + macOS — done; Windows pulled into v0.1.0 with TASK-006 |
| **Target OS (v2)** | (closed target — Windows arrived early) |
| **License** | MIT OR Apache-2.0 |

---

## Table of Contents

1. [Abstract](#1-abstract)
2. [Motivation and Problem](#2-motivation-and-problem)
3. [Design — Three-Layer Separation](#3-design--three-layer-separation)
4. [Process Lifecycle Separation (Layer 1)](#4-process-lifecycle-separation-layer-1)
5. [Context-Penetration-Free Communication (Layer 2)](#5-context-penetration-free-communication-layer-2)
6. [Discovery Mechanisms (Layer 3)](#6-discovery-mechanisms-layer-3)
7. [Health and Liveness Detection](#7-health-and-liveness-detection)
8. [LLM Conversation Protocol — Full Schema](#8-llm-conversation-protocol--full-schema)
9. [Rust Crate Structure and Architecture](#9-rust-crate-structure-and-architecture)
10. [OS Independence — `ProcessInspector` Trait Contract](#10-os-independence--processinspector-trait-contract)
11. [Usage Scenarios](#11-usage-scenarios)
12. [Harness-Independence Proof Matrix](#12-harness-independence-proof-matrix)
13. [Security, Limitations, and Race Conditions](#13-security-limitations-and-race-conditions)
14. [Future Work (v1.5 / v2 / v3)](#14-future-work-v15--v2--v3)
15. [Open Questions and Discussion](#15-open-questions-and-discussion)
16. [References](#16-references)

---

## 1. Abstract

The biggest operational bottleneck of LLM-based coding agents is **long build times**. Existing solutions are either tightly bound to the harness (plugin/hook system, MCP, output contract), pollute LLM context (continuous log stream), or depend on the OS (`/proc` assumption, Linux-only).

This RFC proposes a **harness- and OS-independent** build monitor with a **three-layer separation** (process lifecycle, communication, discovery). The target implementation is a single static binary in Rust; Linux and macOS are supported in v1.

**Main contribution:** a zero-harness-dependency contract so that the LLM can manage the build process via **passive monitoring + pull-based queries**. Existing harnesses (OpenCode, Claude Code, Aider, Cursor BG, Codex CLI, raw shell) can use HBMon with no changes whatsoever.

**Reference existence proof:** the `build-mon.mjs` (+ hbmon-powered `hbmon-build-mon.mjs` adapter) + `opencode-settle-noticer` combination in the `aydemir/opencode-plugins` repo is the proven, working form of this philosophy at plugin/shim level (`.sh` predecessors were ported to Node because of multios path problems — opencode-plugins TASK-127 — and moved to `scripts/archive/`). This RFC elevates it into an OS-independent, harness-independent, first-class Rust tool.

---

## 2. Motivation and Problem

### 2.1 Problem — Harness-Locked Build Orchestration

Current LLM harnesses manage build processes as follows:

```
┌──────────────────────────────────────┐
│  LLM Harness (OpenCode, Claude…)    │
│  ├─ session timeout (örn. 5dk)      │
│  ├─ context window (örn. 200K tok)  │
│  ├─ tool allowlist                   │
│  ├─ child reaper (SIGHUP on exit)   │
│  └─ output contract (JSON schema)    │
└──────────────────────────────────────┘
        │
        ▼ (kısıtlar)
┌──────────────────────────────────────┐
│  Build process (make, cargo, npm)   │
└──────────────────────────────────────┘
```

Typical outcomes:

- The LLM session closes in the middle of a 4-minute build
- Context fills up with a continuous log stream
- Child processes die when the session ends
- The user has to keep asking "is the build done?" over and over

### 2.2 Observations — `aydemir/opencode-plugins` Case Study

Findings drawn from real experience:

1. **The `build-mon.mjs` + `opencode-settle-noticer` combination provides real benefit.** While the build runs in the background, the LLM does not sit idle; it can stay busy with other work. (History: the predecessor was `build-mon.sh`; multios path problems forced the Node port — see `scripts/archive/`.)
2. **The DHS PTC (context-saver) pattern works.** Build output does not leak into context; a summary is pushed when the event completes.
3. **The plugin/shim level gives the maximum for build monitoring — but slightly above the architectural minimum.** The truly right place is a first-class subsystem in the harness core.
4. **Practical constraint:** no PR budget for the harness core → staying at plugin/shim level is a deliberate choice.

### 2.3 Goal

> **HBMon is a single-binary, zero-runtime-dependency tool that, no matter which harness is used and no matter which OS it runs on (Linux/macOS), lets the LLM monitor the build process without spending tokens, be notified on completion, and query its instantaneous state when needed.**

### 2.4 Non-Goals (Out of Scope, v1)

- ~~No Windows support in v1 (pushed to v2)~~ → done: Windows landed in v0.1.0 with TASK-006 (named pipe transport + Win32 inspector + Job-Object detach/kill)
- No container-aware cgroup v2 separation in v1 (v1.5)
- No multi-build parallel monitoring in v1 (v2)
- No TUI/Web UI in v1 (v2)
- No plugin system (Lua/WASM) in v1 (v3 — over-engineering risk)
---

## 3. Design — Three-Layer Separation

```
┌─────────────────────────────────────────────────┐
│                  LLM Harness                    │
│         (shell tool + file read + env)          │
└─────────────────────────────────────────────────┘
        │                              ▲
        │ spawn                        │ pull (status)
        │ env: HBMON_SOCK, HBMON_LOG   │
        ▼                              │
┌─────────────────────────────────────────────────┐
│              HBMon (daemon)                     │
│  ┌───────────────┐  ┌────────────────────────┐  │
│  │ Katman 1:     │  │ Katman 2:              │  │
│  │ Yaşam Döngüsü │  │ İletişim               │  │
│  │ (daemonize,   │  │ (UDS + JSONL + exit)   │  │
│  │  setsid)      │  │                        │  │
│  └───────────────┘  └────────────────────────┘  │
│  ┌─────────────────────────────────────────────┐ │
│  │ Katman 3: Keşif (env, ps scan, handshake)  │ │
│  └─────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────┘
        │
        ▼ (süreç ağacı izleme, OS API)
┌─────────────────────────────────────────────────┐
│  Build Process Tree                             │
│  (make → cc, cc, cc → ld)                      │
└─────────────────────────────────────────────────┘
```

| Layer | Problem | Solution |
|---|---|---|
| **1. Lifecycle** | Child dies when the harness closes | Double-fork + setsid → reparent to init |
| **2. Communication** | Log fills LLM context | Sidecar JSONL + UDS (pull) + exit code |
| **3. Discovery** | How does the LLM find the monitor? | Env var + `/tmp/hbmon-*.sock` convention |

**Result:** The monitor lives outside the harness's field of view; the LLM reaches it via standard means (env, file, socket).

---

## 4. Process Lifecycle Separation (Layer 1)

### 4.1 Problem

When the LLM harness session closes (user cancel, network drop, session timeout), **SIGHUP is sent to the child processes.** This cannot be fully prevented even with `nohup` or `&` because:

- Some harnesses kill the process group
- Some do cgroup-based reaping
- When the terminal closes, processes attached to the controlling terminal receive SIGHUP

### 4.2 Solution — Daemonization Pattern

The LLM's shell tool spawns HBMon as follows:

```bash
hbmon watch --pid $$ --detach -- make -j8
```

Inside `hbmon` (`src/daemon/mod.rs`):

```
1. fork()
   ↓
2. setsid()                          ← yeni session
   ↓
3. fork()                            ← bir kez daha
   ↓
4. close(STDIN), close(STDOUT), close(STDERR)
   ↓
5. /tmp/hbmon-<uuid>.pid yaz
   ↓
6. /tmp/hbmon-<uuid>.sock aç (UDS)
   ↓
7. /tmp/hbmon-<uuid>.jsonl oluştur
   ↓
8. exit(0) (parent LLM'e döner)
```

**Daemon HBMon:**

- Now reparented to PID 1 (init)
- New session, no controlling terminal
- STDIN/STDOUT/STDERR closed
- The LLM gets exit code 0 and can continue its own work
- When the build finishes, the daemon cleans up after itself (pid/sock/log unlink + exit)

### 4.2.1 Guarantee Boundary (cgroup)

The above pattern does not conflate two separate dependencies:

- session / process-group dependency → solved with double-fork + setsid.
- cgroup membership is a separate mechanism; setsid does not remove processes from the cgroup.

Therefore the host/harness cgroup policy (reaping, limit, kill) is outside HBMon's control area. The guarantee is limited to this sentence: HBMon strives to be a supervisor independent of the terminal/session/process-group lifecycle; cgroup policies are a separate boundary. This does not mean the cgroup problem is solved — it only describes the boundary of the control area.

### 4.3 OS Behavior Matrix

| OS | setsid | Double-fork | Notes |
|---|---|---|---|
| Linux | `setsid(2)` | ✅ | Standard POSIX pattern |
| macOS | `setsid(2)` | ✅ | Same, available in `libc` |
| Windows | none (Job Object + self re-spawn) | ✅ (raw `CreateProcessW`) | `DETACHED_PROCESS` + `CREATE_NEW_PROCESS_GROUP` + `BREAKAWAY`; stdio NUL; no handle inheritance (`bInheritHandles=FALSE`, TASK-006) |

### 4.4 Rust Implementation Skeleton

```rust
// src/daemon/mod.rs
pub fn daemonize() -> Result<()> {
    // İlk fork
    match unsafe { fork() } {
        -1 => return Err(Error::Fork),
        0 => {}, // child devam
        _ => std::process::exit(0), // parent çık
    }

    // setsid
    if unsafe { libc::setsid() } == -1 {
        return Err(Error::Setsid);
    }

    // İkinci fork
    match unsafe { fork() } {
        -1 => return Err(Error::Fork),
        0 => {}, // grandchild devam
        _ => std::process::exit(0), // first child çık
    }

    // stdio kapat
    unsafe {
        libc::close(0);
        libc::close(1);
        libc::close(2);
    }

    // Working directory güvenli yere
    std::env::set_current_dir("/").ok();

    // umask temizle
    unsafe { libc::umask(0o022); }

    Ok(())
}
```

### 4.5 Killing Processes — `kill` Subcommand

If the LLM or the user wants:

```bash
hbmon kill --sock /tmp/hbmon-abc.sock --signal SIGTERM
# veya
hbmon kill --pid 12345 --signal SIGTERM
```

This sends `{"op":"kill","signal":15}` over UDS; the daemon terminates **the whole process group** with `kill(-pgid, sig)`.

**Windows note (TASK-006):** no graceful group signal — both `Term` and `Kill` terminate with `TerminateJobObject` (build `exit 1` → mapped to `failed`). If the Job cannot be assigned, fallback: Toolhelp tree walk + leaves-first `TerminateProcess` (`kill_tree`).
---

## 5. Context-Penetration-Free Communication (Layer 2)

### 5.1 Design Principle

> **HBMon never sends data directly to the LLM (push). The LLM pulls whenever it needs to (pull).**

For two reasons:

1. LLM context is precious — every log line means tokens
2. Most harnesses do not accept push events (subprocess model)

### 5.2 Three Channels

#### 5.2.1 Sidecar JSONL Event Log

```
/tmp/hbmon-<uuid>.jsonl
```

**Structure:** Each line is a single event, newline-delimited JSON. Append-only.

**Example:**

```jsonl
{"ts":"2026-09-09T00:00:00Z","v":1,"ev":"ready","uuid":"abc","sock":"/tmp/hbmon-abc.sock","log":"/tmp/hbmon-abc.jsonl","root_pid":1234,"cmd":"make -j8"}
{"ts":"2026-09-09T00:00:01Z","v":1,"ev":"metric","uuid":"abc","cpu":1.2,"rss_mb":8,"io_r":0,"io_w":0,"fds":5}
{"ts":"2026-09-09T00:00:05Z","v":1,"ev":"child_spawn","uuid":"abc","pid":1235,"ppid":1234,"cmd":"cc -c foo.c"}
{"ts":"2026-09-09T00:00:30Z","v":1,"ev":"stall_suspect","uuid":"abc","reason":"no_io_no_cpu","idle_sec":28.0,"threshold_sec":30.0}
{"ts":"2026-09-09T00:01:42Z","v":1,"ev":"exit","uuid":"abc","pid":1234,"code":0,"duration_sec":102.3,"state":"done"}
```

**LLM consumption:**

```bash
tail -n 20 /tmp/hbmon-abc.jsonl
```

**Advantages:**

- Persistent (can be inspected after the build)
- Append-only, no race condition
- The LLM reads it with any file read tool
- Standard format (JSONL, parse with jq)

**Rotation:** If it exceeds 100MB, old lines are deleted FIFO (max 100MB cap).

#### 5.2.2 Unix Domain Socket — JSON-RPC Over UDS

```
/tmp/hbmon-<uuid>.sock
```

**Connection model:** The LLM connects on each query, gets the answer, closes. No persistent connection (keep the state machine simple).

**Protocol:** UTF-8 newline-delimited JSON request, newline-delimited JSON response.

##### Operation: `status`

**Request:**
```json
{"v":1,"op":"status","id":"req-1"}
```

**Response:** (full schema in Section 8.)

Compact (poll-friendly, opt-in): `{"v":1,"op":"status","id":"req-1","compact":true}`
→ `state,uuid,elapsed_sec,health{stall_score,threshold_sec},last_event`
(plus `code` if finished); no `tree/metrics-detail/log_tail/root_cmd`.

##### Operation: `metrics` (Lightweight)

Returns only instantaneous metrics (no tree, fast):

```json
{"v":1,"op":"metrics","id":"req-2"}
→ {"v":1,"id":"req-2","ok":true,"cpu":12.3,"rss_mb":412,"io_r":88,"io_w":21,"fds":14,"net_tcp":3}
```

##### Operation: `wait` (Blocking)

Blocks until the build finishes. The LLM's way of saying "don't ask now, tell me when ready".

**Request:**
```json
{"v":1,"op":"wait","id":"req-3","timeout_sec":300,"poll_ms":500}
```

**Response (blocks until timeout, then):**
```json
{
  "v": 1,
  "id": "req-3",
  "ok": true,
  "state": "done",
  "code": 0,
  "duration_sec": 47.3,
  "exit_event": { "...": "..." }
}
```

**Behavior:**

- When `timeout_sec` expires, returns the last snapshot (state = "running" or "timeout")
- If the build finishes early, returns immediately
- The process tree is checked at a 500ms poll interval

**Early return (`until`):** if an `until` list is given (canonical: `done`,
`failed`, `dep_missing`, `timeout`, `stall_suspect`, `oom_suspect`; aliases:
`stalled`=stall_suspect, `oom_killed`=oom_suspect), the call returns at the
first matching signal with a `woke_on`
field; no polling or harness plugin is needed. An unknown signal name
yields an `INVALID_UNTIL` error (no silent ignoring). This is synchronous
multiplexed waiting, not async push (without `until`, it returns only on terminal
states).

> Note (v1 outcome, TASK-047): a terminal state (`done`/`failed`/
> `dep_missing`/`timeout`/`oom_killed`) returns even when not listed;
> `woke_on` carries the canonical signal name (`oom_suspect` for OOM).
> Waiting for an "early signal" on a finished build was a deadline/linger
> trap: 124 before the timeout, and exit 3 once the linger (60 s) ended
> and the daemon withdrew.
##### Operation: `kill`

```json
{"v":1,"op":"kill","id":"req-4","signal":15}
→ {"v":1,"id":"req-4","ok":true,"killed":true}
```

##### Operation: `log_tail` (Last N Lines)

```json
{"v":1,"op":"log_tail","id":"req-5","n":20}
→ {"v":1,"id":"req-5","ok":true,"lines":["...","..."]}
```

Optional server-side filter (TASK-029, backward compatible): if `"event":"metric"`
is given, only the last N lines whose `ev` exactly matches are returned
(`hbmon log --sock … --tail 20 --event metric`).

##### Operation: `shutdown`

Shuts down the daemon (the build does not continue):

```json
{"v":1,"op":"shutdown","id":"req-6","force":false}
→ {"v":1,"id":"req-6","ok":true}
```

With `force=false`, it first sends `SIGTERM` to the build, waits 5s, and sends `SIGKILL` if it is still alive.

#### 5.2.3 Exit Code (Final Summary)

If the LLM uses `hbmon wait` or `hbmon exec`, the daemon runs as the LLM's parent and gives a summary via exit code when the build finishes:

| Exit | Meaning | Trigger |
|---|---|---|
| `0` | Success | Build process `exit(0)` |
| `1` | Build error | `exit(≠0)` or no pattern matched |
| `2` | Dependency missing | Pattern match: "command not found", "ModuleNotFoundError", etc. |
| `3` | Generic process error | Unexpected (crash, signal) |
| `124` | Timeout | `--timeout-sec` exceeded |
| `130` | SIGINT (Ctrl+C) | User cancel — not implemented |
| `137` | OOM killer | dmesg/cgroup detection |
| `143` | SIGTERM | Killed by the daemon — not implemented |

> Note (v1 outcome): stall is not reflected in the exit code — `Stalled` is only a
> `state` + `stall_suspect` event; the build exit code is passed through as-is.
> Code `147` was from an older draft of this RFC and was not implemented.

**Advantage:** All harnesses understand exit codes. No extra contract needed.

### 5.3 Channel Selection Guide (For LLMs)

```
LLM ne yapmak istiyor?                    Kanal
─────────────────────────────────────────────────────────
Derleme bitene kadar bekle, sonra devam et → exit code
Bitti mi diye uzun poll                   → UDS wait
Şu an ne aşamada, hızlıca bak            → UDS status
Son olayları log olarak oku              → JSONL tail
Build'i öldür                             → UDS kill
```

### 5.4 Wire Format — Common Rules

- **Version:** Every message starts with `"v":1` (for schema evolution)
- **Newline-delimited:** Every message is a single line (ends with `\n`)
- **UTF-8:** All string fields are UTF-8
- **Timestamp:** ISO 8601 UTC (`"2026-09-09T00:00:00Z"`)
- **Large numbers:** In MB (integer)
- **PID:** Unsigned 32-bit integer
- **ID:** Client-side UUID/string for request/response matching

---

## 6. Discovery Mechanisms (Layer 3)

### 6.1 Three-Layer Discovery

#### Layer A — Explicit Env Variable (Most Robust)

```bash
# LLM shell tool:
HBMON_SOCK=/tmp/hbmon-abc.sock \
HBMON_LOG=/tmp/hbmon-abc.jsonl \
HBMON_UUID=abc \
  hbmon watch --pid $$ --detach -- make -j8
```

The LLM uses `HBMON_SOCK` in subsequent queries.

**Advantage:** Deterministic, no race condition, harness-independent.

**Disadvantage:** The LLM must retain the environment (most harnesses do).

#### Layer B — Convention-Based Scan (Fallback)

If the LLM has lost the env:

```bash
ls -t /tmp/hbmon-*.sock 2>/dev/null | head -1
```

Newest socket = most recently spawned monitor.

**Caution:** With multiple active builds, the wrong monitor may be selected. Hence **UUID-mandatory** usage is recommended.

#### Layer C — Handshake (Wrapper Mode)

```bash
hbmon exec -- make -j8
```

On the first line the daemon handshakes (to stdout):

```json
{"v":1,"ev":"ready","uuid":"abc","sock":"/tmp/hbmon-abc.sock","log":"/tmp/hbmon-abc.jsonl"}
```

Then the build output follows.

**The LLM parses:** first line is the handshake, the rest is build output. (The LLM can already split it as `head -n 1` + remainder.)

> Note (v1 outcome, TASK-007): `exec` is ephemeral — `sock`/`log` in the handshake
> are reserved names, no files are created; `status`/`wait` are not attempted.
> The handshake carries `"ephemeral":true` + `"note"`.

> Note (v1 outcome, TASK-047): `exec --timeout-sec N` has a watchdog
> (TERM → 5 s grace → KILL, exit 124; `0`/absent = off, the direct child
> is killed — use `watch` for the group). Dependency scanning in `exec`
> covers both stdout and stderr (TASK-048/S3b, parity with `watch`);
> `watch` scans both streams via `.out` — this difference is
> documented in `PROTOCOL.md`.

### 6.2 Which Should Be Preferred?

| Usage | Recommended |
|---|---|
| If the LLM shell tool will run the command **once** and leave | Layer C (handshake) |
| If the LLM will do other work for a **long time** and query later | Layer A (env) |
| If the LLM has lost the env / multi-build | Layer B (scan) — but carefully |

---

## 7. Health and Liveness Detection

### 7.1 State Machine

```
        spawn
          │
          ▼
      ┌───────┐
      │running│◀─────────────┐
      └───┬───┘              │
          │                  │
          ├─ child exit OK ───┤
          │                  │
          ├─ child exit fail │
          │       │          │
          │       ▼          │
          │  ┌──────┐        │
          │  │failed│        │
          │  └──────┘        │
          │                  │
          ├─ idle > 3×p95 ───┤
          │       │          │
          │       ▼          │
          │  ┌───────┐       │
          │  │stalled│       │
          │  └───┬───┘       │
          │      │ resume    │
          │      └───────────┘
          │
          ├─ OOM killed
          │       │
          │       ▼
          │  ┌──────────┐
          │  │oom_killed│
          │  └──────────┘
          │
          ├─ dep pattern match
          │       │
          │       ▼
          │  ┌────────────┐
          │  │dep_missing │
          │  └────────────┘
          │
          └─ timeout
                  │
                  ▼
            ┌──────────┐
            │ timeout  │
            └──────────┘
```

### 7.2 Stall / Hang Detection — Adaptive Threshold

**Problem:** A fixed threshold is wrong. A 5-second `cc -E` and a 600-second `lld -flto` cannot fall under the same rule.

**Solution:**

1. The first **60 seconds** are observed (warmup)
2. Every "quiet moment" during this period is recorded (moments when CPU=0 + I/O=0 + new child=0)
3. The **rolling p95** idle time is computed
4. Stall rule: `current_idle > 3 × p95_idle` AND `idle > 30s` (absolute lower bound)

**Edge cases:**

- **Network-bound build** (`go mod download`, `cargo fetch`): I/O active but CPU low. Rule: network I/O = "alive" signal
- **Link phase** (lld, ld): CPU high but no file writes. Rule: CPU > 0 = "alive" signal
- **Very short build** (< 10s): stall detection disabled (false-positive prevention)

**Output:**

```json
{"ts":"...","v":1,"ev":"stall_suspect","uuid":"abc","reason":"no_io_no_cpu","idle_sec":47.2,"threshold_sec":42.0,"p95_idle":14.0}
```

**Resume:**

```json
{"ts":"...","v":1,"ev":"stall_resolved","uuid":"abc","lasted_sec":47.2}
```
### 7.3 OOM Killer Detection

**Linux:** pattern over `/var/log/kern.log` or `journalctl -k -n 100`:

```
Out of memory: Killed process 1234 (cc) total-vm:...
```

or

```
oom-kill:constraint=CONSTRAINT_MEMCG,...
```

**macOS:** `log show --predicate 'eventMessage contains "jetsam"' --last 5m` (memory-pressure killer).

**Trigger:** if the OOM-killed PID is in our process tree → `state = "oom_killed"`, exit 137.

### 7.4 Dependency-Missing Detection

**Method:** stderr pattern matching + regex set.

**Default pattern set (v1):**

| Pattern | Category | Language/System |
|---|---|---|
| `command not found` | `dep_missing:command` | POSIX shell |
| `No such file or directory` | `dep_missing:path` | POSIX |
| `Cannot find module` | `dep_missing:npm` | Node.js |
| `ModuleNotFoundError` | `dep_missing:pymod` | Python |
| `error: failed to parse manifest` | `dep_missing:crates` | Cargo |
| `error: package ID specification` | `dep_missing:cabal` | Haskell |
| `fatal error: .*: No such file` | `dep_missing:header` | C/C++ |
| `Could not find .* in` | `dep_missing:lib` | ld linker |
| `Package .* was not found` | `dep_missing:pkg` | Debian/RHEL pkg-config |
| `error: linking with \S+ failed` | `dep_missing:link` | Rust |

**Trigger:** on pattern match → `state = "dep_missing"`, `ev = "dep_missing"`, exit 2.

**User extension:** additional patterns via `~/.config/hbmon/patterns.json` (or `$XDG_CONFIG_HOME/hbmon/patterns.json` if absent) — in v1 (JSON instead of the draft's `patterns.toml`: zero-dep principle, TASK-001).

### 7.5 Timeout

**CLI flag:** `--timeout-sec 600`

**Behavior:** on expiry, `SIGTERM` → 5s → `SIGKILL`. State = "timeout", exit 124.

---

## 8. LLM Conversation Protocol — Full Schema

### 8.1 `status` Response (Full)

The `status` response contains these fields: `v`, `id`, `ok`, `state` (`running` | `stalled` | `oom_killed` | `done` | `failed` | `dep_missing` | `timeout`), `uuid`, `root_pid`, `root_cmd`, `started_at`, `elapsed_sec`, `metrics` (`cpu_pct`, `rss_mb`, `io_read_mb`, `io_write_mb`, `fds_open`, `net_tcp`, `net_udp`), `tree` (pid/cmd/cpu/rss_mb/state/children), `health` (`stall_score`, `threshold_sec`, `last_io_at`, `last_cpu_nonzero_at`, `last_child_spawn_at`), `log_tail`, `last_event`.

> Note (v1 outcome): `eta_sec` remains in the schema as optional, but the daemon
> does not currently return it — future work (v1.5+).

### 8.2 Field Descriptions

| Field | Type | Description |
|---|---|---|
| `v` | u8 | Protocol version (currently 1) |
| `id` | string | Request/response matching |
| `ok` | bool | Error status (`err` field if false) |
| `state` | enum | `running` \| `stalled` \| `oom_killed` \| `done` \| `failed` \| `dep_missing` \| `timeout` |
| `uuid` | string | Monitor unique ID |
| `root_pid` | u32 | Monitored root process |
| `root_cmd` | string | Root process command line |
| `started_at` | timestamp | Daemon start time |
| `elapsed_sec` | float | `now - started_at` |
| `eta_sec` | float? | Estimated remaining time — not returned in v1 (future) |
| `metrics.cpu_pct` | float | Total process-tree CPU % |
| `metrics.rss_mb` | u32 | Total RSS (MB) |
| `metrics.io_read_mb` | u32 | Total read I/O (since build start) |
| `metrics.io_write_mb` | u32 | Total write I/O |
| `metrics.fds_open` | u32 | Open file-descriptor count |
| `metrics.net_tcp` | u32 | Active TCP connection count |
| `metrics.net_udp` | u32 | Active UDP "connection" count |
| `tree` | array | Process tree (root + children) |
| `health.stall_score` | float | 0.0 (active) — 1.0 (fully stalled) |
| `health.last_io_at` | timestamp | Last I/O activity |
| `health.last_cpu_nonzero_at` | timestamp | Last CPU>0 moment |
| `log_tail` | array<json string> | Last N lines of the JSONL log (unparsed) |
| `last_event` | object | Most recent event (summary) |

### 8.3 Event Types (Full List)

| Event | Trigger | Extra Fields |
|---|---|---|
| `ready` | Daemon handshake (stdout line 1; not written to `.jsonl`) | `sock`, `log`, `root_pid`, `cmd` |
| `spawn` | (in schema; not emitted in v0.2.0) | `pid`, `cmd`, `ppid` |
| `child_spawn` | (in schema; not emitted in v0.2.0 — its projection is `health.last_child_spawn_at`) | `pid`, `ppid`, `cmd` |
| `child_exit` | (in schema; not emitted in v0.2.0) | `pid`, `code`, `duration_sec` |
| `exit` | Root finished | `pid`, `code`, `duration_sec`, `state`, `summary?` |
| `metric` | Periodic measurement | `cpu`, `rss_mb`, `io_r`, `io_w`, `fds` |
| `stall_suspect` | Stall heuristic triggered | `reason`, `idle_sec`, `threshold_sec` |
| `stall_resolved` | Stall ended | `lasted_sec` |
| `oom_suspect` | OOM killer | `pid`, `killed_by` |
| `dep_missing` | Pattern match | `pattern_id`, `category`, `match_text` |
| `signal` | (in schema; not emitted in v0.1.1) | `pid`, `sig` |
| `health_change` | (in schema; not emitted in v0.1.1 — its projection is `status.state`) | `from`, `to` |
| `timeout` | Timeout exceeded | `elapsed_sec`, `limit_sec` |
| `shutdown` | Daemon shutting down | `reason` |
| `serve_error` | IPC serve/bind failed (TASK-053; daemon never runs blind, diagnosis stays in the log) | `error` |

> `exit.summary` (TASK-050, optional): preview cut at a line boundary from
> the last ~2KB of `.out` (leading `…` when trimmed). Consumers read the
> summary first and open the full log only when needed (context economy).
> Raw truncation — no LLM/AI interpretation. Old logs without summary
> remain valid.

### 8.4 Error Response

```json
{
  "v": 1,
  "id": "req-1",
  "ok": false,
  "err": {
    "code": "BUILD_NOT_FOUND",
    "message": "root process 1234 has already exited"
  }
}
```

**Error codes:**

| Code | Meaning |
|---|---|
| `INVALID_REQUEST` | JSON parse / schema error |
| `UNKNOWN_OP` | Unknown operation |
| `BUILD_NOT_FOUND` | Monitored process no longer exists |
| `ALREADY_SHUTDOWN` | Daemon shut down — not implemented (absent from dispatch) |
| `TIMEOUT` | `wait` timeout exceeded — not implemented (`wait` returns `ok:true` + `timeout:true` instead) |
| `INTERNAL` | Unexpected internal error — not implemented |

---

## 9. Rust Crate Structure and Architecture

### 9.1 Crate Structure (Single Crate, Modular)

```
hbmon/
├── Cargo.toml
├── README.md
├── src/
│   ├── main.rs              ← CLI entry (clap derive)
│   ├── lib.rs               ← kütüphane re-exports
│   ├── cli/                 ← watch, status, wait, events, exec, kill, shutdown
│   ├── daemon/              ← daemonize, lifecycle, pidfile, signals
│   ├── ipc/                 ← UDS server, JSON-RPC dispatcher, codec
│   ├── proc/                ← ProcessInspector trait + Linux/macOS impl
│   ├── metrics/             ← cpu, mem, io, fds, net
│   ├── health/              ← state, stall, oom, dep_missing, timeout
│   ├── eventlog/            ← JSONL appender (rotation ile)
│   ├── platform/            ← cfg(target_os) dispatch
│   └── util/                ← uuid, time
├── tests/
│   ├── integration.rs       ← uçtan uca: watch/status/wait/kill
└── docs/
    └── examples/
```

### 9.2 Dependency Principles

- **Why no `tokio`:** The single binary must stay small (goal < 5MB). Thread-per-task suffices; an async runtime adds ~2MB.
- **Why no `sysinfo`:** It gives no child tree, OOM detection, or stall heuristic; our own trait layer is more testable.
- **Why no `nix`:** Direct `libc` suffices for a few syscalls.

### 9.3 Module Dependency Graph

```
cli ──→ daemon ──→ proc ──→ platform
            │         │
            ├──→ ipc ─┤
            ├──→ eventlog
            ├──→ metrics
            ├──→ health ──→ proc
            └──→ util
```

**Rule:** Upper layers depend on lower layers; never the reverse. No circular dependencies.
---

## 10. OS Independence — `ProcessInspector` Trait Contract

```rust
pub trait ProcessInspector: Send + Sync {
    fn list_children(&self, pid: u32) -> Result<Vec<u32>, String>;
    fn metrics(&self, pid: u32) -> Result<Metrics, String>;
    fn cmdline(&self, pid: u32) -> Result<String, String>;
    fn tree(&self, pid: u32) -> Result<TreeNode, String>;
    fn is_alive(&self, pid: u32) -> bool;
    fn bulk_metrics(&self, pids: &[u32]) -> Result<HashMap<u32, Metrics>, String>;
}
```

- **Linux:** direct `/proc` parsing (dependency-free, best-effort).
- **macOS:** `libproc` FFI (`proc_listpids`, `proc_pidinfo`); metrics best-effort in v1.
- **Windows (realized with TASK-006):** raw Win32 FFI, no new crates —
  Toolhelp snapshot for the tree, `GetProcessMemoryInfo` for RSS,
  `GetProcessTimes` for raw CPU time (centisecond → `CpuTracker` unchanged),
  `GetProcessIoCounters` for IO counters, `GetProcessHandleCount` for fds;
  named-pipe transport (`\\.\pipe\hbmon-<uuid>`), wire format v1 unchanged.
  Documented gaps: `net_udp` = 0 (no owner-mapped UDP table),
  `cmdline` = exe path (not argv).

CPU percentage always requires a delta → `CpuTracker` (jiffies difference / elapsed time, 100Hz assumption).

---

## 11. Usage Scenarios

### 11.1 Scenario 1 — Simple Spawn + Wait

```bash
hbmon exec -- make -j8
# → exit 0 (başarı) veya 1 (fail) veya 2 (dep missing)
```

### 11.2 Scenario 2 — Active Monitoring (LLM Busy)

```bash
hbmon watch --detach --uuid $U -- make -j8   # hemen döner
# ... LLM başka iş yapar ...
hbmon status --sock /tmp/hbmon-$U.sock
hbmon wait --sock /tmp/hbmon-$U.sock --timeout 600
```

### 11.3 Scenario 3 — Early Error Detection

If the LLM sees the `dep_missing` event via periodic `log_tail`, it intervenes before the build finishes.

### 11.4 Scenario 4 — Multiple Builds (first-class in v2, scan in v1)

```bash
ls -t /tmp/hbmon-*.sock | head -2
```

### 11.5 Scenario 5 — `build-mon` Migration (history: `.sh` → `.mjs`)

| `build-mon.sh` (archived) / `build-mon.mjs` (current) | `hbmon` |
|---|---|
| `pgrep -P $PID` polling | `hbmon wait` (blocking) |
| Custom event parsing | `hbmon status --format json` |
| OS-specific (pgrep, ps) / Node multi-OS | Cross-OS trait |
| No stall/OOM/dep-missing | All included |

Note: the opencode-plugins side was ported to Node due to multios-path problems (TASK-127); the `.sh` predecessors live in `scripts/archive/`. The `hbmon-build-mon.mjs` adapter realizes the build-mon contract with the hbmon engine (hbmon side: TASK-004).

---

## 12. Harness-Independence Proof Matrix

| Harness | Shell tool | Env var | File read | HBMon compatible? |
|---|---|---|---|---|
| OpenCode / Claude Code / Aider / Cursor BG / Codex CLI / Continue.dev | ✅ | ✅ | ✅ | ✅ |
| Raw LLM + bash | ✅ | ✅ | ✅ | ✅ |

Required minimum: **shell command + file read.** Both exist in all modern harnesses.

---

## 13. Security, Limitations, and Race Conditions

- **File permissions:** pid/sock/jsonl/out all `0600` (Windows: `%TEMP%` files under
  default ACL; pipe `\\.\pipe\hbmon-<uuid>` created with a current-user-only DACL —
  unix `0600` equivalent, no cross-user connect — TASK-041); symlinks rejected at open (`O_EXCL` + ownership check).
- **Path injection:** paths containing `..` are rejected; `--uuid` is additionally charset-locked (`1..=64` chars, `[A-Za-z0-9_-]` — TASK-027; production uuid is 16 hex chars / 64-bit).
- **Resource limit:** daemon ~10MB RAM, <%1 CPU idle target.
- **Double spawn:** if a live socket exists, `MONITOR_ALREADY_EXISTS`.
- **Zombie:** 60s linger after daemon exit (for wait/status), then pid/sock cleanup; JSONL + .out remain. Old files are removed with `hbmon cleanup`.
- **Early death (<100ms):** reported via exit code, no query needed.
- **Known limits:** deep trees (>1000 processes) capped; metrics 1Hz by default; JSONL 100MB FIFO cap; container PID namespace deferred to v1.5.

---

## 14. Future Work (v1.5 / v2 / v3)

### 14.1 v1.0 (MVP) — Status

- [x] Full Linux monitoring + macOS process supervision
- [x] Daemonization, UDS JSON-RPC, JSONL, exit mapping
- [x] Stall/OOM/dep-missing/timeout (ETA to come — `eta_sec` not returned in v1)
- [x] CLI: watch, status, wait, exec, kill, shutdown, cleanup, list, log, events
  (+`status --compact`, `wait --until` vocabulary, `cleanup --dir`, `watch --max-log-mb`,
  `log --event`, `list --state`/`--live-only`, `events` stream — TASK-005/016/017/023/024/029/046)
- [x] 92 unit + 21 integration (+2 sandbox-ignore) + 3 drift + 1 smoke tests
  (2026-09-18, `cargo test --locked -j2` green; contract lock TASK-028)

### 14.2 v1.5

- [x] Container-aware (cgroup v2) — pulled into v1 (graceful fallback, pre-TASK)
- [x] Custom patterns (`~/.config/hbmon/patterns.json`; JSON instead of the draft's TOML — zero-dep, TASK-001) — pulled into v1
- [ ] `journalctl` OOM support, supervised mode
- [ ] TUI (Ratatui)

### 14.3 v2

- [x] Windows port — TASK-006 (named pipe transport + Win32 inspector +
  Job-Object detach/kill), on `windows-latest` CI; 45 unit + 7 integration green
- [ ] Multi-build first-class (scan suffices in v1)
- [ ] Prometheus exporter, Web UI

### 14.4 v3

- [ ] Plugin (Lua/WASM — carefully), distributed monitoring, ML ETA

### 14.5 Won't Do

- ❌ Build orchestration, cache layer, LLM model selection, IDE integration

---

## 15. Open Questions and Discussion

1. Should stall thresholds be customized per build type (compile/link/network)?
2. ETA: simple EMA or phase-weighted?
3. Is the default metric interval of 1Hz sufficient?
4. Should the log rotation cap be user-defined?
5. Is dmesg sufficient for OOM (container privileges)?
6. Should the stall score be float or boolean?
7. License: dual MIT/Apache-2.0 chosen (2026-09-09).
8. Test strategy: unit (mock-free, real regex/stall/CPU) + integration (real daemon) adopted.
9. v0.1.0/v0.1.1 maintenance closed on 2026-09-13: TASK-001..030
   (exec ephemeral handshake, incremental dep-scan, Windows port, wait--until,
   status-compact, sock-gc/list, log CLI + cap, dep-patterns, uuid lock,
   contract lock, log/list filters, crate hygiene, crates.io release).
10. v0.2.0 (TASK-047/048): exec --timeout-sec watchdog, terminal wait return,
    sock cleanup security fix, log tail_filter memory bound, kill/shutdown
    exit code consistency, O_NOFOLLOW, dead code removal, P0/S1 patches.
11. v0.2.1 (TASK-050/051): `exit.summary` — last ~2KB preview of `.out`
    (optional, cut at line boundary); additive, no breaking changes.
12. v0.2.2 (TASK-053/054/055): Windows detach linger (`HBMON_DETACHED_CHILD`),
    macOS UDS path helper, `serve_error` diagnosis; additive, no breaking changes.

---

## 16. References

- `aydemir/opencode-plugins` — `build-mon.mjs`, `hbmon-build-mon.mjs` adapter, `opencode-settle-noticer`, DHS PTC (existence proof; `.sh` predecessors in `scripts/archive/`)
- POSIX: `setsid(2)`, `fork(2)`; Linux: `proc(5)`, `oom(7)`; macOS: `libproc.h`, `proc_pidinfo(3)`
- Rust: `clap` 4, `serde`/`serde_json` 1, `libc` 0.2, `regex` 1, `once_cell` 1, `rand` 0.8
- Build systems: Recursive Make Considered Harmful (Miller, 1997); Build Systems à la Carte (Mokhov et al., 2018)

---

**End of document. v0.1 Draft + v1/v2 MVP outcome notes (in sync with v0.2.2, TASK-031/047/048/050/053/054). Open for feedback and revision.**
