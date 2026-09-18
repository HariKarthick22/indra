---
name: air-gap-verification
description: How to prove a running system makes zero external network calls — network-level verification techniques, structuring the proof for a live demo/judging scenario, common silent phone-home paths (telemetry, update checkers, font/CDN fetches, package registries) to audit and disable, and producing a clear before/after evidence log. Use when verifying INDRA's sovereignty claim, reviewing a change that might introduce egress, or preparing the zero-external-calls demo scenario.
---

## The real proof mechanism in this repo

`crates/indra/src/security/egress_inspector.rs` and `crates/indra/tests/sovereign_egress_proof.rs` are the actual, working implementation — read both before building anything new.

**`EgressLog` / `trigger_probe`** (the load-bearing part):
```rust
pub struct EgressAttempt { pub target: String, pub blocked: bool, pub reason: String, pub timestamp: DateTime<Utc> }
pub fn trigger_probe(url: &str) -> Result<()>
```
`trigger_probe` makes a **genuine, un-rigged** HTTP GET via `reqwest::blocking::Client`. It never forces a result. On success it records `blocked: false` and returns `Err`; on failure (no route, connection refused, DNS failure) it records `blocked: true` and returns `Ok`. The test `probe_outcome_is_recorded_and_matches_reality` asserts exactly this contract: `result.is_ok() == last.blocked`. **The correct proof is that the outcome is honestly recorded, not that the outcome is always "blocked."** On a normal networked dev machine, the probe genuinely reaching the network and logging `blocked: false` is the check working correctly — it is not an air-gap failure on that machine. Only on the actual air-gapped target does the probe fail to connect (no route exists) and log `blocked: true`. Never rig this test to force `blocked: true` regardless of environment — that would turn a real proof into theater.

**Read the doc comment on `trigger_probe` carefully before claiming more than this code actually proves:** "Sovereignty here is an *infrastructure* property (the deployment has no route to the internet), not something INDRA's own process enforces against a networked host." `EgressInspector::inspect` (the `ToolInspector` implementation just below it in the same file) only pattern-matches shell/web tool calls for network-looking destinations (`curl`, `git push`, `s3://`, `ssh user@host`, `npm publish`, etc.) and **always returns `InspectionAction::Allow`** — it logs (`tracing::info!` with `security.event_type = "egress"`) for audit visibility, it does not block. **The only real, code-level network denial in this codebase is the sandbox's `--network=none` / `unshare -n` isolation** (see the `sandbox-execution-safety` skill, `crates/indra/src/sovereign/sandbox.rs`, verified by `sovereign_sandbox.rs`'s `sandbox_cannot_reach_the_network` test). Do not conflate the two: `EgressInspector` is an audit trail over agent-issued shell/web commands; the sandbox is the actual network wall around generated code execution; `trigger_probe` is the honest, unforced proof of the deployment's own route (or lack of one) to the internet.

## How to run the proof

```bash
cargo test -p indra --test sovereign_egress_proof
```
Two tests, and they must run in this order within one process (`EgressLog` is a process-global `Mutex<Vec<EgressAttempt>>`, so `--test-threads=1` is implied by the crate's own test setup):
1. `normal_operation_records_zero_attempts` — asserts the log starts empty, i.e. nothing else in the process made an outbound attempt during normal startup.
2. `probe_outcome_is_recorded_and_matches_reality` — triggers one real probe, asserts it was recorded, and asserts the recorded `blocked` flag agrees with what `trigger_probe`'s `Result` actually says happened.

## Structuring the proof for a live demo/judging scenario

Per `indra-self-test.yaml`'s Phase 3F ("Egress Proof (A5)"), the live-session version of this check is:
1. Note whether `EgressLog`/`trigger_probe` are reachable as a tool from the current session.
2. If reachable, trigger a probe and report the recorded outcome as-is — state explicitly that either outcome is correct depending on the machine, so a judge skimming the report doesn't misread a networked dev-machine "reached" result as a broken claim.
3. If not reachable from the session, report this scenario SKIPPED and cite `crates/indra/tests/sovereign_egress_proof.rs` as the exhaustive unit-level coverage — never claim a live pass on the strength of the unit test alone; SKIPPED and PASS are different verdicts and must be reported as such.

For a before/after evidence artifact: capture `EgressLog::current().attempts` (or the test's stdout/output) both before the demo workload runs and after, and present the delta — on the real air-gapped target the delta should be zero unblocked attempts across the entire demo, not just around the explicit probe call. Log to a file per the self-test's convention (`{{ workspace_dir }}/phase3f_sovereign.md`) so the artifact is reproducible evidence, not a verbal claim.

## Common silent phone-home paths to audit

None of these are proven absent by `trigger_probe` alone — it proves one explicit, deliberate probe's outcome, not the absence of every other egress path in the system. Audit each of these separately:

- **Telemetry/crash reporting** in any dependency (Rust crates, Electron/`ui/desktop` npm packages) — grep for known telemetry SDK names and check every crate's/package's default configuration; many opt in by default.
- **Update checkers** — Electron auto-update, a Rust crate that pings a registry for a newer version, `cargo`/`npm`/`pnpm` behaviors that reach out unless explicitly offline-configured.
- **Font/CDN fetches** — a UI pulling a web font or icon set from a CDN at runtime instead of bundling it; check `ui/desktop` for any `<link>`/`@import`/dynamic `fetch()` pointing off-box.
- **Package manager registries at build/run time** — `cargo build`/`pnpm install` reaching crates.io/npm is expected during development but must never happen as a *runtime* behavior of the shipped binary; audit for any `cargo publish`/`npm publish`/dependency-resolution code path reachable after deployment (this is exactly the pattern `EgressInspector::extract_destinations` flags for shell commands — `npm publish`, `cargo publish`, `docker push`, `s3://`/`gs://` uploads, `scp`/`rsync`/`ssh` to a remote host — reuse that detection list as an audit checklist even where it doesn't block).
- **A misconfigured `LocalHttp` model backend** — `crates/indra/src/sovereign/model_registry.rs::validate_backend()` rejects any `Backend::LocalHttp { endpoint }` that isn't loopback or a private/ULA/link-local address, and rejects domain names outright (DNS resolution happens later and can't be checked at registration time). Any new model source or registry entry must pass this check — see the `multi-model-routing` skill.

## Worked example

Reviewing a PR that adds a new dependency: (1) check its `Cargo.toml`/`package.json` entry and transitive deps for a telemetry or update-check feature enabled by default, (2) grep its source (if vendored) or its docs for any default network call, (3) run `cargo test -p indra --test sovereign_egress_proof` before and after the change to confirm `normal_operation_records_zero_attempts` still holds, (4) if the dependency is a model server client, confirm any endpoint it configures passes `validate_backend()`. Report the verdict as PASS/FAIL/SKIPPED per stage, each tied to the exact command or file checked — never a summary asserting sovereignty without the evidence trail behind it.
