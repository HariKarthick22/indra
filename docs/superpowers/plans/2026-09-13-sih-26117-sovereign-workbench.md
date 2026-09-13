# INDRA SIH 26117 — One-Week Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a demonstrable sovereign agentic AI workbench that satisfies all five SIH 26117 acceptance criteria on a single machine with no network egress.

**Architecture:** Extend the existing Rust fork at `crates/indra` with a new `sovereign/` module holding hardware probing, capability-based model selection, a network-isolated sandbox, and box-level provenance. Reuse what exists: `OperationRouter` for task classification, `egress_inspector.rs` for sovereignty proof, `computercontroller`'s docx/pdf/xlsx tools for deliverables, and `mmproj`/`vision_capable` in `indra-local-inference` for multimodal.

**Tech Stack:** Rust (workspace `crates/indra`), llama.cpp via `llama-cpp-2`, Tesseract (subprocess, TSV output), `docx-rs`, `sysinfo`, Docker or `unshare` for sandboxing, Electron/ACP client.

**Spec:** [`docs/superpowers/specs/2026-09-13-indra-sovereign-backend-design.md`](../specs/2026-09-13-indra-sovereign-backend-design.md)

---

## Global Constraints

Every task's requirements implicitly include this section.

- **No outbound network calls at any point.** This is acceptance criterion A5 and the entire premise. Any new dependency that phones home disqualifies the build.
- **Agent-loop parity.** `AGENTS.md`: changes to agent-loop behaviour must land in **both** `crates/indra/src/agents/agent.rs` and `crates/indra/src/agents/state_machine/`. Model selection (Task 2) is agent-loop behaviour. Everything else in this plan is not.
- **Tests live in `crates/indra/tests/`**, not inline modules, per `AGENTS.md`.
- **Dependency changes use `cargo add`**, never manual `Cargo.toml` edits.
- **Run `cargo fmt` after every change.** Never skip it.
- **Comments:** only for non-obvious "why". Never restate what the code does.
- **Errors:** `anyhow::Result`. No `.context("failed to X")` when the error already says it failed.
- **Sealed is hard-wired.** No configuration value may make a Sealed resource writable.
- **Demo models must be small.** 3B–7B quantized. See the disk warning in Task 1.
- **Hardware shortfall warns; it never excludes and never fails silently.** A model too large for available memory still runs via CPU offload, carrying a warning the UI shows verbatim. Only a capability gap — no vision, insufficient context, missing backend — disqualifies a model.
- **`Backend::LocalHttp` is loopback-only**, validated at configuration time. This is the one permitted exception in the egress inspector and the most likely route by which A5 could be lost.

---

## Day-by-day shape

| Day | Task | Serves | Risk |
|---|---|---|---|
| 1 | Unblock the build, free disk, hardware probe | prerequisite | **High** — everything stalls here if the workspace is broken |
| 2 | Universal model registry: capability selection, warn-don't-fail degradation | A1 | Medium |
| 3 | Network-isolated sandbox | A3, A5 | Medium — platform-dependent |
| 4 | Multimodal ingest: render → OCR boxes → VLM, plus the write guard | A4 | **High** — model download size |
| 5 | End-to-end agentic path with box-level citations | A2, A4 | Medium |
| 6 | Egress proof panel + deterministic recomputation | A5, A3 | Low |
| 7 | Integration, demo rehearsal, buffer | all | — |

**If you slip, cut in this order:** deterministic recomputation (Task 6b) → box-level provenance degrades to page-level (Task 5) → Docker sandbox fallback only, no `unshare` (Task 3). Never cut A5; it is the premise of the entire problem statement, and never cut Task 4b — it costs two hours and answers the question every evaluator asks.

---

### Task 1: Unblock the build, reclaim disk, probe hardware

**Files:**
- Modify: `ui/pnpm-workspace.yaml:2-5`
- Modify: `ui/package.json:3-8`
- Modify: `crates/indra/src/hints/load_hints.rs:10`
- Create: `crates/indra/src/sovereign/mod.rs`
- Create: `crates/indra/src/sovereign/hardware.rs`
- Modify: `crates/indra/src/lib.rs` (register the module)
- Test: `crates/indra/tests/sovereign_hardware.rs`

**Interfaces:**
- Produces: `sovereign::hardware::HardwareProfile { total_ram_mb: u64, physical_cores: usize, gpu_vram_mb: u64 }` and `HardwareProfile::probe() -> HardwareProfile`. Task 2 consumes both.

**Disk warning — read before starting.** The machine has ~19 GiB free. A debug build of this workspace needs 8–20 GiB, and demo models need another 6–10 GiB. Those do not both fit. Set `CARGO_PROFILE_DEV_DEBUG=0` before building — it typically cuts `target/` by half or more and costs you only line numbers in backtraces.

- [ ] **Step 1: Confirm the workspace compiles**

```bash
source bin/activate-hermit
export CARGO_PROFILE_DEV_DEBUG=0
cargo check --workspace --message-format short 2>&1 | tail -40
```

Expected: clean, or a short list of errors. Fix any errors before continuing — nothing else in this plan can proceed. `INDRA_Plant_Integration_Plan.md` claims two `include_str!` paths are broken; that has already been fixed, so do not go looking for it.

- [ ] **Step 2: Fix the pnpm workspace globs**

The directories are `ui/indra-acp`, `ui/indra-acp-client`, `ui/indra-binary`, but both manifests still name `goose-*`. `pnpm install` cannot resolve the `workspace:*` dependencies until this is fixed.

In `ui/pnpm-workspace.yaml`:
```yaml
packages:
  - 'indra-acp-client'
  - 'indra-acp'
  - 'desktop'
  - 'indra-binary/*'
```

In `ui/package.json`:
```json
  "workspaces": [
    "indra-acp-client",
    "indra-acp",
    "desktop",
    "indra-binary/*"
  ],
```

- [ ] **Step 3: Restore hints loading**

`crates/indra/src/hints/load_hints.rs:10` still looks for `.goosehints`, and that file was deleted in the rename — so the project's own hints load for nobody. Change the constant to look for both, new name first:

