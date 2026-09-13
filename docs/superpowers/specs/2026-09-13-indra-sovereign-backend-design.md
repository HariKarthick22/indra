# INDRA — Sovereign On-Premise Agentic AI Workbench

**Implements:** Smart India Hackathon problem statement **26117**, Mangalore Refinery and
Petrochemicals Limited — *Sovereign On-Premise Agentic AI Workbench using Open-Weight
Multimodal LLMs for Confidential Industrial Work*.

**Target:** a demonstrable prototype on a single workstation or mid-range GPU server.
**Timeline:** one week.

**Builds on:** the Rust fork at `crates/indra`. `INDRA_Plant_Integration_Plan.md` remains
the reference for local-inference performance work; `INDRA_Full_Plan.md` remains the
reference for the phased feature set. Neither is the reference for deployment, identity,
or write safety — this document is.

---

## 1. Acceptance criteria

These are the problem statement's own judging criteria. Everything in this spec exists to
serve one of them. Anything serving none of them is out of scope.

| # | Criterion | Demonstrated by |
|---|---|---|
| **A1** | Model auto-selection across at least two task types | A coding request and a document-summary request visibly select different models, with the reason shown |
| **A2** | An agentic task carried end to end | Read a scanned inspection report → extract findings → draft an approval note as a `.docx` |
| **A3** | A coding task run and verified in a sandbox | Code generated, executed, output checked, result reported |
| **A4** | A multimodal task | Understanding of a scanned document, engineering drawing, or photograph |
| **A5** | Proof that no external call is made at any point | A live panel showing outbound attempts, plus a deliberately triggered violation being blocked and logged |

A5 is the load-bearing one. The problem statement is explicit: proof "is the actual proof
of the sovereign claim, not just a statement of it."

## 2. Deployment context

MRPL is a 15 MMTPA refinery, an ONGC subsidiary under the Ministry of Petroleum and
Natural Gas — a Government of India PSU operating designated critical infrastructure.

- **No outbound network, ever.** No cloud control plane, no hosted identity, no telemetry,
  no email egress. Google OAuth and email invitations are removed from the design; neither
  can complete without internet access.
- **No proprietary data is required for the demo.** Public sample scanned PDFs and open
  P&ID datasets only. SAP PM, OSIsoft PI and SharePoint connectors are therefore **not
  built** — but the write-safety model that governs them is, because it also governs the
  files the agent touches.

## 3. Write safety

The property INDRA guarantees is **no unreviewed side effect** — not "read-only".
Read-only achieves the property by removing the system's ability to act, which reduces
INDRA to a search engine with citations.

Every resource has exactly one write class, a property of the resource rather than of the
user, agent, or session.

| Class | Applies to | Agent may | Undo |
|---|---|---|---|
| **Sealed** | Source documents; any OT source (OSIsoft PI, Purdue Level ≤3) | Read only | None — nothing is written |
| **Versioned** | Generated deliverables, INDRA's own store | Write a new version | Restore prior version |
| **Gated** | External systems of record (SAP PM) — *designed, not built this week* | Emit a typed proposal only | Compensating transaction, itself gated |

**Sealed is hard-wired.** Not a config value, environment variable, or policy entry. No
code path writes to a Sealed resource, so no misconfiguration can create one.

**Gated undo is not erasure.** Reversing TECO on a SAP PM order re-fires the purchase
requisition workflow and, where partial settlement has run, produces unbalanced cost
postings needing Controlling to correct. A compensating transaction is a new proposal
requiring its own approval, never a silent rollback. This is why Gated is specified now
even though it is not built now: designing it later would invite a naive rollback layer.

### 3.1 Two invariants

**The memory layer is not a security boundary.** Its role is provenance, retrieval, and
institutional memory. Security boundaries must be narrow, auditable, and non-inferential;
a memory graph is none of those. The boundary is the write-class registry plus the
sandbox. A defect in memory must degrade answer quality, never authorization.

**Authority is mirrored, never granted.** Where a directory exists, group membership and
existing authorizations decide who may approve. INDRA's own team concept scopes *what work
is visible* and carries no security weight. For the demo, a local user table implements the
same interface. This mirrors the rule already set for SharePoint ACLs in
`INDRA_Plant_Integration_Plan.md` — mirror existing permissions rather than re-deriving
them, because re-derivation is how data leaks.

## 4. Architecture

