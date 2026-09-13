# INDRA — Full Build Plan (v2)
**Sovereign Agentic AI Workbench — Phased Delivery Plan for Replit MVP**

*v2 change from v1: formalizes the Specialist (skill-routing) abstraction so engineering/domain agents — Inspection, P&ID, Engineering Calc — are real architectural citizens from Phase 3 onward, not something the generic chat path quietly absorbed.*

---

## 0. How to use this document

Your original 47-section spec is the **north star architecture doc** — keep it, don't discard it. This file turns it into an **executable sequence**: one Replit Agent prompt per phase, each producing a working, testable checkpoint before the next phase starts. Never paste more than one phase's prompt into Replit Agent at a time.

Rule for every phase: **run it → click through it → fix what's broken → only then move to the next prompt.**

---

## 1. Tech stack (locked decisions)

| Layer | Choice | Why |
|---|---|---|
| Frontend | React + TypeScript + Tailwind | Component reuse across chat/memory/audit views; responsive for free |
| Backend | Node.js (TypeScript), service-module structure (`auth/`, `chat/`, `agent/`, `context/`, `memory/`, `retrieval/`, `models/`, `specialists/`, `tools/`, `sandbox/`, `governance/`, `audit/`, `artifacts/`, `files/`) | One language across stack = faster iteration on Replit; matches original §29 service boundaries |
| DB | Postgres (Replit-managed) | Relational core fits the §28 entity list; add pgvector later — don't install it in Phase 1 |
| Search (MVP) | Postgres full-text search (`tsvector`) | Skip a vector DB entirely until well past MVP |
| Model access | `ModelGateway` interface, one real adapter + `MockProvider` fallback | Never call a provider SDK directly from route handlers |
| Specialist access | `SpecialistRegistry` interface, one real specialist + registered-but-stub others | Keeps the router's shape correct without building every domain pipeline |
| Sandbox (MVP) | Node `child_process`, timeout + resource limits, labeled `sandbox: adapter-mock` in UI | Not real isolation — say so |
| Docs generated | `docx` npm lib, `exceljs` | Small, no external service dependency |
| i18n | `react-i18next`, EN default, TR second locale | Wire mechanism in Phase 1, translate strings once UI text is stable |

---

## 2. Two routers, not one — the core architectural fix

Your own INDRA notes already say this correctly: **task-aware skill routing** (P&ID skill, Inspection skill, Coding skill, each with its own tools) is a separate decision from **model routing** (fast vs. reasoning, vision vs. text model). v1 of this plan only built the second one generically. v2 makes both real:

```
User request
    │
    ▼
UNDERSTANDING
    │
    ▼
OPERATION ROUTER          → classifies into an operation:
    │                        GENERAL_CHAT / INSPECTION_ANALYSIS / PID_ANALYSIS /
    │                        ENGINEERING_CALCULATION / CODE_GENERATION ...
    ▼
SPECIALIST SELECTION      → picks the specialist that owns this operation
    │
    ├─ InspectionAnalysisSpecialist   (REAL — Phase 3 onward)
    │     • context scope: inspection/mechanical docs for the resolved asset only
    │     • system prompt: domain framing (cite findings, flag anomalies vs. prior)
    │     • allowed tools: EnterpriseSearch, DocxGenerator
    │     • model: reasoning-tier text model
    │
    ├─ PIDAnalysisSpecialist          (STUB — registered, not implemented)
    │     • would need: vision/OCR pipeline (deferred)
    │
    └─ EngineeringCalcSpecialist      (STUB — registered, not implemented)
          • would need: deterministic solver (deferred)
    │
    ▼
CONTEXT_RESOLUTION   (Enterprise Memory, scoped by the chosen specialist + asset + ACL)
    │
    ▼
PLANNING → EXECUTION (specialist's tools) → VERIFICATION → APPROVAL_REQUIRED
    │
    ▼
DELIVERY → AUDIT
```