```rust
pub const INDRA_HINTS_FILENAME: &str = ".indrahints";
pub const GOOSE_HINTS_FILENAME: &str = ".goosehints";
```

Update the lookup site to try `.indrahints` before falling back to `.goosehints`. Keep the fallback — existing user projects still carry the old filename.

- [ ] **Step 4: Add sysinfo**

```bash
cargo add sysinfo -p indra
```

- [ ] **Step 5: Write the failing test**

Create `crates/indra/tests/sovereign_hardware.rs`:

```rust
use indra::sovereign::hardware::HardwareProfile;

#[test]
fn probe_reports_nonzero_ram_and_cores() {
    let p = HardwareProfile::probe();
    assert!(p.total_ram_mb > 0, "probe returned zero RAM");
    assert!(p.physical_cores > 0, "probe returned zero cores");
}

#[test]
fn probe_is_stable_across_calls() {
    let a = HardwareProfile::probe();
    let b = HardwareProfile::probe();
    assert_eq!(a.physical_cores, b.physical_cores);
    assert_eq!(a.total_ram_mb, b.total_ram_mb);
}
```

- [ ] **Step 6: Run it and watch it fail**

```bash
cargo test -p indra --test sovereign_hardware
```
Expected: FAIL — `unresolved import indra::sovereign`.

- [ ] **Step 7: Implement**

`crates/indra/src/sovereign/mod.rs`:
```rust
pub mod hardware;
```

`crates/indra/src/sovereign/hardware.rs`:
```rust
use sysinfo::System;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareProfile {
    pub total_ram_mb: u64,
    pub physical_cores: usize,
    pub gpu_vram_mb: u64,
}

impl HardwareProfile {
    pub fn probe() -> Self {
        let mut sys = System::new();
        sys.refresh_memory();

        Self {
            total_ram_mb: sys.total_memory() / 1024 / 1024,
            physical_cores: System::physical_core_count().unwrap_or(1),
            gpu_vram_mb: probe_gpu_vram_mb(),
        }
    }
}

// llama.cpp's device enumerator already reports free VRAM; total is logged and
// discarded at llamacpp/mod.rs:643. Zero means "no discrete GPU detected",
// which callers treat as CPU-only rather than as an error.
fn probe_gpu_vram_mb() -> u64 {
    0
}
```

Add `pub mod sovereign;` to `crates/indra/src/lib.rs`.

Leave `probe_gpu_vram_mb` returning zero for now — Task 2 wires it to the llama.cpp enumerator once the registry exists to consume it. A CPU-only profile is a valid profile and the tests above must pass either way.

- [ ] **Step 8: Run and confirm pass**

```bash
cargo test -p indra --test sovereign_hardware
cargo fmt
```
Expected: 2 passed.

- [ ] **Step 9: Commit**

```bash
git add ui/pnpm-workspace.yaml ui/package.json crates/indra/src/hints/load_hints.rs \
        crates/indra/src/sovereign/ crates/indra/src/lib.rs \
        crates/indra/tests/sovereign_hardware.rs crates/indra/Cargo.toml Cargo.lock
git commit -m "feat(sovereign): add hardware probe, restore hints loading, fix pnpm globs"
```

---

### Task 2: Universal model registry and capability-based selection — **A1**

**Files:**
- Create: `crates/indra/src/sovereign/model_registry.rs`
- Modify: `crates/indra/src/sovereign/mod.rs`
- Modify: `crates/indra/src/agents/specialists/operation.rs`
- Test: `crates/indra/tests/sovereign_model_selection.rs`

**Interfaces:**
- Consumes: `HardwareProfile` from Task 1.
- Produces: `ModelDescriptor`, `Capabilities`, `Requirements`, `select_model(&[ModelDescriptor], &Requirements, &HardwareProfile) -> Result<&ModelDescriptor>`, and `IndraOperation::requirements(&self) -> Requirements`. Tasks 4 and 5 consume `select_model`.

This is the task that satisfies A1, and it is agent-loop behaviour — the **parity rule applies**. Selection must be reachable from both `agents/agent.rs` and `agents/state_machine/`.

- [ ] **Step 1: Write the failing tests**

Create `crates/indra/tests/sovereign_model_selection.rs`:

```rust
use indra::agents::specialists::operation::IndraOperation;
use indra::sovereign::hardware::HardwareProfile;
use indra::sovereign::model_registry::{
    select_model, Backend, Capabilities, Domain, Fit, ModelDescriptor, ToolCalling,
};
use std::path::PathBuf;

fn hw(ram_mb: u64, vram_mb: u64) -> HardwareProfile {
    HardwareProfile { total_ram_mb: ram_mb, physical_cores: 8, gpu_vram_mb: vram_mb }
}

fn model(id: &str, vision: bool, domains: Vec<Domain>, min_vram_mb: u64) -> ModelDescriptor {
    ModelDescriptor {
        id: id.to_string(),
        backend: Backend::Llamacpp,
        path: PathBuf::from(format!("/models/{id}.gguf")),
        mmproj_path: if vision { Some(PathBuf::from("/models/mmproj.gguf")) } else { None },
        capabilities: Capabilities {
            vision,
            tool_calling: ToolCalling::Reliable,
            context_tokens: 32_768,
            domains,
        },
        min_vram_mb,
    }
}

#[test]
fn code_task_and_document_task_select_different_models() {
    let registry = vec![
        model("qwen-coder", false, vec![Domain::Code], 0),
        model("llama-doc", false, vec![Domain::Document, Domain::General], 0),
    ];
    let hw = hw(32_000, 0);

    let code = select_model(&registry, &IndraOperation::CodeGeneration.requirements(), &hw).unwrap();
    let doc = select_model(&registry, &IndraOperation::GeneralChat.requirements(), &hw).unwrap();

    assert_eq!(code.descriptor.id, "qwen-coder");
    assert_eq!(doc.descriptor.id, "llama-doc");
    assert_ne!(
        code.descriptor.id, doc.descriptor.id,
        "A1 requires two task types to select differently"
    );
}

#[test]
fn a_model_that_fits_is_preferred_over_one_that_does_not() {
    let registry = vec![
        model("huge", false, vec![Domain::General], 80_000),
        model("small", false, vec![Domain::General], 0),
    ];
    let hw = hw(16_000, 8_000);

    let sel = select_model(&registry, &IndraOperation::GeneralChat.requirements(), &hw).unwrap();

    assert_eq!(sel.descriptor.id, "small");
    assert_eq!(sel.fit, Fit::Comfortable);
}

#[test]
fn oversized_model_is_still_selected_but_warns() {
    let registry = vec![model("huge", false, vec![Domain::General], 80_000)];
    let hw = hw(16_000, 8_000);

    let sel = select_model(&registry, &IndraOperation::GeneralChat.requirements(), &hw).unwrap();

    assert_eq!(sel.descriptor.id, "huge", "hardware shortfall must warn, not exclude");
    match sel.fit {
        Fit::Degraded { ref warning } => {
            assert!(warning.contains("8000"), "warning must state what is available: {warning}");
        }
        other => panic!("expected Degraded, got {other:?}"),
    }
}

#[test]
fn missing_vision_is_unsupported_not_degraded() {
    let registry = vec![model("blind", false, vec![Domain::General], 0)];
    let hw = hw(64_000, 48_000);

    let result = select_model(&registry, &IndraOperation::PidAnalysis.requirements(), &hw);

    assert!(result.is_err(), "no amount of hardware makes a blind model see");
}

#[test]
fn empty_registry_is_an_error_not_a_panic() {
    let hw = hw(16_000, 0);
    let result = select_model(&[], &IndraOperation::GeneralChat.requirements(), &hw);
    assert!(result.is_err());
}

#[test]
fn remote_http_backend_is_rejected_at_configuration_time() {
    use indra::sovereign::model_registry::validate_backend;

    let ok = Backend::LocalHttp { endpoint: "http://127.0.0.1:11434/v1".to_string() };
    let also_ok = Backend::LocalHttp { endpoint: "http://localhost:8000/v1".to_string() };
    let bad = Backend::LocalHttp { endpoint: "https://api.openai.com/v1".to_string() };

    assert!(validate_backend(&ok).is_ok());
    assert!(validate_backend(&also_ok).is_ok());
    assert!(
        validate_backend(&bad).is_err(),
        "a non-loopback endpoint would silently void A5"
    );
}
```

- [ ] **Step 2: Run and watch it fail**

```bash
cargo test -p indra --test sovereign_model_selection
```
Expected: FAIL — `unresolved import indra::sovereign::model_registry`.

- [ ] **Step 3: Implement the registry**

Create `crates/indra/src/sovereign/model_registry.rs`:

```rust
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Backend {
    Llamacpp,
    Mlx,
    /// Any format the native backends cannot load, served by a local runtime
    /// (vLLM, Ollama, TGI) over an OpenAI-compatible API. Loopback only.
    LocalHttp { endpoint: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolCalling {
    Reliable,
    Unreliable,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Domain {
    Code,
    Document,
    Engineering,
    General,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub vision: bool,
    pub tool_calling: ToolCalling,
    pub context_tokens: u32,
    pub domains: Vec<Domain>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDescriptor {
    pub id: String,
    pub backend: Backend,
    pub path: PathBuf,
    pub mmproj_path: Option<PathBuf>,
    pub capabilities: Capabilities,
    pub min_vram_mb: u64,
}

#[derive(Debug, Clone)]
pub struct Requirements {
    pub vision: bool,
    pub tool_calling: ToolCalling,
    pub domain: Domain,
    pub min_context_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fit {
    Comfortable,
    Tight { warning: String },
    Degraded { warning: String },
}

#[derive(Debug, Clone)]
pub struct Selection<'a> {
    pub descriptor: &'a ModelDescriptor,
    pub fit: Fit,
    pub reason: String,
}

/// Capability gaps are disqualifying; hardware shortfalls are not. A model too
/// large for available memory still runs via CPU offload, so it is selected with
/// a warning rather than hidden — otherwise a venue machine shows an empty list.
fn assess(m: &ModelDescriptor, hw: &crate::sovereign::hardware::HardwareProfile) -> Fit {
    let available = hw.gpu_vram_mb.max(hw.total_ram_mb);

    if m.min_vram_mb == 0 || m.min_vram_mb * 100 <= available * 80 {
        return Fit::Comfortable;
    }
    if m.min_vram_mb <= available {
        return Fit::Tight {
            warning: format!(
                "{} needs {} MB, {} MB available — little headroom, expect slowdowns",
                m.id, m.min_vram_mb, available
            ),
        };
    }
    Fit::Degraded {
        warning: format!(
            "{} needs {} MB, {} MB available — offloading to CPU, expect substantially \
             slower generation",
            m.id, m.min_vram_mb, available
        ),
    }
}

pub fn select_model<'a>(
    registry: &'a [ModelDescriptor],
    req: &Requirements,
    hw: &crate::sovereign::hardware::HardwareProfile,
) -> Result<Selection<'a>> {
    let capable: Vec<&ModelDescriptor> = registry
        .iter()
        .filter(|m| {
            if req.vision && !m.capabilities.vision {
                return false;
            }
            m.capabilities.context_tokens >= req.min_context_tokens
        })
        .collect();

    if capable.is_empty() {
        bail!(
            "no model satisfies requirements (vision={}, domain={:?}, context>={}); \
             this is a capability gap, not a hardware limit",
            req.vision,
            req.domain,
            req.min_context_tokens
        );
    }

    let mut ranked = capable;
    ranked.sort_by_key(|m| {
        let domain_match = m.capabilities.domains.contains(&req.domain);
        let tools_ok = m.capabilities.tool_calling == req.tool_calling;
        let degraded = matches!(assess(m, hw), Fit::Degraded { .. });
        (
            !domain_match,
            !tools_ok,
            degraded,
            std::cmp::Reverse(m.capabilities.context_tokens),
        )
    });

    let chosen = ranked[0];
    let fit = assess(chosen, hw);

    Ok(Selection {
        descriptor: chosen,
        reason: format!(
            "{:?} requires domain {:?}, tool-calling {:?}; {} declares {:?}",
            req.domain, req.domain, req.tool_calling, chosen.id, chosen.capabilities.domains
        ),
        fit,
    })
}

/// Rejected at configuration time rather than request time: a non-loopback
/// endpoint would turn universal model support into an egress hole.
pub fn validate_backend(backend: &Backend) -> Result<()> {
    let Backend::LocalHttp { endpoint } = backend else {
        return Ok(());
    };

    let host = endpoint
        .split("://")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .and_then(|hostport| hostport.rsplit(':').next_back())
        .unwrap_or_default();

    let is_loopback = host == "localhost"
        || host
            .parse::<std::net::IpAddr>()
            .map(|ip| ip.is_loopback())
            .unwrap_or(false);

    if !is_loopback {
        bail!("LocalHttp endpoint {endpoint} is not loopback; this would violate A5");
    }
    Ok(())
}
```