```
Electron client ──ACP──► INDRA server (single host, no egress)
                          │
                          ├─ OperationRouter ──► ModelRegistry  (capability match)
                          ├─ SpecialistRegistry
                          ├─ Planner ──────────► task DAG
                          ├─ Executor ─────────► concurrent nodes, traced
                          ├─ Tool Fabric ──────► docx / xlsx / pdf / shell / search
                          ├─ Sandbox ──────────► no network namespace
                          ├─ Ingest ───────────► render → OCR(boxes) → VLM(meaning)
                          ├─ WriteGuard ───────► Sealed / Versioned / Gated
                          └─ EgressInspector ──► live proof panel
```

### 4.1 Capability-based model routing (serves A1)

Models are **descriptor files**, not code. Adding a model is adding a file — which is the
problem statement's requirement that new open-weight models be addable "without
redesigning the system".

```rust
pub struct ModelDescriptor {
    pub id: String,
    pub backend: Backend,          // Llamacpp | Mlx
    pub path: PathBuf,
    pub mmproj_path: Option<PathBuf>,
    pub capabilities: Capabilities,
    pub min_vram_mb: u64,
}

pub struct Capabilities {
    pub vision: bool,
    pub tool_calling: ToolCalling,  // Reliable | Unreliable | None
    pub context_tokens: u32,
    pub domains: Vec<Domain>,       // Code | Document | Engineering | General
}
```

Routing matches a task's **requirements** against declared capabilities — not model size.
`IndraOperation::CodeGeneration` requires `Domain::Code` and `ToolCalling::Reliable`;
`IndraOperation::PidAnalysis` requires `vision: true`. The selection and its reason are
surfaced to the UI, so A1 is visible rather than asserted.

Selection is additionally informed by **probed hardware**. The problem statement permits a
smaller model "if 120B class hardware isn't available at the venue"; the same binary must
therefore pick a 3B model on a laptop and a 32B model on the GPU server with no config
change. `crates/` currently contains no `sysinfo`, `num_cpus`, or `available_parallelism`
call, so this probe is new.

### 4.1.1 Degrade with a warning, never fail silently

Hardware shortfall **warns; it does not exclude**. A model too large for available VRAM is
still runnable — slowly, via CPU offload — and the operator is told so rather than being
shown an empty model list at the venue.

```rust
pub enum Fit {
    Comfortable,
    Tight       { warning: String },
    Degraded    { warning: String },
    Unsupported { reason: String },
}
```

Only `Unsupported` removes a model from consideration, and only for a reason no amount of
patience fixes — a missing `mmproj` for a vision task, an absent backend, a corrupt file.
`Degraded` is returned alongside the selected model and surfaced verbatim in the UI:
*"Selected qwen-32b. Requires 20 GB VRAM, 8 GB available — offloading 18 of 64 layers to
CPU. Expect roughly 3 tokens/sec."*

The same rule governs runtime. If a model fails to load, or inference exhausts memory
mid-turn, INDRA records the failure, warns, and retries on the next-best model rather than
failing the turn. Degradation is a reported state, not an error.

### 4.1.2 Universal model support

"Any open-weight model" is served by three backends behind one interface:

| Backend | Covers | Mechanism |
|---|---|---|
| `Llamacpp` | GGUF — the bulk of open-weight releases | Native, already present |
| `Mlx` | Apple Silicon native | Native, already present at `mlx.rs` |
| `LocalHttp` | Everything else — safetensors, vLLM, Ollama, TGI | OpenAI-compatible HTTP to **loopback only** |

`LocalHttp` is what makes the system genuinely universal: any format the native backends
cannot load is served by a local runtime on `127.0.0.1` and reached over HTTP.

**This requires an explicit exception in the egress inspector, and the exception must be
narrow.** Loopback and link-local addresses are permitted; every other destination is
blocked and logged. A `LocalHttp` endpoint resolving to anything other than a loopback
address is rejected at configuration time, not at request time — otherwise "universal
model support" becomes the hole through which A5 is lost.

### 4.2 Ingest: OCR for coordinates, VLM for meaning (serves A2, A4)

The two are complementary and must not be conflated.

- **OCR (Tesseract, invoked as a subprocess with TSV output)** yields words, lines, and
  **bounding boxes**, deterministically.
- **The vision model** (llama.cpp multimodal — `mmproj_path` and `vision_capable` already
  exist at `crates/indra-local-inference/src/lib.rs:252`) yields *understanding*: what the
  drawing shows, what the finding means.

Bounding boxes come from OCR, never from the model. Local vision models are unreliable at
precise coordinates; OCR is not. Every retrieved chunk therefore retains the boxes of the
OCR lines that formed it, and every claim in a generated deliverable can be traced to a
highlighted region of the source page.

### 4.3 Sandbox with no network namespace (serves A3, A5)

Sandboxed code runs with **no network interface at all**, so it cannot make an external
call even if the generated code tries. One mechanism satisfies two criteria: genuine
isolation, and sovereignty proof that holds against adversarial code.