A `Specialist` is defined by: **name, owned operation(s), context-scope rule, system-prompt template, allowed tools, preferred model tier.** New domain agents (P&ID, Engineering Calc, HSE, whatever comes next) are added by implementing this interface — the router, memory, and audit systems never need to change.

---

## 3. Phased plan

### **Phase 1 — Application Shell** ✅ start here
**Goal:** Three-zone responsive workspace exists, feels like a real product, does nothing real yet.

**In scope:** Sidebar (nav placeholders: New Chat, Conversations, Enterprise Memory, Files, Projects, Tools, Models, Runs, Audit, Settings — collapsible), center chat panel with mocked streaming response, right context panel (empty state), auth placeholder (single fake user), dark/light theme, responsive to tablet.

**Out of scope:** real model calls, real DB beyond a `users` stub, mobile redesign, memory tree.

**Checkpoint:** app loads, mock chat works, panels collapse, layout holds on resize.

**Replit Agent prompt:**
```
Build INDRA — a Claude/Codex-style AI workbench, web app.
Stack: React + TypeScript + Tailwind frontend, Node.js + TypeScript backend, Postgres.

Layout: three-zone workspace.
- Left sidebar (collapsible): nav items New Chat, Conversations, Enterprise Memory,
  Files, Projects, Tools, Models, Runs, Audit, Settings — all as placeholder routes/pages.
- Center panel: chat interface. User can type and send a message; backend returns a
  MOCKED streamed text response (no real AI call yet). Support markdown rendering and
  code block syntax highlighting in messages.
- Right panel (collapsible): empty state placeholder only, labeled "Context".

Include: dark/light theme toggle, single fake authenticated user (no real auth flow),
responsive layout that degrades gracefully to tablet width.

Do NOT implement: real model integration, tools, sandbox, memory tree, RAG, audit,
approvals, or mobile-specific redesign. Those come in later phases.

Visual design: clean, minimal, professional, AI-native — not an enterprise ERP
dashboard. Subtle borders, rounded cards, restrained animation.

After building, run the app and verify: sidebar collapses, message send/receive
works with the mock response, layout holds at tablet width.
```

---

### **Phase 2 — Real Chat + Model Gateway**
**Goal:** Replace the mock with a real streaming model call, behind an abstraction.

**In scope:** `ModelGateway` interface (`generate()`, `stream()`), one real provider adapter via env var/Secrets, `MockProvider` fallback, conversation persistence (`conversations` + `messages` tables, create/rename/delete/search), stop-generation, regenerate, edit-message, copy buttons.

**Out of scope:** tools, specialists/routing, file upload.

**Checkpoint:** real model answers, conversations persist across reload, stop/regenerate work.

**Prompt:**
```
Add real AI chat to INDRA (existing three-zone shell from Phase 1).

Create a ModelGateway abstraction (backend/models/) with:
- a generate/stream interface, provider-agnostic
- one real provider adapter using an API key from Secrets/env var (never hardcoded)
- a MockProvider used automatically if no key is configured, clearly labeled in the UI
  ("Mock model — no provider configured") so it's obvious which is active

Persist conversations and messages in Postgres (conversations, messages tables).
Add: new/rename/delete/search conversation, stop-generation button, regenerate last
response, edit a sent user message, copy button on responses and on code blocks.

Do not add tools, specialists, file upload, or routing logic yet — one model, one path.
Run and verify: real streamed response appears, refreshing the page keeps history,
stop/regenerate/edit all work.
```

---

### **Phase 3 — Agent Runtime + Operation Router + Specialist Registry**
**Goal:** Introduce the real agentic loop AND the specialist/skill-routing abstraction together — this is the phase that was incomplete in v1.