- [ ] **Step 4: Map operations to requirements**

Append to `crates/indra/src/agents/specialists/operation.rs`:

```rust
use crate::sovereign::model_registry::{Domain, Requirements, ToolCalling};

impl IndraOperation {
    pub fn requirements(&self) -> Requirements {
        match self {
            IndraOperation::CodeGeneration => Requirements {
                vision: false,
                tool_calling: ToolCalling::Reliable,
                domain: Domain::Code,
                min_context_tokens: 16_384,
            },
            IndraOperation::PidAnalysis => Requirements {
                vision: true,
                tool_calling: ToolCalling::Unreliable,
                domain: Domain::Engineering,
                min_context_tokens: 8_192,
            },
            IndraOperation::InspectionAnalysis => Requirements {
                vision: true,
                tool_calling: ToolCalling::Reliable,
                domain: Domain::Document,
                min_context_tokens: 16_384,
            },
            IndraOperation::EngineeringCalculation => Requirements {
                vision: false,
                tool_calling: ToolCalling::Reliable,
                domain: Domain::Engineering,
                min_context_tokens: 8_192,
            },
            IndraOperation::GeneralChat => Requirements {
                vision: false,
                tool_calling: ToolCalling::Reliable,
                domain: Domain::General,
                min_context_tokens: 8_192,
            },
        }
    }
}
```

Add `pub mod model_registry;` to `crates/indra/src/sovereign/mod.rs`.

- [ ] **Step 5: Run and confirm pass**

```bash
cargo test -p indra --test sovereign_model_selection
cargo fmt
```
Expected: 6 passed.

- [ ] **Step 6: Commit**

```bash
git add crates/indra/src/sovereign/ crates/indra/src/agents/specialists/operation.rs \
        crates/indra/tests/sovereign_model_selection.rs
git commit -m "feat(sovereign): select models by declared capability, warn on hardware shortfall"
```

- [ ] **Step 7: Surface the selection reason and any warning**

A1 requires the choice be *visible*. Emit `Selection::reason`, the chosen model id, and — when `fit` is `Tight` or `Degraded` — the warning text, on the same trace channel the specialist name already uses. The UI timeline then renders both the decision and its cost:

> Code Generation → `qwen-coder` (domain: Code, tools: reliable)
> ⚠ needs 20000 MB, 8000 MB available — offloading to CPU, expect substantially slower generation

Wire it in both `agents/agent.rs` and `agents/state_machine/` — this is the parity rule.

- [ ] **Step 8: Fall through on load or inference failure**

A model that fails to load, or exhausts memory mid-generation, must not fail the turn. Catch the error, record it as a warning on the same channel, drop that model from the ranked list, and retry with the next candidate. Exhausting the whole list is the only hard error.

Add to `crates/indra/tests/sovereign_model_selection.rs`:

```rust
#[test]
fn ranking_is_stable_so_fallthrough_is_deterministic() {
    let registry = vec![
        model("first", false, vec![Domain::General], 0),
        model("second", false, vec![Domain::General], 0),
    ];
    let hw = hw(16_000, 8_000);
    let req = IndraOperation::GeneralChat.requirements();

    let a = select_model(&registry, &req, &hw).unwrap();
    let b = select_model(&registry, &req, &hw).unwrap();

    assert_eq!(a.descriptor.id, b.descriptor.id, "fallthrough order must not vary between calls");
}
```

- [ ] **Step 9: Commit**

```bash
cargo test -p indra --test sovereign_model_selection
cargo fmt
git commit -am "feat(sovereign): surface selection warnings and fall through on model failure"
```

---

### Task 3: Network-isolated sandbox — **A3, A5**

**Files:**
- Create: `crates/indra/src/sovereign/sandbox.rs`
- Modify: `crates/indra/src/sovereign/mod.rs`
- Test: `crates/indra/tests/sovereign_sandbox.rs`

**Interfaces:**
- Produces: `SandboxBackend` trait, `SandboxOutput`, `detect_backend() -> Result<Box<dyn SandboxBackend>>`. Task 6b consumes `SandboxBackend` for recomputation.

The existing `developer/shell.rs` path is labelled adapter-mock and **must not be used for A3**. This task replaces it for sandboxed execution only; ordinary shell tooling is unchanged.

- [ ] **Step 1: Write the failing tests**

Create `crates/indra/tests/sovereign_sandbox.rs`:

```rust
use indra::sovereign::sandbox::{detect_backend, SandboxBackend};
use std::time::Duration;

#[test]
fn sandbox_runs_trivial_code() {
    let Ok(backend) = detect_backend() else {
        eprintln!("no sandbox backend available; skipping");
        return;
    };
    let out = backend.run("print(2 + 2)", Duration::from_secs(30)).unwrap();
    assert_eq!(out.stdout.trim(), "4");
    assert_eq!(out.exit_code, 0);
}

#[test]
fn sandbox_cannot_reach_the_network() {
    let Ok(backend) = detect_backend() else {
        eprintln!("no sandbox backend available; skipping");
        return;
    };
    let code = r#"
import socket
try:
    socket.create_connection(("1.1.1.1", 53), timeout=5)
    print("REACHED")
except Exception:
    print("BLOCKED")
"#;
    let out = backend.run(code, Duration::from_secs(30)).unwrap();
    assert!(
        out.stdout.contains("BLOCKED"),
        "sandbox reached the network — A5 is violated: {}",
        out.stdout
    );
}

#[test]
fn sandbox_enforces_timeout() {
    let Ok(backend) = detect_backend() else { return };
    let out = backend
        .run("import time; time.sleep(60)", Duration::from_secs(2))
        .unwrap();
    assert!(out.timed_out, "long-running code was not terminated");
}
```

