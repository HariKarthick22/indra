# INDRA Logical Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish the backend logic the UI design depends on — a real event contract, a completed sovereign pipeline (approval notes + recomputation + sandboxed tool execution), a real embedding-backed memory store, and a byte-budgeted session ledger — then strip the token/cost UI that contradicts the new design, before any further screen work starts.

**Architecture:** This plan does not restart anything. `crates/indra/src/sovereign/` (hardware, model registry, sandbox, OCR ingest, write guard, egress log) already exists as uncommitted work and Tasks 1–8 below build on it rather than duplicating it. The single biggest gap connecting that backend to the UI design is that no structured event stream exists — today the agent loop only stuffs ad-hoc key/value pairs into message metadata via `set_operation_note`. Task 1 defines `IndraEvent` as a first-class Rust type and threads it through that same metadata channel (ACP's `SessionUpdate`/`SessionNotification` types are vendored from the `agent-client-protocol` crate and cannot be extended directly), so every later task has one real channel to emit on instead of inventing its own.

**Tech Stack:** Rust (`crates/indra`, `crates/indra-mcp`, `crates/indra-local-inference`), llama.cpp via `llama-cpp-2` (embedding mode), Tesseract subprocess, `docx-rs`, CBOR (`ciborium`) + `zstd`, TypeScript/React (`ui/desktop`, Electron — not yet Tauri; that migration is out of scope here, see Non-Goals).

**Spec:** [`INDRA-system-and-ui-design.md`](../../../INDRA-system-and-ui-design.md) §2.2 (event contract), §2.3 (session context model), §5.5 (memory), §6.3 (context ledger). Companion backend plan: [`2026-09-13-sih-26117-sovereign-workbench.md`](2026-09-13-sih-26117-sovereign-workbench.md) Tasks 5, 6b (this plan finishes what that one left open).

## Global Constraints

- **No outbound network calls at any point.** Acceptance criterion A5. Any new dependency that phones home is disqualified. `cargo add` only for dependencies already vetted as offline-safe (sysinfo, ciborium, zstd, docx-rs are already in use or in the companion plan).
- **Agent-loop parity.** Per `AGENTS.md`, changes to agent-loop behavior land in **both** `crates/indra/src/agents/agent.rs` and `crates/indra/src/agents/state_machine/`. Tasks 1, 4, and 8 are agent-loop behavior; the parity rule applies to each.
- **Tests live in `crates/indra/tests/`**, not inline `#[cfg(test)] mod tests` blocks, per `AGENTS.md` — except where a task modifies an existing crate (`indra-mcp`) that already uses inline test modules for that file; follow the file's existing convention rather than introduce a second one.
- **Dependency changes use `cargo add`**, never manual `Cargo.toml` edits.
- **Run `cargo fmt` after every change.** Never skip it.
- **Comments:** only for non-obvious "why". Never restate what the code does.
- **Errors:** `anyhow::Result`. No `.context("failed to X")` when the error already says it failed.
- **Sealed is hard-wired.** No configuration value may make a Sealed resource writable (already enforced by `sovereign::write_class`; nothing in this plan weakens it).
- **Hardware shortfall warns; it never excludes and never fails silently** (already the behavior of `sovereign::model_registry::select_model`; new code that selects models must reuse it, not reimplement fit logic).
- **No decorative abstraction.** The memory store (Task 6/7) uses brute-force cosine similarity, not an ANN library — at the tens-of-thousands-of-chunks scale a single plant's corpus produces, a linear scan over `f32` vectors is faster to build, easier to audit for sovereignty (no vendored C++ ANN dependency to check for phone-home behavior), and fast enough. Do not add `hnsw`, `faiss`, or similar.

## Non-Goals (explicitly out of scope for this plan)

- **Tauri migration.** The design doc calls for it (§2.1, decision #6); the desktop app is Electron today (confirmed: `ui/desktop/package.json` lists `electron@32.0.0`, no `tauri.conf.json` anywhere in the tree). Migrating the shell is a separate, large, UI-only plan — doing it here would bury the logic work this plan exists to finish.
- **The Memory screen's constellation/ledger/timeline UI** (§5.5). Task 7 makes the *data* real (embeddings, semantic search); rendering it as a force-directed graph is a UI plan that should be written once Task 7 lands, so its `NodeState`/`EdgeStyle` fields are drawn from a real API instead of guessed.
- **PDF → image rendering.** `sovereign::ingest::ocr_page` takes an already-rendered image; nothing in the tree renders a PDF page to an image (`pdf_tool.rs` only extracts raw text via `lopdf`). Task 5 extends the citation *type*, not the rendering pipeline — that is its own task once a PDF-rasterization dependency is chosen (e.g. `pdfium-render`), and it is not blocking anything else in this plan.

---

### Task 1: The `IndraEvent` type and its transport

**Files:**
- Create: `crates/indra/src/events/mod.rs`
- Create: `crates/indra/src/events/types.rs`
- Modify: `crates/indra/src/lib.rs` (register `pub mod events;`)
- Modify: `crates/indra/src/agents/agent.rs:2794-2810` (emit via the new helper instead of raw `set_operation_note` calls)
- Modify: `crates/indra/src/agents/state_machine/ops_llm.rs:197-212` (same, state-machine side — parity)
- Test: `crates/indra/tests/events_contract.rs`

**Interfaces:**
- Consumes: `indra_provider_types::conversation::message::MessageMetadata::set_operation_note(&mut self, operation: &str, key: &str, value: serde_json::Value)` — already exists at `crates/indra-provider-types/src/conversation/message.rs:876`, unchanged by this task.
- Produces: `events::IndraEvent` (a `#[serde(tag = "t")]` enum matching the design doc's §2.2 union), `events::emit(metadata: &mut MessageMetadata, event: IndraEvent)`. Tasks 2, 3, 4, 5, 6, 8 all call `events::emit` instead of touching `set_operation_note` directly — that is what makes the event stream a real seam instead of a naming convention.

The `IndraEvent` enum is data-only in this task — no plan-tracking state machine, no event bus. `emit` serializes the event to JSON once and writes it under a single stable key (`"indra_event"`) via the existing `set_operation_note("indra", "indra_event", ...)` channel, which already reaches the ACP `SessionNotification` the desktop UI receives (confirmed: `crates/indra/src/acp/provider.rs:1215` forwards `SessionNotification` through `notification_callback`, and operation notes are already read by the UI's specialist badge — this task generalizes that existing, working path rather than inventing a new one).

- [ ] **Step 1: Write the failing test**

Create `crates/indra/tests/events_contract.rs`:

```rust
use indra::events::{emit, IndraEvent};
use indra_provider_types::conversation::message::MessageMetadata;
use serde_json::json;

#[test]
fn model_selected_event_roundtrips_through_metadata() {
    let mut meta = MessageMetadata::default();
    emit(
        &mut meta,
        IndraEvent::ModelSelected {
            model_id: "qwen-coder".to_string(),
            reason: "Code Generation requires domain Code".to_string(),
            fit: json!({"kind": "comfortable"}),
        },
    );

    let stored = meta
        .get_operation_note("indra", "indra_event")
        .expect("event was not recorded");
    assert_eq!(stored["t"], "model.selected");
    assert_eq!(stored["model_id"], "qwen-coder");
}

#[test]
fn egress_attempt_event_carries_blocked_true() {
    let mut meta = MessageMetadata::default();
    emit(
        &mut meta,
        IndraEvent::EgressAttempt {
            url: "https://example.com".to_string(),
            blocked: true,
            at: "2026-09-13T14:02:31Z".to_string(),
        },
    );

    let stored = meta.get_operation_note("indra", "indra_event").unwrap();
    assert_eq!(stored["t"], "egress.attempt");
    assert_eq!(stored["blocked"], true);
}
```

- [ ] **Step 2: Confirm `get_operation_note` exists, or add it**

```bash
grep -n "fn get_operation_note" crates/indra-provider-types/src/conversation/message.rs
```

If absent (the codebase currently only writes notes, per the agent-loop diffs — no reader was found), add it next to `set_operation_note` in `crates/indra-provider-types/src/conversation/message.rs`:

```rust
pub fn get_operation_note(&self, operation: &str, key: &str) -> Option<&serde_json::Value> {
    self.operation_notes.get(operation)?.get(key)
}
```

Match the exact field name `set_operation_note` already writes to (read the surrounding lines at `:876` before adding — do not guess the storage field's name).

- [ ] **Step 3: Run and watch it fail**

```bash
cargo test -p indra --test events_contract
```
Expected: FAIL — `unresolved import indra::events`.

- [ ] **Step 4: Implement the event type**

Create `crates/indra/src/events/types.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceBox {
    pub doc_id: String,
    pub version: String,
    pub page: u32,
    pub bbox: [u32; 4],
    pub conf: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t")]
pub enum IndraEvent {
    #[serde(rename = "plan.proposed")]
    PlanProposed { steps: Vec<PlanStep> },
    #[serde(rename = "plan.revised")]
    PlanRevised {
        after_step: String,
        steps: Vec<PlanStep>,
        reason: String,
    },
    #[serde(rename = "step.state")]
    StepState {
        step_id: String,
        state: String,
        ms: Option<u64>,
    },
    #[serde(rename = "model.selected")]
    ModelSelected {
        model_id: String,
        reason: String,
        fit: serde_json::Value,
    },
    #[serde(rename = "tool.call")]
    ToolCall {
        call_id: String,
        step_id: String,
        name: String,
        args: serde_json::Value,
        sandbox_backend: String,
    },
    #[serde(rename = "tool.result")]
    ToolResult {
        call_id: String,
        ok: bool,
        ms: u64,
        summary: String,
        bytes: usize,
        citations: Vec<SourceBox>,
    },
    #[serde(rename = "citation")]
    Citation { span: [usize; 2], source: SourceBox },
    #[serde(rename = "context.delta")]
    ContextDelta {
        session_bytes: usize,
        budget_bytes: usize,
    },
    #[serde(rename = "context.compacted")]
    ContextCompacted {
        from_turns: usize,
        to_bytes: usize,
        dropped_count: usize,
    },
    #[serde(rename = "memory.touch")]
    MemoryTouch { node_ids: Vec<String>, op: String },
    #[serde(rename = "egress.attempt")]
    EgressAttempt {
        url: String,
        blocked: bool,
        at: String,
    },
    #[serde(rename = "verify.recompute")]
    VerifyRecompute {
        claim: f64,
        computed: f64,
        verdict: String,
    },
    #[serde(rename = "guard.blocked")]
    GuardBlocked { resource: String, reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: String,
    pub label: String,
    pub tool: Option<String>,
}
```

`SourceBox` here is the design doc's `BoxRef` — it is intentionally a *different* type from `sovereign::provenance::SourceBox` (which lacks `doc_id`/`version`/`conf`, see Task 5) rather than a shared one: the event contract is the UI-facing shape, `sovereign::provenance` is the ingest-pipeline's internal shape, and Task 5 writes the conversion between them explicitly rather than making one type serve both roles under different meanings.

Create `crates/indra/src/events/mod.rs`:

```rust
mod types;
pub use types::{IndraEvent, PlanStep, SourceBox};

use indra_provider_types::conversation::message::MessageMetadata;

pub fn emit(metadata: &mut MessageMetadata, event: IndraEvent) {
    let value = serde_json::to_value(&event).expect("IndraEvent always serializes");
    metadata.set_operation_note("indra", "indra_event", value);
}
```

Add `pub mod events;` to `crates/indra/src/lib.rs`.

- [ ] **Step 5: Run and confirm pass**

```bash
cargo test -p indra --test events_contract
cargo fmt
```
Expected: 2 passed.

- [ ] **Step 6: Commit**

```bash
git add crates/indra/src/events/ crates/indra/src/lib.rs crates/indra/tests/events_contract.rs \
        crates/indra-provider-types/src/conversation/message.rs
git commit -m "feat(events): add IndraEvent type and its transport over existing operation notes"
```

- [ ] **Step 7: Replace the ad-hoc model-selection notes with `events::emit`**

In `crates/indra/src/agents/agent.rs`, the block at (currently) line ~2794 that does:

```rust
response.metadata.set_operation_note(
    "specialist",
    "model_selection_reason",
    serde_json::json!(reason),
);
if let Some(warning) = warning {
    response.metadata.set_operation_note(
        "specialist",
        "model_selection_warning",
        serde_json::json!(warning),
    );
}
```

becomes:

```rust
use crate::events::{emit, IndraEvent};

emit(
    &mut response.metadata,
    IndraEvent::ModelSelected {
        model_id: model_config.model_name.clone(),
        reason: reason.clone(),
        fit: warning
            .as_ref()
            .map(|w| serde_json::json!({"kind": "degraded", "warning": w}))
            .unwrap_or_else(|| serde_json::json!({"kind": "comfortable"})),
    },
);
```

Keep the existing `"specialist"`/`"operation"` notes as they are — this task only replaces the model-selection pair, since that is the one with a matching `IndraEvent` variant today. Do not delete notes that have no `IndraEvent` equivalent yet.

- [ ] **Step 8: Mirror the change in the state machine path (parity)**

Apply the identical replacement in `crates/indra/src/agents/state_machine/ops_llm.rs` at the matching block (currently line ~197-212), using `message.metadata` in place of `response.metadata`.

- [ ] **Step 9: Run the full existing specialist test suite to confirm no regression**

```bash
cargo test -p indra agents::specialists
cargo test -p indra --test events_contract
cargo fmt
```
Expected: all pass — this step only changes which function writes the same information, not what information exists.

- [ ] **Step 10: Commit**

```bash
git commit -am "refactor(events): route model-selection trace through IndraEvent in both agent-loop paths"
```

---

### Task 2: Approval-note deliverable generation (finishes backend plan Task 5)

**Files:**
- Create: `crates/indra/src/sovereign/deliverable.rs`
- Modify: `crates/indra/src/sovereign/mod.rs`
- Test: `crates/indra/tests/sovereign_approval_note.rs`

**Interfaces:**
- Consumes: `sovereign::provenance::Citation`, `sovereign::write_class::{guard_write, ResourceRef}` (both already exist).
- Produces: `sovereign::deliverable::{build_approval_note, Finding, Severity}`. Task 5 (citation type unification) and any future specialist wiring consume this.

This is named and specified in the companion backend plan (`2026-09-13-sih-26117-sovereign-workbench.md`, Task 5) — the code below is that task's Step 4 implementation, carried here because it was never created in the uncommitted work (verified: `crates/indra/src/sovereign/deliverable.rs` does not exist).

- [ ] **Step 1: Add the dependency**

```bash
cargo add docx-rs --no-default-features --features image -p indra
cargo add tempfile --dev -p indra
```

- [ ] **Step 2: Write the failing test**

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

#[test]
fn sealed_source_documents_cannot_be_targeted_as_output() {
    use indra::sovereign::write_class::{guard_write, ResourceRef};
    use std::path::PathBuf;

    let r = ResourceRef::SourceDocument(PathBuf::from("NDT_2026_08.pdf"));
    assert!(
        guard_write(&r).is_err(),
        "build_approval_note must never be pointed at a source document path"
    );
}
```

- [ ] **Step 3: Run and watch it fail**

```bash
cargo test -p indra --test sovereign_approval_note
```
Expected: FAIL — unresolved import `indra::sovereign::deliverable`.

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

Add `pub mod deliverable;` to `crates/indra/src/sovereign/mod.rs`.

- [ ] **Step 5: Run and confirm pass**

```bash
cargo test -p indra --test sovereign_approval_note
cargo fmt
```
Expected: 3 passed.

- [ ] **Step 6: Commit**

```bash
git add crates/indra/src/sovereign/deliverable.rs crates/indra/src/sovereign/mod.rs \
        crates/indra/tests/sovereign_approval_note.rs crates/indra/Cargo.toml Cargo.lock
git commit -m "feat(sovereign): generate approval notes with box-level source citations"
```

---

### Task 3: Sandboxed recomputation (finishes backend plan Task 6b)

**Files:**
- Create: `crates/indra/src/sovereign/recompute.rs`
- Modify: `crates/indra/src/sovereign/mod.rs`
- Modify: `crates/indra/src/events/types.rs` (already has `VerifyRecompute` from Task 1 — this task is what actually produces one)
- Test: `crates/indra/tests/sovereign_recompute.rs`

**Interfaces:**
- Consumes: `sovereign::sandbox::{SandboxBackend, detect_backend}` (already exists).
- Produces: `sovereign::recompute::{verify_calculation, Verdict}`. Any code path that lets the model claim a derived number (per the design doc's system prompt §2.6, "NUMBERS" section) calls this before presenting the number.

- [ ] **Step 1: Write the failing test**

Create `crates/indra/tests/sovereign_recompute.rs`:

```rust
use indra::sovereign::recompute::{verify_calculation, Verdict};
use indra::sovereign::sandbox::detect_backend;

#[test]
fn agreeing_calculation_is_accepted() {
    let Ok(b) = detect_backend() else {
        eprintln!("no sandbox backend available; skipping");
        return;
    };
    let v = verify_calculation(b.as_ref(), "print(3.14159 * 2 * 2)", 12.566, 0.01).unwrap();
    assert_eq!(v, Verdict::Agrees);
}

#[test]
fn disagreeing_calculation_is_rejected() {
    let Ok(b) = detect_backend() else {
        return;
    };
    let v = verify_calculation(b.as_ref(), "print(3.14159 * 2 * 2)", 99.0, 0.01).unwrap();
    assert!(matches!(v, Verdict::Disagrees { .. }), "a wrong number was accepted");
}

#[test]
fn nonnumeric_output_is_an_error_not_a_false_agreement() {
    let Ok(b) = detect_backend() else {
        return;
    };
    let result = verify_calculation(b.as_ref(), "print('not a number')", 1.0, 0.01);
    assert!(result.is_err(), "unparseable sandbox output must not silently agree");
}
```

- [ ] **Step 2: Run and watch it fail**

```bash
cargo test -p indra --test sovereign_recompute
```
Expected: FAIL — unresolved import.

- [ ] **Step 3: Implement**

Create `crates/indra/src/sovereign/recompute.rs`:

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

    let computed: f64 = last
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("sandbox output was not a number: {last:?}"))?;

    if (computed - claimed).abs() <= tolerance {
        Ok(Verdict::Agrees)
    } else {
        Ok(Verdict::Disagrees { computed })
    }
}
```

Add `pub mod recompute;` to `crates/indra/src/sovereign/mod.rs`.

- [ ] **Step 4: Run and confirm pass**

```bash
cargo test -p indra --test sovereign_recompute
cargo fmt
```
Expected: 3 passed (2 skip silently if no sandbox backend is available on the CI runner, per the existing `detect_backend` skip convention used throughout the sovereign test suite).

- [ ] **Step 5: Commit**

```bash
git add crates/indra/src/sovereign/recompute.rs crates/indra/src/sovereign/mod.rs \
        crates/indra/tests/sovereign_recompute.rs
git commit -m "feat(sovereign): verify model calculations by sandboxed recomputation"
```

---

### Task 4: Wire the sandbox into specialist tool execution — closes the biggest correctness gap

**Files:**
- Modify: `crates/indra/src/agents/platform_extensions/code_execution.rs`
- Modify: `crates/indra/src/agents/agent.rs` (call site for code-execution tool dispatch — parity)
- Modify: `crates/indra/src/agents/state_machine/ops_toolcalling.rs` (same, state-machine side — parity)
- Test: `crates/indra/tests/sovereign_sandbox_wiring.rs`

**Interfaces:**
- Consumes: `sovereign::sandbox::{detect_backend, SandboxBackend, SandboxOutput}` (exists), `events::{emit, IndraEvent}` (Task 1).
- Produces: a code-execution tool call path that actually runs inside `sovereign::sandbox` rather than the existing unisolated path.

The gap this closes, stated precisely by the research pass: `sovereign::sandbox` exists and is fully tested in isolation, but nothing in `code_execution.rs` or either agent loop calls `detect_backend()`/`SandboxBackend::run()` — the specialist tool-call path still runs code through the pre-existing, explicitly-labeled-mock `developer/shell.rs` path. Without this task, A3 and A5 are true only of a module nobody calls.

- [ ] **Step 1: Read the current dispatch path before changing it**

```bash
grep -n "fn execute\|pub fn" crates/indra/src/agents/platform_extensions/code_execution.rs
```

Read the full function this returns — do not guess its signature. Note whether it already returns `anyhow::Result<...>` and what shape its success value takes, so Step 3 below matches it exactly rather than assuming.

- [ ] **Step 2: Write the failing test**

Create `crates/indra/tests/sovereign_sandbox_wiring.rs` (adjust the function name/signature in the assertions to match what Step 1 found — this is intentionally the one test in this plan whose exact call shape depends on a read-first step):

```rust
// Adjust `execute_python` to the real function name found in Step 1.
use indra::agents::platform_extensions::code_execution::execute_python;

#[test]
fn code_execution_runs_through_the_isolated_sandbox() {
    let result = execute_python("print(2 + 2)");
    match result {
        Ok(output) => assert!(output.contains('4'), "expected sandboxed output to contain 4"),
        Err(e) => {
            // Acceptable only when no sandbox backend exists on this machine —
            // must NOT silently fall back to unisolated execution instead.
            assert!(
                e.to_string().contains("no isolated sandbox available"),
                "code execution failed for a reason other than a missing sandbox: {e}"
            );
        }
    }
}

#[test]
fn code_execution_cannot_reach_the_network() {
    let code = r#"
import socket
try:
    socket.create_connection(("1.1.1.1", 53), timeout=5)
    print("REACHED")
except Exception:
    print("BLOCKED")
"#;
    if let Ok(output) = execute_python(code) {
        assert!(output.contains("BLOCKED"), "code execution reached the network — A5 violated");
    }
}
```

- [ ] **Step 3: Run and watch it fail (or reveal the current unisolated behavior)**

```bash
cargo test -p indra --test sovereign_sandbox_wiring
```
Expected: either a compile error (function name doesn't match — fix the import to match Step 1's finding) or `code_execution_cannot_reach_the_network` failing with `"REACHED"` in the output, which is exactly the bug this task fixes.

- [ ] **Step 4: Rewrite the dispatch to use `sovereign::sandbox`**

In `crates/indra/src/agents/platform_extensions/code_execution.rs`, replace the body of the execution function with:

```rust
use crate::events::{emit, IndraEvent};
use crate::sovereign::sandbox::detect_backend;
use anyhow::Result;
use std::time::Duration;

pub fn execute_python(code: &str) -> Result<String> {
    let backend = detect_backend()?;
    let output = backend.run(code, Duration::from_secs(30))?;

    if output.exit_code != 0 {
        anyhow::bail!("sandboxed execution exited {}: {}", output.exit_code, output.stderr);
    }

    Ok(output.stdout)
}
```

Keep the function's existing name and signature if Step 1 found one different from `execute_python` — the body above is what changes, not the public interface other code already calls.

- [ ] **Step 5: Emit the tool-call event around the real call site**

At the point in `agents/agent.rs` (and mirrored in `agents/state_machine/ops_toolcalling.rs`) where this function is actually invoked from a tool call, wrap it:

```rust
let call_id = uuid::Uuid::new_v4().to_string();
let started = std::time::Instant::now();

emit(
    &mut response.metadata, // or `message.metadata` in the state-machine path
    IndraEvent::ToolCall {
        call_id: call_id.clone(),
        step_id: step_id.clone(),
        name: "execute_python".to_string(),
        args: serde_json::json!({ "code": code }),
        sandbox_backend: "auto-detected".to_string(),
    },
);

let result = execute_python(&code);

emit(
    &mut response.metadata,
    IndraEvent::ToolResult {
        call_id,
        ok: result.is_ok(),
        ms: started.elapsed().as_millis() as u64,
        summary: result.as_deref().unwrap_or("execution failed").chars().take(200).collect(),
        bytes: result.as_deref().map(str::len).unwrap_or(0),
        citations: vec![],
    },
);
```

Use whatever `step_id` is already in scope at that call site (the plan/step machinery from Task 1's `PlanStep` is not built yet — pass an empty string or the operation name as a placeholder `step_id` until a later task adds real plan tracking; do not block this task on plan tracking that doesn't exist yet).

- [ ] **Step 6: Run and confirm pass**

```bash
cargo test -p indra --test sovereign_sandbox_wiring
cargo fmt
```
Expected: 2 passed (or both skip with the "no isolated sandbox available" message on a machine with neither `unshare` nor Docker — that is a correct skip, not a failure, per Task 3's `detect_backend` convention).

- [ ] **Step 7: Commit**

```bash
git add crates/indra/src/agents/platform_extensions/code_execution.rs \
        crates/indra/src/agents/agent.rs crates/indra/src/agents/state_machine/ops_toolcalling.rs \
        crates/indra/tests/sovereign_sandbox_wiring.rs
git commit -m "fix(sovereign): route code-execution tool calls through the isolated sandbox, not the mock shell path"
```

---

### Task 5: Unify citation provenance — add `doc_id`/`version`/`conf` to match the event contract

**Files:**
- Modify: `crates/indra/src/sovereign/provenance.rs`
- Modify: `crates/indra/src/sovereign/ingest.rs` (its `OcrLine` already carries `confidence` — this task threads it into `Citation` instead of dropping it)
- Modify: `crates/indra/src/sovereign/deliverable.rs` (from Task 2 — update its `Citation` field usage to match)
- Test: `crates/indra/tests/sovereign_ingest.rs` (extend existing tests, do not replace them)

**Interfaces:**
- Modifies: `sovereign::provenance::{SourceBox, Citation}` gain `doc_id`, `version`, `conf` fields. This is a breaking change to Task 2's `Finding`/`build_approval_note` test fixtures — this task must run after Task 2, and its Step 4 updates those fixtures in place.
- Produces: `provenance::SourceBox::to_event_box(&self) -> events::SourceBox` — the explicit conversion from the ingest-pipeline type to the UI-facing event type named in Task 1.

- [ ] **Step 1: Extend the failing-first test**

Add to `crates/indra/tests/sovereign_ingest.rs` (do not delete the existing two tests):

```rust
#[test]
fn ocr_line_confidence_survives_into_a_citation_box() {
    use indra::sovereign::provenance::SourceBox;

    let box_ = SourceBox {
        doc_id: "NDT_2026_08.pdf".to_string(),
        version: "v3".to_string(),
        page: 1,
        x: 100,
        y: 220,
        w: 400,
        h: 24,
        conf: 0.42,
    };
    let event_box = box_.to_event_box();
    assert_eq!(event_box.conf, 0.42);
    assert_eq!(event_box.doc_id, "NDT_2026_08.pdf");
}
```

- [ ] **Step 2: Run and watch it fail**

```bash
cargo test -p indra --test sovereign_ingest ocr_line_confidence
```
Expected: FAIL — no field `doc_id` on `SourceBox`, no method `to_event_box`.

- [ ] **Step 3: Update the type**

Replace the contents of `crates/indra/src/sovereign/provenance.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceBox {
    pub doc_id: String,
    pub version: String,
    pub page: u32,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub conf: f32,
}

impl SourceBox {
    pub fn to_event_box(&self) -> crate::events::SourceBox {
        crate::events::SourceBox {
            doc_id: self.doc_id.clone(),
            version: self.version.clone(),
            page: self.page,
            bbox: [self.x, self.y, self.w, self.h],
            conf: self.conf,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub document_id: String,
    pub text: String,
    pub boxes: Vec<SourceBox>,
}
```

- [ ] **Step 4: Fix every call site the wider fields break**

```bash
cargo build -p indra --tests 2>&1 | grep "missing field\|no field"
```

This will list every `SourceBox { .. }` literal missing `doc_id`/`version`/`conf` — expect hits in `crates/indra/src/sovereign/ingest.rs` (the `parse_tsv` function that builds `OcrLine`, which does not construct `SourceBox` directly today — check whether `OcrLine.bbox` needs its own small struct or should be folded into the new `SourceBox`; read the current `OcrLine` definition before deciding, since Task 1 and this task must not silently create two incompatible box shapes) and in Task 2's `deliverable.rs` test fixtures (`crates/indra/tests/sovereign_approval_note.rs`). Fix each by threading through the real `doc_id` (the filename already available at each call site) and `version` (use the literal string `"v1"` where no version-tracking system exists yet — Task 8 does not add document versioning, so this is an honest placeholder, not a guess dressed as data; do not invent a fake version scheme here).

- [ ] **Step 5: Run and confirm pass**

```bash
cargo test -p indra --test sovereign_ingest
cargo test -p indra --test sovereign_approval_note
cargo fmt
```
Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add crates/indra/src/sovereign/provenance.rs crates/indra/src/sovereign/ingest.rs \
        crates/indra/src/sovereign/deliverable.rs crates/indra/tests/sovereign_ingest.rs \
        crates/indra/tests/sovereign_approval_note.rs
git commit -m "feat(sovereign): carry doc_id/version/confidence on citation boxes, matching the event contract"
```

---

### Task 6: Local embedding generation — the missing prerequisite for real memory

**Files:**
- Modify: `crates/indra-local-inference/src/llamacpp/mod.rs`
- Create: `crates/indra-local-inference/src/embedding.rs`
- Modify: `crates/indra-local-inference/src/lib.rs`
- Test: `crates/indra-local-inference/tests/embedding_smoke.rs`

**Interfaces:**
- Produces: `indra_local_inference::embedding::{embed_text(model_path: &Path, text: &str) -> Result<Vec<f32>>, EMBEDDING_DIM: usize}`. Task 7 consumes both.

Verified before writing this task: `crates/indra-local-inference` has no embedding-generation path today (grep for `embed`/`Embedding` only hit unrelated substrings — "pangu-**embed**ded", "**embed**ded chat template"). `llama-cpp-2` (the binding already used elsewhere in this crate) supports embedding pooling natively; this task is new code, not a fix to broken code.

- [ ] **Step 1: Confirm the llama-cpp-2 version already in use supports embeddings**

```bash
grep -n "^llama-cpp-2" crates/indra-local-inference/Cargo.toml
```

Check that version's docs (offline: `cargo doc -p llama-cpp-2 --no-deps --open` if available, otherwise read `~/.cargo/registry/src/*/llama-cpp-2-*/src/context/params.rs` for an `embeddings` field on the context params) for `LlamaContextParams::with_embeddings(true)` or equivalent. This crate is already a dependency — no network access needed to check its API.

- [ ] **Step 2: Write the failing test**

Create `crates/indra-local-inference/tests/embedding_smoke.rs`:

```rust
use indra_local_inference::embedding::embed_text;
use std::path::Path;

#[test]
fn embedding_of_real_text_is_a_nonzero_vector_of_fixed_dimension() {
    let model_path = std::env::var("INDRA_TEST_EMBEDDING_MODEL").unwrap_or_default();
    if model_path.is_empty() {
        eprintln!("INDRA_TEST_EMBEDDING_MODEL not set; skipping (no model on this machine)");
        return;
    }

    let a = embed_text(Path::new(&model_path), "wall thickness measurement").unwrap();
    let b = embed_text(Path::new(&model_path), "wall thickness measurement").unwrap();

    assert!(!a.iter().all(|&x| x == 0.0), "embedding was all zeros");
    assert_eq!(a.len(), b.len(), "dimension must be stable across calls");
    assert_eq!(a, b, "identical input must produce identical embedding (determinism)");
}

#[test]
fn different_text_produces_different_embeddings() {
    let model_path = std::env::var("INDRA_TEST_EMBEDDING_MODEL").unwrap_or_default();
    if model_path.is_empty() {
        return;
    }

    let a = embed_text(Path::new(&model_path), "wall thickness measurement").unwrap();
    let b = embed_text(Path::new(&model_path), "quarterly revenue report").unwrap();
    assert_ne!(a, b, "unrelated text produced identical embeddings");
}
```

This test is gated on an env var rather than a bundled model, matching the existing convention in this crate's other tests (`gguf_filename_parsing.rs` sidesteps needing a real model file; embedding tests need a *loadable* model, which a demo machine may not have yet — skipping cleanly is correct here, not a shortcut).

- [ ] **Step 3: Run and watch it fail**

```bash
cargo test -p indra-local-inference --test embedding_smoke
```
Expected: FAIL — unresolved import `indra_local_inference::embedding`.

- [ ] **Step 4: Implement**

Read `crates/indra-local-inference/src/llamacpp/mod.rs` in full before writing this — it already owns the pattern for loading a GGUF model and creating a `LlamaContext`; reuse that loading code path rather than duplicating model-loading logic. The sketch below assumes the existing loader exposes (or can be minimally extended to expose) a raw `LlamaContext`; adjust names to match what Step 1's read actually finds:

```rust
// crates/indra-local-inference/src/embedding.rs
use anyhow::{bail, Result};
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::{params::LlamaModelParams, LlamaModel};
use std::path::Path;

pub fn embed_text(model_path: &Path, text: &str) -> Result<Vec<f32>> {
    let backend = LlamaBackend::init()?;
    let model_params = LlamaModelParams::default();
    let model = LlamaModel::load_from_file(&backend, model_path, &model_params)?;

    let ctx_params = LlamaContextParams::default().with_embeddings(true);
    let mut ctx = model.new_context(&backend, ctx_params)?;

    let tokens = model.str_to_token(text, llama_cpp_2::model::AddBos::Always)?;
    if tokens.is_empty() {
        bail!("text tokenized to zero tokens");
    }

    let mut batch = LlamaBatch::new(tokens.len(), 1);
    for (i, token) in tokens.iter().enumerate() {
        let is_last = i == tokens.len() - 1;
        batch.add(*token, i as i32, &[0], is_last)?;
    }
    ctx.decode(&mut batch)?;

    let embedding = ctx.embeddings_seq_ith(0)?;
    Ok(embedding.to_vec())
}
```

The exact method names (`embeddings_seq_ith`, `with_embeddings`, `LlamaBatch::add`'s signature) depend on the pinned `llama-cpp-2` version — Step 1's read is what makes these real rather than guessed; do not skip it and paste this verbatim if the API differs.

Add `pub mod embedding;` to `crates/indra-local-inference/src/lib.rs`.

- [ ] **Step 5: Run against a real model if one is available, otherwise confirm the clean skip**

```bash
cargo build -p indra-local-inference --tests
cargo test -p indra-local-inference --test embedding_smoke
cargo fmt
```
Expected: compiles; tests either pass (2) or skip cleanly with the "not set" message — a compile failure is the only unacceptable outcome at this step.

- [ ] **Step 6: Commit**

```bash
git add crates/indra-local-inference/src/embedding.rs crates/indra-local-inference/src/lib.rs \
        crates/indra-local-inference/tests/embedding_smoke.rs
git commit -m "feat(local-inference): add local text embedding via llama.cpp embedding pooling"
```

---

### Task 7: Real semantic memory — replace flat-file category storage with embeddings + cosine search

**Files:**
- Modify: `crates/indra-mcp/src/memory/mod.rs`
- Create: `crates/indra-mcp/src/memory/vector_store.rs`
- Test: (this crate uses inline `#[cfg(test)]` modules already — follow that convention; add tests inside `vector_store.rs` and extend the existing test module in `memory/mod.rs`, do not create a new top-level test file)

**Interfaces:**
- Consumes: `indra_local_inference::embedding::embed_text` (Task 6).
- Produces: `memory::vector_store::{VectorStore, VectorEntry}`, `VectorStore::insert(id: String, text: &str, embedding: Vec<f32>)`, `VectorStore::search(query_embedding: &[f32], top_k: usize) -> Vec<(String, f32)>` (id, cosine-similarity score, descending). `MemoryServer::retrieve_memories` calls `search` in addition to (not instead of) the existing category lookup, so category filtering and semantic ranking compose rather than one replacing the other.

Verified before writing this task: `MemoryServer` (`crates/indra-mcp/src/memory/mod.rs`, 851 lines) stores memories as category-tagged text files with no embeddings, no vector search, no graph — `retrieve_memories` is a category/substring lookup today. This task adds a real vector index alongside it; it does not touch the existing file-storage format, so memories written before this task remain readable (a new call re-embeds on first read and caches the result — see Step 4).

- [ ] **Step 1: Write the failing test**

Add to a new `crates/indra-mcp/src/memory/vector_store.rs`:

```rust
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct VectorEntry {
    pub id: String,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Default)]
pub struct VectorStore {
    entries: Vec<VectorEntry>,
}

impl VectorStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, id: String, embedding: Vec<f32>) {
        self.entries.retain(|e| e.id != id);
        self.entries.push(VectorEntry { id, embedding });
    }

    pub fn search(&self, query: &[f32], top_k: usize) -> Vec<(String, f32)> {
        let mut scored: Vec<(String, f32)> = self
            .entries
            .iter()
            .map(|e| (e.id.clone(), cosine_similarity(query, &e.embedding)))
            .collect();
        scored.sort_by(|a, b| b.1.total_cmp(&a.1));
        scored.truncate(top_k);
        scored
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_vectors_score_one() {
        let store_query = vec![1.0, 0.0, 0.0];
        let mut store = VectorStore::new();
        store.insert("a".to_string(), vec![1.0, 0.0, 0.0]);

        let results = store.search(&store_query, 1);
        assert_eq!(results[0].0, "a");
        assert!((results[0].1 - 1.0).abs() < 1e-6);
    }

    #[test]
    fn orthogonal_vectors_score_zero() {
        let mut store = VectorStore::new();
        store.insert("a".to_string(), vec![0.0, 1.0]);

        let results = store.search(&[1.0, 0.0], 1);
        assert!(results[0].1.abs() < 1e-6);
    }

    #[test]
    fn top_k_truncates_and_ranks_descending() {
        let mut store = VectorStore::new();
        store.insert("close".to_string(), vec![0.9, 0.1]);
        store.insert("far".to_string(), vec![0.1, 0.9]);
        store.insert("exact".to_string(), vec![1.0, 0.0]);

        let results = store.search(&[1.0, 0.0], 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "exact");
        assert_eq!(results[1].0, "close");
    }

    #[test]
    fn reinserting_an_id_replaces_rather_than_duplicates() {
        let mut store = VectorStore::new();
        store.insert("a".to_string(), vec![1.0, 0.0]);
        store.insert("a".to_string(), vec![0.0, 1.0]);
        assert_eq!(store.len(), 1);
    }
}
```

- [ ] **Step 2: Run and confirm the new module's own tests pass in isolation**

```bash
cargo test -p indra-mcp memory::vector_store
```
Expected: 4 passed — this module has no external dependency yet, so it should pass immediately; the TDD "watch it fail" step here is Step 1 itself not compiling until the module is registered (Step 3).

- [ ] **Step 3: Register the module and confirm it compiles**

Add `mod vector_store;` inside `crates/indra-mcp/src/memory/mod.rs` (check whether the surrounding module already re-exports its submodules publicly — match that convention).

```bash
cargo build -p indra-mcp
cargo test -p indra-mcp memory::vector_store
```
Expected: compiles, 4 passed.

- [ ] **Step 4: Wire embeddings into `MemoryServer`**

Read `crates/indra-mcp/src/memory/mod.rs:117-355` (the `MemoryServer::new`, `remember`, `retrieve` methods) fully before editing — the exact field names for where memory text and category live are needed to write real code here, not guessed ones.

Add a `vector_store: VectorStore` field to `MemoryServer` (adjust the struct literal in `new()`/`Default` accordingly), and in `remember` (currently at line ~260), after the existing file-write logic succeeds, add:

```rust
if let Ok(embedding) = crate::memory::embed_memory_text(&params.content) {
    self.vector_store.insert(memory_id.clone(), embedding);
}
```

where `memory_id` is whatever identifier `remember` already assigns to the stored memory (read the existing code to find its real name — do not invent one). Add a small helper in `memory/mod.rs`:

```rust
fn embed_memory_text(text: &str) -> anyhow::Result<Vec<f32>> {
    let model_path = std::env::var("INDRA_EMBEDDING_MODEL")
        .map_err(|_| anyhow::anyhow!("INDRA_EMBEDDING_MODEL not configured"))?;
    indra_local_inference::embedding::embed_text(std::path::Path::new(&model_path), text)
}
```

This makes embedding failure non-fatal to `remember` (a memory still saves to disk even with no embedding model configured — category-based retrieval keeps working exactly as before; semantic search simply returns nothing until a model is configured). Add `indra-local-inference` as a dependency of `indra-mcp` if it is not already:

```bash
cargo add indra-local-inference -p indra-mcp
```

- [ ] **Step 5: Extend `retrieve_memories` to rank by semantic similarity when a query embeds successfully**

In `retrieve` (currently line ~287), after the existing category-filtered candidate list is built, add:

```rust
if let Ok(query_embedding) = embed_memory_text(&params.query) {
    let ranked = self.vector_store.search(&query_embedding, params.limit.unwrap_or(10));
    let ranked_ids: std::collections::HashSet<_> = ranked.iter().map(|(id, _)| id.clone()).collect();
    candidates.retain(|c| ranked_ids.contains(&c.id));
    candidates.sort_by_key(|c| {
        ranked.iter().position(|(id, _)| id == &c.id).unwrap_or(usize::MAX)
    });
}
```

Match `candidates`/`c.id`/`params.limit` to whatever the real local variable and field names are — read before writing, per Step 4's instruction. When no embedding model is configured, this block's `if let Ok` simply doesn't run, and `retrieve` behaves exactly as it does today (category/substring match, unranked) — this is a deliberate, tested fallback, not an oversight.

- [ ] **Step 6: Add a regression test proving the fallback**

Add to `memory/mod.rs`'s existing `#[cfg(test)] mod tests`:

```rust
#[test]
fn retrieve_still_works_with_no_embedding_model_configured() {
    std::env::remove_var("INDRA_EMBEDDING_MODEL");
    let mut server = MemoryServer::new();
    // Use the existing test helpers in this module for remember/retrieve setup —
    // read the surrounding tests in this file for the established pattern
    // (fixture directories, params construction) rather than inventing a new one.
}
```

Fill in the body using this file's own existing test fixtures (it already has tests for `remember`/`retrieve` — copy their setup pattern exactly).

- [ ] **Step 7: Run the full memory test suite**

```bash
cargo test -p indra-mcp memory
cargo fmt
```
Expected: all pass, including the pre-existing tests (this task must not regress category-based retrieval).

- [ ] **Step 8: Commit**

```bash
git add crates/indra-mcp/src/memory/ crates/indra-mcp/Cargo.toml Cargo.lock
git commit -m "feat(memory): add embedding-backed semantic search alongside existing category storage"
```

---

### Task 8: Byte-budgeted session ledger — replaces the token-based mental model in the UI

**Files:**
- Create: `crates/indra/src/session/context_ledger.rs`
- Modify: `crates/indra/src/session/mod.rs`
- Modify: `crates/indra/src/context_mgmt/mod.rs` (emit `IndraEvent::ContextDelta`/`ContextCompacted` from the existing compaction path — does not replace its token-based trigger logic, only adds visibility)
- Test: `crates/indra/tests/session_context_ledger.rs`

**Interfaces:**
- Produces: `session::context_ledger::{SessionRecord, Ref, RefKind}`, `SessionRecord::to_cbor_zstd(&self) -> Result<Vec<u8>>`, `SessionRecord::byte_size(&self) -> usize`. This is additive — the existing token-based `context_mgmt` compaction trigger is unchanged; this task gives the *design doc's* byte-ledger a real, separately-tracked representation that the UI can render per §6.3, without redesigning when compaction actually fires (that trigger logic in `context_mgmt/mod.rs`'s 1348 lines is out of scope for this plan — replacing a mature, tested compaction trigger is a bigger, riskier change than adding a parallel, observational ledger).

- [ ] **Step 1: Write the failing test**

Create `crates/indra/tests/session_context_ledger.rs`:

```rust
use indra::session::context_ledger::{Ref, RefKind, SessionRecord};

#[test]
fn a_typical_session_record_is_a_few_kilobytes() {
    let mut record = SessionRecord::new("session-1");
    record.add_ref(Ref {
        kind: RefKind::Doc,
        id: "NDT_2026_08.pdf".to_string(),
        bytes: 412,
        label: "NDT_2026_08.pdf v3 pages 1,4,7".to_string(),
    });
    record.add_ref(Ref {
        kind: RefKind::Fact,
        id: "fact-1".to_string(),
        bytes: 156,
        label: "Wall thickness 6.2 mm at grid E-4".to_string(),
    });

    let encoded = record.to_cbor_zstd().unwrap();
    assert!(encoded.len() < 8_000, "typical session exceeded the 8kB target: {}", encoded.len());
}

#[test]
fn byte_size_matches_encoded_length() {
    let mut record = SessionRecord::new("session-2");
    record.add_ref(Ref {
        kind: RefKind::Decision,
        id: "d1".to_string(),
        bytes: 198,
        label: "Use ASME B31.3 remaining-life method".to_string(),
    });

    let encoded = record.to_cbor_zstd().unwrap();
    assert_eq!(record.byte_size(), encoded.len());
}

#[test]
fn exceeding_the_hard_cap_is_reported_not_panicked() {
    let mut record = SessionRecord::new("session-3");
    for i in 0..10_000 {
        record.add_ref(Ref {
            kind: RefKind::Fact,
            id: format!("fact-{i}"),
            bytes: 140,
            label: "x".repeat(140),
        });
    }
    assert!(record.byte_size() > 32_768, "test setup should exceed the cap");
    assert!(record.exceeds_hard_cap(), "record must report when it is over the 32kB cap");
}
```

- [ ] **Step 2: Add dependencies**

```bash
cargo add ciborium -p indra
cargo add zstd -p indra
```

- [ ] **Step 3: Run and watch it fail**

```bash
cargo test -p indra --test session_context_ledger
```
Expected: FAIL — unresolved import `indra::session::context_ledger`.

- [ ] **Step 4: Implement**

Create `crates/indra/src/session/context_ledger.rs`:

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};

const HARD_CAP_BYTES: usize = 32_768;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefKind {
    Doc,
    Chunk,
    Decision,
    Artifact,
    Fact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ref {
    pub kind: RefKind,
    pub id: String,
    pub bytes: usize,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: String,
    pub refs: Vec<Ref>,
}

impl SessionRecord {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            refs: Vec::new(),
        }
    }

    pub fn add_ref(&mut self, r: Ref) {
        self.refs.push(r);
    }

    pub fn to_cbor_zstd(&self) -> Result<Vec<u8>> {
        let mut cbor = Vec::new();
        ciborium::into_writer(self, &mut cbor)?;
        Ok(zstd::encode_all(&cbor[..], 19)?)
    }

    pub fn byte_size(&self) -> usize {
        self.to_cbor_zstd().map(|v| v.len()).unwrap_or(usize::MAX)
    }

    pub fn exceeds_hard_cap(&self) -> bool {
        self.byte_size() > HARD_CAP_BYTES
    }
}
```

Add `pub mod context_ledger;` to `crates/indra/src/session/mod.rs`.

- [ ] **Step 5: Run and confirm pass**

```bash
cargo test -p indra --test session_context_ledger
cargo fmt
```
Expected: 3 passed.

- [ ] **Step 6: Commit**

```bash
git add crates/indra/src/session/context_ledger.rs crates/indra/src/session/mod.rs \
        crates/indra/tests/session_context_ledger.rs crates/indra/Cargo.toml Cargo.lock
git commit -m "feat(session): add byte-budgeted CBOR+zstd session ledger per the context-model spec"
```

- [ ] **Step 7: Emit ledger events from the existing compaction path**

Read `crates/indra/src/context_mgmt/mod.rs` for its existing compaction entry point (the file is 1348 lines; do not read all of it — grep for `pub fn` and `pub async fn` to find the function that runs when compaction triggers). At that function's success path, add:

```rust
use crate::events::{emit, IndraEvent};

emit(
    metadata, // whatever &mut MessageMetadata is already in scope there
    IndraEvent::ContextCompacted {
        from_turns: turns_before, // existing local variable at that call site
        to_bytes: 0, // this compaction path is token-based; a real byte count
                     // requires building a SessionRecord from its output, which
                     // is a follow-on task once something actually populates
                     // SessionRecord from live conversation state — do not fake
                     // a number here.
        dropped_count: dropped.len(), // existing local variable, adjust name to match
    },
);
```

Leave `to_bytes: 0` with the comment exactly as above rather than computing a fake number — this task adds the *type* and its wire format; wiring live conversation turns into a populated `SessionRecord` end-to-end (so `to_bytes` is real) is out of scope here because it requires deciding which parts of existing conversation state map to `Ref::Doc` vs `Ref::Fact` vs `Ref::Decision`, which is a product decision belonging to whoever builds the Work/Context-ledger UI screens, not a mechanical wiring step.

- [ ] **Step 8: Commit**

```bash
git commit -am "feat(session): emit context.compacted events from the existing compaction trigger"
```

---

### Task 9: Remove the token/cost UI that contradicts the byte-ledger design

**Files:**
- Delete: `ui/desktop/src/components/MessageUsageStats.tsx`
- Delete: `ui/desktop/src/components/MessageUsageStats.test.tsx`
- Delete: `ui/desktop/src/components/bottom_menu/CostTracker.tsx`
- Delete: `ui/desktop/src/components/bottom_menu/ContextWindowIndicator.tsx`
- Modify: `ui/desktop/src/components/ChatInput.tsx`
- Modify: `ui/desktop/src/components/IndraMessage.tsx`
- Modify: `ui/desktop/src/components/settings/config/ConfigSettings.tsx` (remove the `showPricing` toggle)
- Test: run the existing Vitest suite, do not write new UI tests for a deletion

This is the "token usage screen" removal you asked for — confirmed by direct code search to be these three components (`MessageUsageStats`, `CostTracker`, `ContextWindowIndicator`) plus their wiring, not a single screen. Deleting them without replacement is correct for this plan: Task 8 gives the backend a byte-ledger to report instead, but the *ledger UI itself* (§6.3's gauge and sheet) is a UI-plan concern, out of scope here per this plan's Non-Goals — this task only removes the contradicting old surface so it cannot be confused with, or accidentally kept alongside, whatever replaces it later.

- [ ] **Step 1: Find every remaining reference before deleting anything**

```bash
grep -rln "MessageUsageStats\|CostTracker\|ContextWindowIndicator" ui/desktop/src/
```

Confirm this matches the set already found by prior research (`ChatInput.tsx`, `IndraMessage.tsx`, plus the two component files and their test) — if it finds additional files, add them to this task's file list before proceeding; do not silently expand scope without noting it.

- [ ] **Step 2: Remove the imports and usages first (so the build stays green mid-task)**

In `ui/desktop/src/components/IndraMessage.tsx`, remove the `import` line for `MessageUsageStats` and the JSX block that renders it.

In `ui/desktop/src/components/ChatInput.tsx`, remove the `import` lines for `CostTracker` and `MessageUsageStats` (if imported there for prop-drilling) and their usages.

- [ ] **Step 3: Run the desktop build to catch anything still referencing the components**

```bash
cd ui/desktop && pnpm run typecheck
```
Expected: type errors pointing at any remaining import you missed in Step 2 — fix each until this passes.

- [ ] **Step 4: Delete the component files**

```bash
git rm ui/desktop/src/components/MessageUsageStats.tsx \
       ui/desktop/src/components/MessageUsageStats.test.tsx \
       ui/desktop/src/components/bottom_menu/CostTracker.tsx \
       ui/desktop/src/components/bottom_menu/ContextWindowIndicator.tsx
```

- [ ] **Step 5: Remove the `showPricing` setting**

In `ui/desktop/src/components/settings/config/ConfigSettings.tsx`, remove the toggle control and its associated `getSetting('showPricing')`/`setSetting('showPricing', ...)` calls. Grep to confirm nothing else reads this setting key:

```bash
grep -rn "showPricing" ui/desktop/src/
```

If any remain (the research pass flagged `ModelSettingsPanel.tsx`, `SwitchModelModal.tsx`, `PrivacyInfoModal.tsx`, `useAnalytics.ts`, `useChatSessionTypes.ts`/`useChatSession.ts` as possibly referencing token/usage concepts, not necessarily `showPricing` specifically) — inspect each hit individually; a reference to "usage" meaning API-usage-tracking-for-analytics is not the same as a token-count display and should not be deleted by this task. Only remove code that displays or gates token counts, cost, or context-window-fill to the end user.

- [ ] **Step 6: Run the full desktop test suite**

```bash
cd ui/desktop && pnpm run typecheck && pnpm run test:run
```
Expected: passes with no references to the deleted components.

- [ ] **Step 7: Commit**

```bash
git add -u ui/desktop/src/
git commit -m "chore(ui): remove token/cost display surfaces ahead of the byte-ledger context model"
```

---

## Explicitly deferred (separate plans, not this one)

- **Memory screen UI** (constellation / ledger / timeline, §5.5) — write once Task 7's `VectorStore` API is stable, so the UI plan's data contracts come from a real API instead of the design doc's illustrative shape.
- **Tauri migration** (§2.1 decision 6) — a full desktop-shell rewrite; this plan's backend work does not depend on it and should not wait for it.
- **PDF → image rendering** for the ingest pipeline (`ocr_page` currently takes an image, not a PDF) — needs a rasterization dependency decision (e.g. `pdfium-render`) that hasn't been made.
- **Full IndraEvent plan-tracking** (`plan.proposed`/`plan.revised`/`step.state` actually being emitted from a real planning loop, not just having a type defined) — Task 1 defines the wire format; nothing in the current agent loop tracks multi-step plans as a first-class concept yet, and retrofitting that is a bigger, separate change to the agent loop's control flow.