```rust
pub trait SandboxBackend: Send + Sync {
    fn name(&self) -> &str;
    fn run(&self, code: &str, timeout: Duration) -> Result<SandboxOutput>;
}
```

Two implementations, selected by probe at startup:

- `LinuxNamespace` — `unshare -n -p -f --mount-proc`. Zero dependencies. The plant target.
- `DockerNoNetwork` — `docker run --rm --network=none`. The macOS development fallback.

If neither is available the sandbox refuses to run rather than falling back to an
unisolated subprocess. The current `developer/shell.rs` path is honestly labelled
adapter-mock and must not be used for A3.

### 4.4 Deterministic recomputation (serves A3)

For engineering calculations, the model proposes; the sandbox decides. The model emits the
calculation as executable Python alongside its natural-language answer; the sandbox runs
it; the results are compared. A mismatch rejects the answer and retries once at a higher
capability tier. This makes `EngineeringCalcSpecialist` real rather than a stub, and
answers the objection every PSU evaluator raises: that the system invents numbers.

### 4.5 Egress proof (serves A5)

[`crates/indra/src/security/egress_inspector.rs`](../../../crates/indra/src/security/egress_inspector.rs)
already exists at 555 lines. This spec adds the demonstration surface: a live count of
outbound attempts, a per-attempt log, and a **deliberate violation control** that attempts
a known external endpoint on demand and shows it blocked and recorded.

## 5. Request lifecycle

| # | Stage | Behaviour |
|---|---|---|
| 1 | Admit | Resolve principal → capability set; resolve visibility scope separately |
| 2 | Classify | `OperationRouter` → `IndraOperation` |
| 3 | Select | Specialist, then model by capability match under probed hardware |
| 4 | Resolve | Entity/document scope for retrieval |
| 5 | Plan | Decompose into steps (the existing agent loop; concurrent DAG planning is out of scope this week) |
| 6 | Execute | Every tool call emits a trace span |
| 7 | Verify | Evidence check (claims cite retrieved sources) and, for calculations, recomputation |
| 8 | Emit | Deliverable written as a new version, through the write guard; Gated targets would produce a proposal instead |
| 9 | Record | Provenance retained, including source page boxes |

Two properties are load-bearing:

- **The agent has no direct write path.** Every write passes the write guard. Deliverables
  become new versions; Sealed resources are unreachable by construction.
- **The capability set is resolved once, at stage 1, and threaded through.** No component
  re-derives permissions.

## 6. Module placement

New module `crates/indra/src/sovereign/`. It does not extend `permission/`, which governs
per-tool-call approval, nor `security/`, which performs threat scanning and egress
inspection. Both remain unchanged.

```
crates/indra/src/sovereign/
  mod.rs
  hardware.rs        HardwareProfile, probe()
  model_registry.rs  ModelDescriptor, Capabilities, Fit, Selection, select_model()
  sandbox.rs         SandboxBackend, LinuxNamespace, DockerNoNetwork
  ingest.rs          OcrLine, ocr_page()
  provenance.rs      SourceBox, Citation
  deliverable.rs     Finding, Severity, build_approval_note()
  write_class.rs     WriteClass, write_class_for(), guard_write()
  recompute.rs       Verdict, verify_calculation()
```

## 7. Verification

Tests live in `crates/indra/tests/`, per `AGENTS.md`. Denial cases are written before
access cases.

Minimum bar:

1. No constructible call path writes to a Sealed resource.
2. An unresolved principal is denied, not defaulted.
3. `select_model` returns an error rather than a non-vision model when the operation
   requires vision — a missing capability is `Unsupported`, never `Degraded`.
4. A model whose `min_vram_mb` exceeds the probed profile is still selected when it is the
   only candidate, and returns `Fit::Degraded` carrying a warning.
5. A model load failure falls through to the next-best candidate and records a warning,
   rather than failing the turn.
6. Sandboxed code attempting an outbound connection fails, and the failure is recorded.
7. A `LocalHttp` endpoint that does not resolve to a loopback address is rejected at
   configuration time.
8. A calculation whose recomputation disagrees with the stated answer is rejected.

## Out of scope

Deliberately excluded, so nothing is discovered late:

- SAP PM, OSIsoft PI, SharePoint and SMB connectors (Gated is specified, not built)
- AD/LDAP connector — the interface is specified; a local user table implements it
- Speculative decoding, KV-cache quantization, `fit_params` tuning
  (`INDRA_Plant_Integration_Plan.md` Part A) — none is required by an acceptance criterion
- Agent definitions distributed from Git repositories
- Full agent-run audit trail
- Electron rendering performance, login and loading experience
- Team management UI