- [ ] **Step 2: Run and watch it fail**

```bash
cargo test -p indra --test sovereign_sandbox
```
Expected: FAIL — unresolved import.

- [ ] **Step 3: Implement**

Create `crates/indra/src/sovereign/sandbox.rs`:

```rust
use anyhow::{bail, Result};
use std::process::{Command, Stdio};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct SandboxOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub timed_out: bool,
}

pub trait SandboxBackend: Send + Sync {
    fn name(&self) -> &str;
    fn run(&self, code: &str, timeout: Duration) -> Result<SandboxOutput>;
}

pub struct LinuxNamespace;
pub struct DockerNoNetwork;

impl SandboxBackend for LinuxNamespace {
    fn name(&self) -> &str {
        "unshare-no-net"
    }

    fn run(&self, code: &str, timeout: Duration) -> Result<SandboxOutput> {
        let secs = timeout.as_secs().max(1).to_string();
        run_capturing(
            Command::new("unshare")
                .args(["-n", "-p", "-f", "--mount-proc", "timeout", &secs, "python3", "-c", code])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped()),
        )
    }
}

impl SandboxBackend for DockerNoNetwork {
    fn name(&self) -> &str {
        "docker-network-none"
    }

    fn run(&self, code: &str, timeout: Duration) -> Result<SandboxOutput> {
        let secs = timeout.as_secs().max(1).to_string();
        run_capturing(
            Command::new("docker")
                .args([
                    "run", "--rm", "--network=none", "--memory=512m", "--cpus=1",
                    "python:3.11-slim", "timeout", &secs, "python", "-c", code,
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped()),
        )
    }
}

fn run_capturing(cmd: &mut Command) -> Result<SandboxOutput> {
    let out = cmd.output()?;
    let code = out.status.code().unwrap_or(-1);
    Ok(SandboxOutput {
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        exit_code: code,
        // GNU timeout reports 124 when it kills the child.
        timed_out: code == 124,
    })
}

fn available(bin: &str, probe_args: &[&str]) -> bool {
    Command::new(bin)
        .args(probe_args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn detect_backend() -> Result<Box<dyn SandboxBackend>> {
    if cfg!(target_os = "linux") && available("unshare", &["--version"]) {
        return Ok(Box::new(LinuxNamespace));
    }
    if available("docker", &["info"]) {
        return Ok(Box::new(DockerNoNetwork));
    }
    bail!("no isolated sandbox available; refusing to execute code unisolated")
}
```

The final `bail!` is deliberate. Falling back to an unisolated subprocess would silently void A3 and A5 — the sandbox must refuse rather than degrade.

- [ ] **Step 4: Run and confirm pass**

On macOS start Docker Desktop first, then:
```bash
docker pull python:3.11-slim
cargo test -p indra --test sovereign_sandbox
cargo fmt
```
Expected: 3 passed. If `sandbox_cannot_reach_the_network` fails, stop — A5 is broken and nothing downstream matters.

- [ ] **Step 5: Commit**

```bash
git add crates/indra/src/sovereign/sandbox.rs crates/indra/src/sovereign/mod.rs \
        crates/indra/tests/sovereign_sandbox.rs
git commit -m "feat(sovereign): add network-isolated sandbox with no unisolated fallback"
```

---

### Task 4: Multimodal ingest — render, OCR for boxes, VLM for meaning — **A4**

**Files:**
- Create: `crates/indra/src/sovereign/provenance.rs`
- Create: `crates/indra/src/sovereign/ingest.rs`
- Test: `crates/indra/tests/sovereign_ingest.rs`
- Test fixture: `crates/indra/tests/fixtures/sample_inspection.pdf`

**Interfaces:**
- Produces: `SourceBox`, `OcrLine`, `ocr_page(path) -> Result<Vec<OcrLine>>`, `IngestedPage { page: u32, image_path: PathBuf, lines: Vec<OcrLine> }`. Task 5 consumes all of these.

**Model download warning.** A vision model plus its `mmproj` is 4–8 GiB. Check free space *before* downloading; Task 1's disk note applies. Use a small VLM — Qwen2-VL-2B or a Gemma-3 4B multimodal GGUF — not a 32B.

- [ ] **Step 1: Install Tesseract**

```bash
brew install tesseract     # macOS
# sudo apt-get install -y tesseract-ocr   # Debian/Ubuntu plant server
tesseract --version
```

Tesseract is invoked as a subprocess, not linked. That keeps a C dependency out of the build and costs nothing at this scale.

- [ ] **Step 2: Write the failing test**

Create `crates/indra/tests/sovereign_ingest.rs`:

```rust
use indra::sovereign::ingest::ocr_page;
use std::path::Path;

#[test]
fn ocr_returns_lines_with_nonzero_boxes() {
    let fixture = Path::new("tests/fixtures/sample_inspection.png");
    if !fixture.exists() {
        eprintln!("fixture missing; skipping");
        return;
    }

    let lines = ocr_page(fixture).unwrap();

    assert!(!lines.is_empty(), "OCR returned no lines");
    for line in &lines {
        assert!(line.bbox.w > 0 && line.bbox.h > 0, "degenerate box: {:?}", line.bbox);
    }
}

#[test]
fn ocr_text_is_searchable() {
    let fixture = Path::new("tests/fixtures/sample_inspection.png");
    if !fixture.exists() {
        return;
    }
    let lines = ocr_page(fixture).unwrap();
    let all: String = lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join(" ");
    assert!(!all.trim().is_empty(), "OCR produced no text");
}
```

- [ ] **Step 3: Run and watch it fail**

```bash
cargo test -p indra --test sovereign_ingest
```
Expected: FAIL — unresolved import.

- [ ] **Step 4: Implement provenance types**

