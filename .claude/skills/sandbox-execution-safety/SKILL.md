---
name: sandbox-execution-safety
description: How to design and review a sandboxed code-execution environment for an agentic system — filesystem/network/process isolation boundaries, default tool permissions (what's granted vs. denied), verifying generated code actually ran and produced its claimed output before reporting success, and safe handling of timeouts/failures. Use when adding or reviewing a code-execution or automation tool in INDRA, or investigating a suspected sandbox escape or over-broad permission grant.
---

## The real isolation code in this repo

`crates/indra/src/sovereign/sandbox.rs` is the actual, working sandbox backend — read it before designing anything new:

- **`SandboxBackend` trait** — `run(code: &str, timeout: Duration) -> Result<SandboxOutput>`, where `SandboxOutput { stdout, stderr, exit_code, timed_out }`.
- **`LinuxNamespace`** — runs `unshare -n -p -f --mount-proc sh -c '<ulimit + timeout + python3>'`. `unshare -n` isolates the network namespace only; it does not bound memory. That's why the shell command also sets `ulimit -v 1048576` (1 GiB address-space cap in KB) before `exec timeout <secs> python3 -c <code>` — without this a fork bomb or memory-exhausting script runs unchecked until the wall-clock timeout fires, risking host OOM in that window.
- **`DockerNoNetwork`** — `docker run --rm --network=none --memory=512m --cpus=1 python:3.11-slim timeout <secs> python -c <code>`. Explicit network, memory, and CPU caps, all enforced by the container runtime rather than by convention.
- **`detect_backend()`** — Linux namespace preferred, falls back to Docker if `unshare` isn't available, and returns `Err` (refusing to execute unisolated) if neither backend is available. **Never add a fallback path that executes code without an isolated backend** — a missing sandbox must fail closed, not run bare.
- **Timeout detection**: GNU `timeout`'s exit code 124 means it killed the child; `SandboxOutput.timed_out` is derived from exactly that, not guessed from wall-clock measurement in the caller.

Real adversarial tests exist in `crates/indra/tests/sovereign_sandbox.rs` — read these before writing new ones, they're the template:

- `sandbox_cannot_reach_the_network` — runs `socket.create_connection(("1.1.1.1", 53), timeout=5)` inside the sandbox and asserts stdout contains `"BLOCKED"`, never `"REACHED"`.
- `sandbox_enforces_timeout` — `time.sleep(60)` against a 2-second timeout, asserts `out.timed_out`.
- `sandbox_rejects_a_memory_bomb` — allocates a 2 GiB `bytearray` and **writes one distinct byte per page** (`for i in range(0, len(x), 4096): x[i] = 1`) before asserting a nonzero exit code. The per-page write matters: a zero-filled allocation can be satisfied by the kernel's shared zero page without committing real physical memory, so `ulimit -v`/cgroup accounting never sees it — a memory-limit test that doesn't touch every page is not actually testing the limit.

## Design checklist for any new code-execution or automation tool

1. **Isolation boundary, enforced by the runtime, not by prompt discipline.** Filesystem scope (a scoped workspace directory, never the whole filesystem), network posture (`--network=none` / `unshare -n`, confirmed by an actual connection-attempt test, not just documentation), and resource limits (CPU, memory via `ulimit -v` or `--memory`, wall-clock timeout, process/thread count) must each be a structural denial, not a comment telling the model not to do something.
2. **Fail closed on any ambiguity.** A crash, a timeout, an unavailable sandbox backend, or an ambiguous permission request must terminate the execution and surface a clear error — never fall back to running unsandboxed or with elevated scope. This is `detect_backend()`'s own behavior (`bail!("no isolated sandbox available; refusing to execute code unisolated")`) and the bar every new execution path should match.
3. **Confirm and log through the existing chokepoints.** Route new execution capability through `crates/indra/src/agents/tool_confirmation_router.rs` and `tool_confirmation_coordinator.rs` so a human can see, before or after the fact, exactly what code ran and what it touched. A tool-confirmation bypass under a specific extension config is a real, recurring regression class — check the actual execution path fires confirmation, not just a happy-path unit test.
4. **Review generated code for the classic escapes** before or as it executes: path traversal out of the scoped workspace, subprocess spawning that bypasses the sandbox wrapper entirely, environment-variable or credential exfiltration, resource exhaustion. Mitigations should be structural (denied by the runtime) — for example, the memory-bomb test above exists because a mitigation that's merely "the model shouldn't write huge arrays" is not a mitigation.
5. **Scope any widened grant to one execution, never persistently.** A legitimate task needing broader filesystem access (e.g. reading a shared knowledge-base directory while writing calculation output) gets the specific paths it needs for that task — the default sandbox boundary never widens as a side effect.

## Verifying claimed success before reporting it

Do not report "the script ran successfully" from an exit code alone:

1. Confirm `exit_code == 0` **and** `!timed_out` — a killed process can still report a stale or empty stdout that looks like success if only the last output line is checked.
2. Confirm the claimed output artifact actually exists and is non-empty where the task was supposed to produce a file (mirror `crates/indra-mcp/tests/sovereign_approval_note.rs`'s pattern: assert the file exists, assert its size is nonzero, don't trust a nonzero exit code as a proxy for file correctness).
3. For a negative test (verifying a boundary holds), the *absence* of the bad outcome plus a *positive* signal that the code actually ran (e.g. stdout containing `"BLOCKED"`, not just an empty stdout that could mean the script never executed) is the correct success condition — an empty result is ambiguous, not passing.

## Worked example: adding a new "run this script" tool

Given a request to wire a new automation script capability into `platform_extensions/`: (1) confirm it dispatches through `code_execution.rs`'s existing sandbox rather than a new `Command::new` call, (2) confirm the scoped workspace directory is task-specific, not the repo root or home directory, (3) confirm a negative test exists proving an outbound socket attempt from inside the new tool is blocked — reuse `sandbox_cannot_reach_the_network`'s pattern rather than assuming coverage carries over from the underlying `sandbox.rs`, since the *dispatch path* into the sandbox is a separate thing to verify from the sandbox mechanism itself, (4) confirm tool-confirmation fires on this path, not just on the pre-existing code-execution tool.