**In scope:**
- `AgentRuntime` state machine: `REQUEST_RECEIVED → UNDERSTANDING → CONTEXT_RESOLUTION → PLANNING → EXECUTION → VERIFICATION → APPROVAL_REQUIRED → DELIVERY → COMPLETED` (+ `FAILED`).
- `OperationRouter` classifying requests into: `GENERAL_CHAT`, `CODE_GENERATION`, `INSPECTION_ANALYSIS`, `PID_ANALYSIS`, `ENGINEERING_CALCULATION` (keyword/simple-intent classification is fine — no need for a trained classifier yet).
- `Specialist` interface: name, owned operation(s), context-scope rule, system-prompt template, allowed tools, preferred model tier.
- `SpecialistRegistry` with **one real specialist**, `GeneralChatSpecialist` (wraps Phase 2's plain chat path), and **two registered-but-stub specialists**: `PIDAnalysisSpecialist`, `EngineeringCalcSpecialist` (return "not yet implemented" if ever selected).
- `ExecutionTimeline` UI showing safe status labels ("Understanding request", "Selecting specialist", "Validating result"...) — never real chain-of-thought — and now also showing **which specialist was selected**.

**Out of scope:** real tools, real verification/approval logic (still trivial pass/auto-approve), memory/RAG, the real `InspectionAnalysisSpecialist` (built in Phase 5 once memory/RAG exist to scope it against).

**Checkpoint:** every message visibly passes through the state machine AND shows a specialist name (e.g. "General Chat Specialist") in the timeline; a request like "analyze the P-101 NDT report" classifies as `INSPECTION_ANALYSIS` and shows "Inspection Analysis Specialist — not yet available" rather than silently falling through to generic chat.

**Prompt:**
```
Add an AgentRuntime, OperationRouter, and Specialist registry to INDRA (existing
chat from Phase 2).

1. AgentRuntime state machine: REQUEST_RECEIVED -> UNDERSTANDING ->
   CONTEXT_RESOLUTION -> PLANNING -> EXECUTION -> VERIFICATION ->
   APPROVAL_REQUIRED -> DELIVERY -> COMPLETED, with FAILED reachable from any step.
   VERIFICATION and APPROVAL_REQUIRED exist as real states but with trivial logic
   for now (always pass / auto-approve) — the shape matters now, the depth comes
   in Phase 7.

2. OperationRouter: classify each request into one of GENERAL_CHAT,
   CODE_GENERATION, INSPECTION_ANALYSIS, PID_ANALYSIS, ENGINEERING_CALCULATION.
   Simple keyword/intent matching is fine (e.g. mentions of "inspection", "NDT",
   "report" -> INSPECTION_ANALYSIS; "P&ID", "drawing", "diagram" -> PID_ANALYSIS;
   "calculate", "load", "pressure" -> ENGINEERING_CALCULATION).

3. Define a Specialist interface: name, owned operation(s), context-scope rule,
   system-prompt template, allowed tools list, preferred model tier. Build a
   SpecialistRegistry. Implement ONE real specialist, GeneralChatSpecialist,
   handling GENERAL_CHAT and CODE_GENERATION by wrapping the existing Phase 2
   chat path. Register two stub specialists, PIDAnalysisSpecialist and
   EngineeringCalcSpecialist, that return a clear "this specialist is not yet
   implemented" response if ever selected — do not implement their real logic.

4. Add an ExecutionTimeline UI element in the chat showing the current state with
   safe non-technical labels, AND which specialist was selected for this request.

Run and verify: a plain question shows "General Chat Specialist" in the timeline
and answers normally. A message like "analyze the P-101 NDT report" is classified
as INSPECTION_ANALYSIS and — since no real Inspection specialist exists yet in
this phase — shows a clear "not yet available" state rather than silently
answering as generic chat.
```

---

### **Phase 4 — Enterprise Memory (read-only tree) + Synthetic Demo Data**
**Goal:** The Obsidian-like org/asset tree exists and is browsable, seeded with your synthetic MRPL/P-101 data.

**In scope:** `memory_nodes`, `documents`, `document_metadata` tables; `MemoryTree` UI for MRPL → Departments → Assets; metadata panel per node; seed script for the demo docs (`NDT_2026_08.pdf`, `Pump_Inspection_SOP.pdf`, `P101_Previous_Inspection.pdf`, `Vibration_2026.xlsx`), clearly labeled synthetic.

**Out of scope:** RAG/search over documents (next phase), ACL enforcement logic (structure fields now, enforce later), OCR.

**Checkpoint:** browse MRPL → Maintenance → Mechanical → P-101, see its metadata, see the 4 attached synthetic documents.

**Prompt:**
```
Add Enterprise Memory to INDRA.

Create memory_nodes, documents, document_metadata tables. Build a tree structure:
MRPL org -> Departments (Operations, Maintenance [Mechanical, Electrical,
Instrumentation], Engineering, Inspection, HSE, Projects, Management) -> Assets
(seed just P-101 under Maintenance/Mechanical for now).

Each node has metadata: Department, Domain, Classification, Owner, Allowed
Operations. Build a MemoryTree UI component in the left sidebar's "Enterprise
Memory" route: expandable tree, click a node to see its metadata and attached
documents in the right context panel.

Seed P-101 with 4 SYNTHETIC demo documents (label them clearly as synthetic/demo
data, not real): NDT_2026_08.pdf, Pump_Inspection_SOP.pdf,
P101_Previous_Inspection.pdf, Vibration_2026.xlsx. Placeholder text content is
fine for now — the goal is the tree/metadata/attachment structure.

Do not implement document search, OCR, or ACL enforcement yet — just the
browsable structure and metadata display.

Run and verify: full tree browsable, P-101 shows its 4 documents and metadata.
```

---

### **Phase 5 — RAG (full-text) + Citations + Real InspectionAnalysisSpecialist**
**Goal:** The Inspection specialist stops being a stub — it actually retrieves scoped memory and answers with citations. This is where the two routers from Section 2 finally meet.

**In scope:**
- Synthetic-but-real text content in the 4 seeded documents.
- Postgres full-text search (`tsvector`) over `document_chunks`.
- `ContextManager` scoping retrieval to a resolved asset + domain (not the whole memory tree).
- **`InspectionAnalysisSpecialist` implemented for real**: owns `INSPECTION_ANALYSIS`, context-scope = inspection/mechanical docs for the resolved asset, system prompt framed for inspection analysis (cite findings, flag anomalies vs. prior inspection), allowed tools = `EnterpriseSearch` (only tool that exists so far).
- Citations rendered in chat (doc, section, asset).

**Out of scope:** vector/graph search, OCR/vision, `PIDAnalysisSpecialist`/`EngineeringCalcSpecialist` (still stubs), DOCX generation (next phase).

**Checkpoint:** "Summarize the P-101 NDT report" is classified `INSPECTION_ANALYSIS`, routed to the real `InspectionAnalysisSpecialist`, retrieves only P-101's scoped chunks, and answers with a visible citation to `NDT_2026_08.pdf` — timeline shows "Inspection Analysis Specialist" instead of the Phase 3 "not yet available" state.

**Prompt:**
```
Add RAG and implement the real Inspection specialist in INDRA.

Populate the 4 P-101 documents (from Phase 4) with short synthetic realistic text
content — inspection findings, SOP steps, prior inspection notes, vibration
readings. Chunk into a document_chunks table. Implement full-text search only
(Postgres tsvector) — no vector DB yet.

Build a ContextManager: given a query and a resolved asset (P-101), retrieves
only that asset's relevant chunks, scoped further by domain (inspection/
mechanical), and assembles a context pack with source metadata (document,
section/page, asset, classification).

Implement InspectionAnalysisSpecialist for real (replacing its Phase 3 stub):
owns the INSPECTION_ANALYSIS operation, uses the ContextManager scoped to
inspection/mechanical docs for the resolved asset, uses a system prompt framed
for inspection analysis (cite specific findings, compare against prior
inspection where relevant), allowed tool = EnterpriseSearch (wraps the
ContextManager). PIDAnalysisSpecialist and EngineeringCalcSpecialist remain
stubs — do not implement them.

When the specialist answers using retrieved context, render a citation/source
card in the chat UI.

Run and verify: "what did the NDT report on P-101 find?" is classified
INSPECTION_ANALYSIS, the timeline shows "Inspection Analysis Specialist" (not
the old "not yet available" stub state), and the answer shows a citation to
NDT_2026_08.pdf.
```

---

### **Phase 6 — Tool Fabric (2 real tools) + Sandbox Adapter**
**Goal:** The Inspection specialist can actually *do* something beyond talk — generate a document.

**In scope:** `Tool` interface (name, description, input/output schema, risk level, timeout), tool registry, two fully working tools: `EnterpriseSearch` (already used by the specialist since Phase 5 — formalize it as a registered tool) and `DocxGenerator` (real downloadable .docx), sandbox adapter (`child_process`, limits, labeled `sandbox: adapter-mock`) behind a third tool `PythonSandbox`, `ToolCallCard` + `ArtifactCard` UI. `InspectionAnalysisSpecialist`'s allowed-tools list is updated to include `DocxGenerator`.

**Out of scope:** the other tools from the original list — registered as name+description only, `implemented: false`.

**Checkpoint:** "Prepare an approval note for the P-101 NDT report" is routed to `InspectionAnalysisSpecialist`, which calls `EnterpriseSearch` then `DocxGenerator`, producing a real downloadable `Approval_Note.docx` as an artifact card, with a `ToolCallCard` showing both tool calls.

**Prompt:**
```
Add a Tool Fabric to INDRA and connect it to the Inspection specialist.

Define a Tool interface: name, description, input schema, output schema, risk
level (LOW/MEDIUM/HIGH), timeout. Build a tool registry. Implement fully:
1. EnterpriseSearch — formalize the Phase 5 ContextManager/retrieval as a
   registered tool (already used by InspectionAnalysisSpecialist)
2. DocxGenerator — takes structured content and produces a real downloadable .docx
3. PythonSandbox — executes short snippets via child_process with a timeout and
   resource limits; label it in the UI as "Sandbox: adapter-mock (not production
   isolation)"

Register (but do NOT implement) as stubs with implemented:false: OCR, P&ID
Vision, Engineering Solver, File Operations, PPTX Generator, XLSX Generator.

Update InspectionAnalysisSpecialist's allowed-tools list to include
DocxGenerator. Wire tool execution into the AgentRuntime's EXECUTION state. Build
a ToolCallCard (tool name/status/duration) and an ArtifactCard (Open/Download)
for generated files.

Run and verify: "prepare an approval note for the P-101 NDT report" routes to
Inspection Analysis Specialist, calls EnterpriseSearch then DocxGenerator, and
produces a real downloadable docx shown as an artifact card with both tool calls
visible.
```

---

### **Phase 7 — Verification + Human Approval (real logic)**
**Goal:** Replace Phase 3's placeholder verification/approval with real risk-based logic, applied per-specialist.

**In scope:** verification pipeline (format check, evidence check — did the specialist's answer actually cite retrieved sources, policy check against classification), risk classification per operation (LOW auto — `GENERAL_CHAT`; MEDIUM configurable — `INSPECTION_ANALYSIS` DOCX generation; HIGH mandatory — reserved for when `EngineeringCalcSpecialist`/authoritative-data operations exist), real `ApprovalCard` blocking `DELIVERY` until acted on.

**Checkpoint:** the P-101 approval-note flow now stops at a real approval card; reject prevents delivery, approve releases it.

**Prompt:**
```
Replace the placeholder verification/approval logic from Phase 3 with real logic.

Verification (VERIFICATION state): check the specialist's response actually cites
retrieved sources (evidence check), check output format is well-formed, run a
basic policy check (classification of cited documents matches user's allowed
access). If verification fails, transition to FAILED and allow a re-plan/retry.

Approval (APPROVAL_REQUIRED state): classify by risk per operation — GENERAL_CHAT
= LOW (auto-pass), INSPECTION_ANALYSIS operations that produce a DOCX artifact =
MEDIUM (require approval by default). Build a real ApprovalCard UI (Operation /
Asset / Specialist / Risk / Evidence count, with Approve / Reject / Modify) that
blocks progression to DELIVERY until the user acts.

Run and verify: the P-101 approval note flow now stops at a real approval card
showing "Inspection Analysis Specialist" and risk MEDIUM; Reject prevents
delivery, Approve releases the artifact.
```

---

### **Phase 8 — Audit Trail**
**Goal:** Every run is fully reconstructable after the fact, including which specialist handled it.

**In scope:** `audit_events` table capturing user, timestamp, request, operation, **specialist selected**, context resolved, sources retrieved, model used, tool calls + I/O, verification result, approval decision, final artifact; `AuditTimeline` UI reachable from sidebar's "Audit"/"Runs".

**Checkpoint:** open Audit, find the P-101 run, see the full timeline including "Specialist: Inspection Analysis".

**Prompt:**
```
Add audit logging to INDRA.

Create an audit_events table recording, for every run: user, timestamp, original
request, operation classified, specialist selected, context/sources resolved,
model used, each tool call (name, input, output, duration), verification result,
approval decision, and final artifact reference.

Build an AuditTimeline UI component reachable from the sidebar's "Runs" and
"Audit" sections — list of past runs, click into one to see its full timeline.

Run and verify: after running the P-101 approval-note flow, open Audit, find
that run, and see every step including which specialist handled it.
```

---

### **Phase 9 — File Workspace + i18n (EN/TR) + Polish**
**Goal:** Round out remaining deferred items now the core loop is proven.

**In scope:** File Explorer (separate from Enterprise Memory), i18n (EN default + TR), friendly error states (model unavailable, tool timeout, permission denied, specialist not implemented, etc.), loading/empty states, RBAC skeleton (roles table + route guards, 2 roles: `engineer`, `admin`).

**Checkpoint:** switch UI language TR↔EN, upload a file, trigger a deliberate error and see the friendly state.

---

### **Phase 10 — Production/On-Prem Prep (documentation, not code)**
**Goal:** Document the real gap between MVP and sovereign production so nothing gets mistaken for production-ready.

**In scope:** README section listing what's mock/adapter vs. production-grade (sandbox isolation, local models, network telemetry, unimplemented specialists like P&ID/Engineering Calc), verify `ModelGateway`/`Specialist`/`Tool` interfaces have no Replit-specific coupling, config layer distinguishing "Local Model" vs "External Provider" in the UI.

**No new Replit Agent build prompt** — review pass + README, done by you.

---

## 4. What's deliberately cut for MVP, and why

| Original item | Status | Reason |
|---|---|---|
| PIDAnalysisSpecialist (real) | Registered stub only | Needs vision/OCR pipeline — real infra cost, no synthetic scanned docs to justify it yet |
| EngineeringCalcSpecialist (real) | Registered stub only | Needs a deterministic solver — separate subsystem, only build when a real calc is core to a demo |
| Vector / graph search | Deferred | Full-text search sells the demo with 4 synthetic docs |
| OCR / Vision pipeline | Stub only | No scanned documents in the synthetic demo set |
| 7 of 10 tools | Registered, not implemented | 3 working tools > 10 fake ones |
| Real zero-egress telemetry | Never on Replit | Can't be proven on a cloud dev platform — label honestly |
| Mobile redesign | Deferred past Phase 9 | Real 3-panel-to-mobile IA is its own project |
| Multi-approver workflows | Deferred | Single-approver HITL proves the pattern first |

---

## 5. Cross-device note

React frontend + responsive breakpoints (Phase 1) give laptop/desktop "universal" behavior for free — verified at the end of every phase. Not universal, and must stay explicitly labeled through every phase: the sandbox (adapter, not isolation) and model/specialist execution that depends on external infra (Vision/OCR for P&ID, a real solver for Engineering Calc, local model inference) — all server/infra-bound, all requiring real re-implementation on-prem. Keeping `ModelGateway`, `Specialist`, and `Tool` strictly interface-based from Phase 2/3 onward means that future work is "write a new adapter/specialist," not "rewrite the router."