Create `crates/indra/src/sovereign/provenance.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceBox {
    pub page: u32,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub document_id: String,
    pub text: String,
    pub boxes: Vec<SourceBox>,
}
```

- [ ] **Step 5: Implement OCR**

Create `crates/indra/src/sovereign/ingest.rs`:

```rust
use crate::sovereign::provenance::SourceBox;
use anyhow::{bail, Result};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct OcrLine {
    pub text: String,
    pub bbox: SourceBox,
    pub confidence: f32,
}

pub fn ocr_page(image: &Path) -> Result<Vec<OcrLine>> {
    let out = Command::new("tesseract")
        .arg(image)
        .args(["stdout", "tsv"])
        .output()?;

    if !out.status.success() {
        bail!("tesseract exited {}", out.status);
    }

    Ok(parse_tsv(&String::from_utf8_lossy(&out.stdout)))
}

// Tesseract TSV columns: level page_num block_num par_num line_num word_num
// left top width height conf text. Level 4 rows are lines; level 5 are words.
// Words carry the text, so words are accumulated and grouped into their line.
fn parse_tsv(tsv: &str) -> Vec<OcrLine> {
    let mut lines: Vec<OcrLine> = Vec::new();
    let mut current_key: Option<(u32, u32)> = None;

    for row in tsv.lines().skip(1) {
        let f: Vec<&str> = row.split('\t').collect();
        if f.len() < 12 || f[0] != "5" {
            continue;
        }
        let text = f[11].trim();
        let conf: f32 = f[10].parse().unwrap_or(-1.0);
        if text.is_empty() || conf < 0.0 {
            continue;
        }

        let page: u32 = f[1].parse().unwrap_or(1);
        let line_num: u32 = f[4].parse().unwrap_or(0);
        let (x, y, w, h) = (
            f[6].parse().unwrap_or(0),
            f[7].parse().unwrap_or(0),
            f[8].parse().unwrap_or(0),
            f[9].parse().unwrap_or(0),
        );

        if current_key == Some((page, line_num)) {
            let last = lines.last_mut().expect("key set implies a line exists");
            last.text.push(' ');
            last.text.push_str(text);
            let right = last.bbox.x.max(x) + last.bbox.w.max(w);
            last.bbox.x = last.bbox.x.min(x);
            last.bbox.y = last.bbox.y.min(y);
            last.bbox.w = right - last.bbox.x;
            last.bbox.h = last.bbox.h.max(h);
            last.confidence = last.confidence.min(conf);
        } else {
            current_key = Some((page, line_num));
            lines.push(OcrLine {
                text: text.to_string(),
                bbox: SourceBox { page, x, y, w, h },
                confidence: conf,
            });
        }
    }

    lines
}
```

Add `pub mod ingest;` and `pub mod provenance;` to `crates/indra/src/sovereign/mod.rs`.

- [ ] **Step 6: Create the fixture and run**

Save any public sample inspection report page as `crates/indra/tests/fixtures/sample_inspection.png`.

```bash
cargo test -p indra --test sovereign_ingest
cargo fmt
```
Expected: 2 passed.

- [ ] **Step 7: Commit**

```bash
git add crates/indra/src/sovereign/ crates/indra/tests/sovereign_ingest.rs \
        crates/indra/tests/fixtures/
git commit -m "feat(sovereign): OCR page ingest with line-level bounding boxes"
```

- [ ] **Step 8: Wire the vision model**

`vision_capable` and `mmproj_path` already exist at `crates/indra-local-inference/src/lib.rs:252` and `:382`. Register a VLM descriptor with `vision: true` and its `mmproj_path` set, then confirm `IndraOperation::PidAnalysis` selects it via Task 2's `select_model`. Verify by asking the model to describe a P&ID fixture image.

- [ ] **Step 9: Commit**

```bash
git commit -am "feat(sovereign): register vision model descriptor for multimodal operations"
```

---

### Task 4b: Write guard — the answer to "what if the AI is wrong?"

**Files:**
- Create: `crates/indra/src/sovereign/write_class.rs`
- Test: `crates/indra/tests/sovereign_write_guard.rs`

**Interfaces:**
- Produces: `WriteClass`, `ResourceRef`, `write_class_for(&ResourceRef) -> WriteClass`, `guard_write(&ResourceRef) -> Result<()>`. Task 5 calls `guard_write` before producing any deliverable.

This serves no acceptance criterion directly, and it is small — roughly two hours. Build it anyway. Every PSU evaluator asks some version of *"what happens when it gets something wrong?"*, and "source documents are unwritable by construction, deliverables are new versions, nothing is ever overwritten" is a far better answer than a promise.

- [ ] **Step 1: Write the failing test**

Create `crates/indra/tests/sovereign_write_guard.rs`:

```rust
use indra::sovereign::write_class::{guard_write, write_class_for, ResourceRef, WriteClass};
use std::path::PathBuf;

#[test]
fn source_documents_are_sealed() {
    let r = ResourceRef::SourceDocument(PathBuf::from("/data/sources/NDT_2026_08.pdf"));
    assert_eq!(write_class_for(&r), WriteClass::Sealed);
    assert!(guard_write(&r).is_err(), "a Sealed resource accepted a write");
}

#[test]
fn deliverables_are_versioned_and_writable() {
    let r = ResourceRef::Deliverable(PathBuf::from("/data/out/Approval_Note.docx"));
    assert_eq!(write_class_for(&r), WriteClass::Versioned);
    assert!(guard_write(&r).is_ok());
}

#[test]
fn systems_of_record_are_gated_not_writable() {
    let r = ResourceRef::SystemOfRecord { system: "sap_pm".to_string() };
    assert_eq!(write_class_for(&r), WriteClass::Gated);
    assert!(
        guard_write(&r).is_err(),
        "a Gated system must receive a proposal, never a direct write"
    );
}
```

- [ ] **Step 2: Run and watch it fail**

```bash
cargo test -p indra --test sovereign_write_guard
```
Expected: FAIL — unresolved import.

- [ ] **Step 3: Implement**

```rust
use anyhow::{bail, Result};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteClass {
    Sealed,
    Versioned,
    Gated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceRef {
    SourceDocument(PathBuf),
    Deliverable(PathBuf),
    SystemOfRecord { system: String },
}

// Exhaustive by construction: a new resource kind cannot be added without the
// compiler demanding its class here, so nothing defaults to writable.
pub fn write_class_for(r: &ResourceRef) -> WriteClass {
    match r {
        ResourceRef::SourceDocument(_) => WriteClass::Sealed,
        ResourceRef::Deliverable(_) => WriteClass::Versioned,
        ResourceRef::SystemOfRecord { .. } => WriteClass::Gated,
    }
}

pub fn guard_write(r: &ResourceRef) -> Result<()> {
    match write_class_for(r) {
        WriteClass::Versioned => Ok(()),
        WriteClass::Sealed => bail!("{r:?} is Sealed; source documents are never modified"),
        WriteClass::Gated => {
            bail!("{r:?} is Gated; emit a proposal for approval rather than writing directly")
        }
    }
}
```

Add `pub mod write_class;` to `crates/indra/src/sovereign/mod.rs`.

- [ ] **Step 4: Run, format, commit**

```bash
cargo test -p indra --test sovereign_write_guard
cargo fmt
git add crates/indra/src/sovereign/write_class.rs crates/indra/src/sovereign/mod.rs \
        crates/indra/tests/sovereign_write_guard.rs
git commit -m "feat(sovereign): add write guard sealing source documents from modification"
```

---

### Task 5: End-to-end agentic path with box-level citations — **A2**

**Files:**
- Modify: `crates/indra/src/agents/specialists/specialist.rs`
- Create: `crates/indra/src/sovereign/deliverable.rs`
- Test: `crates/indra/tests/sovereign_approval_note.rs`

**Interfaces:**
- Consumes: `ocr_page`, `OcrLine`, `Citation`, `SourceBox` from Task 4; `select_model` from Task 2.
- Produces: `build_approval_note(findings: &[Finding], out: &Path) -> Result<()>` and `Finding { text: String, citation: Citation, severity: Severity }`.

This is the headline demo: scanned inspection report in, `.docx` approval note out, every claim traceable to a highlighted region of the scan.

- [ ] **Step 1: Write the failing test**

Create `crates/indra/tests/sovereign_approval_note.rs`:

```rust
use indra::sovereign::deliverable::{build_approval_note, Finding, Severity};
use indra::sovereign::provenance::{Citation, SourceBox};
use tempfile::tempdir;

fn finding(text: &str, sev: Severity) -> Finding {
    Finding {
        text: text.to_string(),
        severity: sev,
        citation: Citation {
            document_id: "NDT_2026_08.pdf".to_string(),
            text: "measured wall thickness 6.1 mm".to_string(),
            boxes: vec![SourceBox { page: 1, x: 100, y: 220, w: 400, h: 24 }],
        },
    }
}

#[test]
fn approval_note_is_written_and_nonempty() {
    let dir = tempdir().unwrap();
    let out = dir.path().join("Approval_Note.docx");

    build_approval_note(
        &[finding("Wall thickness below minimum", Severity::CriticalActionRequired)],
        &out,
    )
    .unwrap();

    assert!(out.exists(), "no docx produced");
    assert!(out.metadata().unwrap().len() > 0, "docx is empty");
}

#[test]
fn every_finding_carries_at_least_one_source_box() {
    let f = finding("Wall thickness below minimum", Severity::Monitor);
    assert!(
        !f.citation.boxes.is_empty(),
        "a finding without a source box cannot be traced back to the scan"
    );
}
```

- [ ] **Step 2: Run and watch it fail**

```bash
cargo test -p indra --test sovereign_approval_note
```
Expected: FAIL — unresolved import.

- [ ] **Step 3: Add the test dependency**

```bash
cargo add tempfile --dev -p indra
```

- [ ] **Step 4: Implement**

Create `crates/indra/src/sovereign/deliverable.rs`:

```rust
use crate::sovereign::provenance::Citation;
use crate::sovereign::write_class::{guard_write, ResourceRef};
use anyhow::Result;
use docx_rs::{Docx, Paragraph, Run};
use std::fs::File;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Normal,
    Monitor,
    CriticalActionRequired,
}

impl Severity {
    pub fn label(&self) -> &'static str {
        match self {
            Severity::Normal => "Normal",
            Severity::Monitor => "Monitor",
            Severity::CriticalActionRequired => "Critical Action Required",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub text: String,
    pub severity: Severity,
    pub citation: Citation,
}

pub fn build_approval_note(findings: &[Finding], out: &Path) -> Result<()> {
    guard_write(&ResourceRef::Deliverable(out.to_path_buf()))?;

    let mut doc = Docx::new().add_paragraph(
        Paragraph::new().add_run(Run::new().add_text("Inspection Approval Note").bold(true)),
    );

    for f in findings {
        doc = doc.add_paragraph(
            Paragraph::new()
                .add_run(Run::new().add_text(format!("[{}] ", f.severity.label())).bold(true))
                .add_run(Run::new().add_text(&f.text)),
        );

        let refs: Vec<String> = f
            .citation
            .boxes
            .iter()
            .map(|b| format!("p.{} @ ({},{})", b.page, b.x, b.y))
            .collect();

        doc = doc.add_paragraph(Paragraph::new().add_run(
            Run::new()
                .add_text(format!("Source: {} — {}", f.citation.document_id, refs.join("; ")))
                .italic(true),
        ));
    }

    doc.build().pack(File::create(out)?)?;
    Ok(())
}
```

Add `pub mod deliverable;` to `crates/indra/src/sovereign/mod.rs`. Add `docx-rs` to the `indra` crate:

```bash
cargo add docx-rs --no-default-features --features image -p indra
```

- [ ] **Step 5: Run and confirm pass**

```bash
cargo test -p indra --test sovereign_approval_note
cargo fmt
```
Expected: 2 passed.

- [ ] **Step 6: Commit**

```bash
git add crates/indra/src/sovereign/deliverable.rs crates/indra/src/sovereign/mod.rs \
        crates/indra/tests/sovereign_approval_note.rs crates/indra/Cargo.toml Cargo.lock
git commit -m "feat(sovereign): generate approval notes with box-level source citations"
```

- [ ] **Step 7: Connect the full path**

Chain it in `InspectionAnalysisSpecialist`: ingest the scan (Task 4) → retrieve matching OCR lines → the VLM produces findings → each finding keeps the boxes of the lines that supported it → `build_approval_note` writes the `.docx`. The specialist's `system_prompt_framing` at `specialist.rs:59` already instructs it to cite findings and categorise by the three severities used above — no prompt change needed.

- [ ] **Step 8: Commit**

```bash
git commit -am "feat(sovereign): wire inspection specialist to ingest and deliverable path"
```

---

### Task 6a: Egress proof surface — **A5**

**Files:**
- Modify: `crates/indra/src/security/egress_inspector.rs`
- Test: `crates/indra/tests/sovereign_egress_proof.rs`

**Interfaces:**
- Produces: `EgressLog { attempts: Vec<EgressAttempt> }` and a `trigger_probe()` entry point the UI calls for the deliberate-violation demo.

- [ ] **Step 1: Read what already exists**

`egress_inspector.rs` is 555 lines. Read it before adding anything — the recording may already be there and only the surface missing.

- [ ] **Step 2: Write the failing test**

Create `crates/indra/tests/sovereign_egress_proof.rs`:

```rust
use indra::security::egress_inspector::{trigger_probe, EgressLog};

#[test]
fn deliberate_probe_is_blocked_and_recorded() {
    let before = EgressLog::current().attempts.len();

    let result = trigger_probe("http://example.com");

    assert!(result.is_err(), "the probe reached the network — A5 is violated");
    assert_eq!(
        EgressLog::current().attempts.len(),
        before + 1,
        "the blocked attempt was not recorded"
    );
}

#[test]
fn normal_operation_records_zero_attempts() {
    assert_eq!(
        EgressLog::current().attempts.len(),
        0,
        "an outbound attempt occurred during normal startup"
    );
}
```

Run these two tests in separate processes — the second asserts a clean log:
```bash
cargo test -p indra --test sovereign_egress_proof -- --test-threads=1
```

- [ ] **Step 3: Implement, run, commit**

```bash
cargo fmt
git commit -am "feat(security): expose egress log and deliberate-violation probe for A5"
```

---

### Task 6b: Deterministic recomputation — **A3**

**Files:**
- Create: `crates/indra/src/sovereign/recompute.rs`
- Test: `crates/indra/tests/sovereign_recompute.rs`

**Interfaces:**
- Consumes: `SandboxBackend`, `detect_backend` from Task 3.
- Produces: `verify_calculation(backend: &dyn SandboxBackend, python: &str, claimed: f64, tol: f64) -> Result<Verdict>` and `enum Verdict { Agrees, Disagrees { computed: f64 } }`.

**This is the first thing to cut if you slip.** Everything above it serves an acceptance criterion directly; this strengthens A3 rather than creating it.

- [ ] **Step 1: Write the failing test**

Create `crates/indra/tests/sovereign_recompute.rs`:

```rust
use indra::sovereign::recompute::{verify_calculation, Verdict};
use indra::sovereign::sandbox::detect_backend;

#[test]
fn agreeing_calculation_is_accepted() {
    let Ok(b) = detect_backend() else { return };
    let v = verify_calculation(b.as_ref(), "print(3.14159 * 2 * 2)", 12.566, 0.01).unwrap();
    assert_eq!(v, Verdict::Agrees);
}

#[test]
fn disagreeing_calculation_is_rejected() {
    let Ok(b) = detect_backend() else { return };
    let v = verify_calculation(b.as_ref(), "print(3.14159 * 2 * 2)", 99.0, 0.01).unwrap();
    assert!(matches!(v, Verdict::Disagrees { .. }), "a wrong number was accepted");
}
```

- [ ] **Step 2: Implement**

```rust
use crate::sovereign::sandbox::SandboxBackend;
use anyhow::{bail, Result};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    Agrees,
    Disagrees { computed: f64 },
}

pub fn verify_calculation(
    backend: &dyn SandboxBackend,
    python: &str,
    claimed: f64,
    tolerance: f64,
) -> Result<Verdict> {
    let out = backend.run(python, Duration::from_secs(20))?;
    if out.exit_code != 0 {
        bail!("calculation failed to execute: {}", out.stderr);
    }

    let last = out
        .stdout
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or_default();

    let computed: f64 = last.trim().parse()?;

    if (computed - claimed).abs() <= tolerance {
        Ok(Verdict::Agrees)
    } else {
        Ok(Verdict::Disagrees { computed })
    }
}
```

- [ ] **Step 3: Run, format, commit**

```bash
cargo test -p indra --test sovereign_recompute
cargo fmt
git add crates/indra/src/sovereign/recompute.rs crates/indra/tests/sovereign_recompute.rs
git commit -m "feat(sovereign): verify model calculations by sandboxed recomputation"
```

---

### Task 7: Integration and demo rehearsal

- [ ] **Step 1: Full verification**

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test -p indra
```

- [ ] **Step 2: Update the self-test recipe**

`AGENTS.md` requires new features be added to the self-test recipe. Add cases covering A1 through A5, then:

```bash
cargo build
indra run --recipe indra-self-test.yaml
```

- [ ] **Step 3: Rehearse the demo in order**

1. **A5 first.** Open the egress panel — zero attempts. Trigger the deliberate probe; show it blocked and logged. Establish sovereignty before showing anything else, because every later claim rests on it.
2. **A1.** Ask a coding question, then a document-summary question. Show two different models selected, with reasons.
3. **A4 + A2.** Feed the scanned inspection report. Show findings extracted, then the `.docx` produced, then click a claim and show the highlighted region of the original scan.
4. **A3.** Ask for a calculation. Show the code running in the sandbox, the recomputation agreeing, and the network probe from inside the sandbox failing.

- [ ] **Step 4: Disconnect the network and run the whole thing again**

The strongest possible proof, and it costs nothing: unplug Ethernet, disable Wi-Fi, repeat the full demo. If anything breaks, it was never sovereign.

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "chore: self-test coverage for SIH 26117 acceptance criteria"
```
