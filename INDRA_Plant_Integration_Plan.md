# INDRA Plant Integration — Build Plan

**Making the agent fast on local hardware, self-selecting about which model runs, connected to four live MRPL data systems, and able to survive files far larger than its own context window.**

Status: plan complete, nothing implemented. Every claim below is traced to a file and line from a read-only audit — **none of it is compile-verified**, because the workspace does not currently build (see Blocker 1) and the machine has no disk headroom (Blocker 2).

---

## Decisions locked

| Question | Decision |
|---|---|
| Codebase | The Rust fork, `crates/indra` — not the Node/React MVP in `INDRA_Full_Plan.md` |
| Deployment | On-prem LAN, air-gapped |
| Model routing | Tiered by task complexity |
| MRPL data sources | All four: SMB share, SharePoint/DMS, SAP PM, OSIsoft PI |
| Data scale | All four: GB-scale doc sets, TB-scale archives, individual multi-GB files, mixed |

This plan assumes and builds on the previously approved **Cortex memory-graph plan**. Entity nodes, scopes, and provenance originate there.

---

## Three blockers before any code

### 1. The workspace does not compile

The module directory was renamed to `src/indra_apps/`, but two `include_str!` calls still point at the old path:

- `crates/indra/src/indra_apps/cache.rs:10` → `include_str!("../goose_apps/clock.html")`
- `crates/indra/src/agents/platform_extensions/apps.rs:59` → `include_str!("../../goose_apps/clock.html")`

The file exists only at `crates/indra/src/indra_apps/clock.html`. **Two one-line fixes. Nothing in the workspace builds until they land.**

### 2. The machine is full — not just the repo

`/System/Volumes/Data` reports **200 GiB used of 228 GiB, ~937 MiB free, 100% capacity**, and drained several GB during a single working session. No Time Machine local snapshots exist to purge. A debug build of 1,344 dependencies needs 8–20 GB.

Safe to reclaim now:

| Target | Size | Notes |
|---|---|---|
| `~/Library/Caches` | 2.6 GB | Safe |
| `target/` via `cargo clean` | 2.3 GB | Safe, rebuilds |
| `~/Library/Application Support/com.docker.install` | 2.1 GB | Installer leftovers |
| `~/.rustup` | 1.8 GB | **Keep** — needs network to restore, matters for an air-gapped target |

**Do not delete `ui/node_modules` yet.** It is 2.2 GB and looks like easy space, but `ui/pnpm-workspace.yaml:2-5` and `ui/package.json:3-8` still list the pre-rename globs `goose-acp`, `goose-acp-client`, `goose-binary/*` while the directories are now `ui/indra-*`. `pnpm install` would fail to resolve the `workspace:*` dependencies and **you could not restore it**. Fix the globs first.

### 3. The rename's callers never caught up

The Cargo manifests are **clean** — every crate directory matches its package name, and `Cargo.lock` is already consistent. The breakage is everything that calls them:

- Root `Justfile` — 16 stale lines (`-p goose-cli --bin goose`, `crates/goose/acp-schema.json`, `ui/goose-acp-client`, …)
- `Dockerfile:34` — `cargo build --release --package goose-cli`
- `flake.nix:104,138,139`
- All four `scripts/*.sh` — `cargo build --bin goose` (the binary is now `indra`)
- 29 GitHub workflow files; `ci.yml` alone has 12 broken invocations
- `crates/indra-sdk/justfile` — every recipe (20 lines)

**Two silent regressions:**

- `.indrahints` was renamed on disk, but `crates/indra/src/hints/load_hints.rs:10` still only looks for `.goosehints`. **The project's own hints file is never loaded.** Same stale literal in `ui/desktop/src/desktopFileAccess.ts` (8 sites).
- `crates/indra/src/skills/builtins/indra_doc_guide.md:3` tells users to set `INDRA_DOCS_ROOT`, while `crates/indra/src/config/base.rs:1263` reads `GOOSE_DOCS_ROOT`. Setting the documented variable does nothing.

**`AGENTS.md` carries 12 stale paths** and should be corrected early, since it misdirects every future agent session:

| Line | Stale | Correct |
|---|---|---|
| 30 | `crates/goose/src/agents/agent.rs` | `crates/indra/...` |
| 30 | `crates/goose/src/agents/state_machine/` | `crates/indra/...` |
| 30 | `GOOSE_STATE_MACHINE=1` | ✅ **correct as-is — do not change** |
| 52 | `cargo test -p goose` | `-p indra` |
| 53 | `cargo test --package goose --test mcp_integration_test` | `--package indra` |
| 93 | `crates/goose/tests/` | `crates/indra/tests/` |
| 94 | `goose-self-test.yaml`, `goose run --recipe` | `indra-self-test.yaml`, `indra run` |
| 97 | `crates/goose-mcp/` | `crates/indra-mcp/` |
| 122 | `crates/goose-cli/src/main.rs` | `crates/indra-cli/...` |
| 124 | `crates/goose/src/agents/agent.rs` | `crates/indra/...` |

All ~160 environment variables are still `GOOSE_*`; no migration has begun. `GOOSE_STATE_MACHINE` is genuinely the live name.

**Leave alone (intentional back-compat):** `config/paths.rs:20-27` (`Block`/`goose` app dirs — changing orphans existing installs), `config/base.rs:31` (`KEYRING_SERVICE = "goose"` — orphans stored secrets), `/etc/goose/config.yaml` paths, outbound API identity headers registered with third parties, the `NOTICE` file.

---

## Part A — Making the local model fast

Almost every performance knob already exists in `ModelSettings` (`crates/indra-local-inference/src/model.rs:56`): `context_size`, `n_batch`, `n_gpu_layers`, `n_threads`, `use_mlock`, `flash_attention`, `draft_model`. **Every one defaults to `None` or `false`.** `config_resolver.rs` looks like a tuner but is only a plumbing indirection — it reads settings, it never derives them.

Hardware probing barely exists. The only real probe is llama.cpp's device enumerator at `llamacpp/mod.rs:601`, and it reads only `memory_free` — `memory_total` is logged at `:643` and never used. There is no `sysinfo`, `num_cpus`, or `available_parallelism` call anywhere in `crates/`. Physical core count, total RAM, and GPU vendor are invisible to the system.

### Correction: speculative decoding is not a wiring job

An earlier read suggested the draft-model plumbing merely needed connecting. That was wrong, and the difference is weeks of work.

`llama-cpp-2` 0.1.146 exposes **no draft API at all**. llama.cpp's `common/speculative.cpp` *is* vendored, but it is excluded by the bindgen allowlist in `build.rs:275`, which admits only `llama_*`, `ggml_*`, `gguf_*`, and `mtmd_*` symbols. Reaching it needs a new C shim, because the C++ signatures use `std::vector`.

The MLX side is also narrower than it looked: `mlx.rs:247` gates on `Model::Gemma4(_)` and is MTP-specific. Any other MLX model with a draft configured falls through silently to single-model generation. **"Parity with MLX" means parity with one model family.**

### The work

| # | Work | Why it pays | State |
|---|---|---|---|
| **A4** | **Fix the cache key first.** `ModelCacheKey::new(backend_id, model_name, chat_template)` (`lib.rs:808`) omits every tuning field. | Re-tuning `n_gpu_layers`, `context_size`, `n_batch`, or `use_mlock` will **not** evict a loaded model — new values silently do nothing. This must land first or A1 and A2 will appear to fail. | **Blocker** |
| **A1** | **Call the auto-tuner that already exists.** `LlamaModelParams::fit_params` (`llama-cpp-2 src/model/params.rs:326`) wraps `llama_params_fit` and selects `n_gpu_layers`, `tensor_split`, and `n_ctx` from available VRAM. | The hardest part of A1 is a dependency call we never make. **Precondition trap:** it requires `n_gpu_layers` to still be `-1`, but `llamacpp/mod.rs:438` sets it first, which disables the fit. | Available |
| **A2** | **Quantize the KV cache.** `with_type_k` / `with_type_v` (`context/params/get_set.rs:493,522`) accept Q8_0, Q4_0, Q5_1. | Highest-leverage unused knob — halves or quarters KV footprint directly. Note `estimate_max_context_for_memory` hardcodes 2 bytes/element at `inference_engine.rs:158`, so that estimate is wrong the moment this is on. | Unused |
| **A3** | **Speculative decoding, hand-rolled.** Draft/verify loop from primitives: `LlamaBatch::add`, `decode`, `candidates_ith`, sampler `accept`, `clear_kv_cache_seq` for rejected drafts. Two models can share one backend. | Still 2–3× on accepted drafts, but this is a real subsystem, not a connection. **Defer until A1/A2 are measured.** | Large |

### Apple Silicon caveat

`MlxBackend::load_model` takes `_settings: &ModelSettings` (`mlx.rs:148`) and **never reads it**. MLX ignores every performance field — `n_gpu_layers`, `n_batch`, `n_threads`, `flash_attention`, `use_mlock` — and its `available_memory_bytes()` returns a hardcoded `0` (`mlx.rs:367`). It also drops `top_k`, `top_p`, `min_p`, and all four penalties.

**Part A buys almost nothing on Mac.** It is work for the Linux plant server, which is where it matters anyway.

### Verification

A benchmark harness recording tokens/sec and time-to-first-token per configuration, run on the target plant hardware — not a laptop. A4 first, then A1 and A2 each must show a measured gain or be reverted.

---

## Part B — Automatic model selection

Nothing routes models today. There is a crash-fallback at `agent.rs:3936` and a TODO at `orchestrator.rs:87`.

**But the classifier already exists.** `crates/indra/src/agents/specialists/router.rs` already classifies every incoming prompt into an `IndraOperation`, and the registry already picks a specialist. Model selection is a second read of a decision the system is *already making* — not a new classifier.

### Design

- Add `ModelTier { Fast, Balanced, Deep }`.
- Add `preferred_tier()` to the existing `Specialist` trait (`specialists/specialist.rs`). The tier concept is already named in the original INDRA spec; it was simply never given a type.
- **Fast** handles routing, classification, summarisation, single-file edits. **Deep** handles inspection analysis and multi-step reasoning.
- **Escalation triggers**, so a wrong guess degrades instead of failing: tool-call parse failure, low token confidence, or context overflow promotes the turn one tier and retries once.

### Parity requirement

`AGENTS.md` requires agent-loop changes to land in **both** the legacy loop (`agents/agent.rs`) and the state machine (`agents/state_machine/`) until the migration completes. Tier selection is agent-loop behaviour. The existing specialist wiring is the template — already duplicated across `agent.rs:2415` and `ops_specialist.rs`.

---

## Part C — Reaching MRPL's real data

Nothing in the repo connects to MRPL today; the name appears only as synthetic demo data in `INDRA_Full_Plan.md`. All four connectors are new, behind one `DataSource` trait: `enumerate()`, `fetch()`, `watch()`, `describe()`.

**The four sources are not four variations of one problem.** Two hold documents; two hold structured records. Treating them alike is the main design trap.

| Source | Shape | Retrieval strategy |
|---|---|---|
| **SMB / file share** | PDF, DOCX, XLSX on a mounted path | **Ingest**: extract, chunk, index. The baseline case. |
| **SharePoint / DMS** | Documents with their own metadata and ACLs | **Ingest**, but *mirror the existing permissions* into scopes rather than inventing new ones. Re-deriving ACLs is how data leaks. |
| **SAP PM** | Work orders, equipment master, notifications | **Query adapter, not ingestion.** Structured relational data maps to entities; chunking destroys the schema that makes it useful. |
| **OSIsoft PI** | Time-series tags — pressure, vibration, flow | **Query adapter.** Retrieval is tag + time range, not text similarity. A historian is never chunked; it is asked. |

### The part that makes it feel automatic

An **entity resolver** sits in front of all four. The user says "P-101"; the resolver identifies the equipment tag, then fans out — PI for current vibration, SAP for open work orders, SMB and SharePoint for inspection history. One question, four systems, one cited answer.

This is what turns four connectors into a single assistant, and it depends on the entity nodes from the Cortex plan.

---

## Part D — Large files, the way Meta solved them

### Correction: the famous bug is dead code

`developer/edit.rs:54` does read whole files before truncating — but `DeveloperClient::get_tools()` (`developer/mod.rs:108-180`) exposes only `write`, `edit`, `shell`, `tree`, and `read_image`. `file_read_with_cwd` has **no non-test callers**. The live read tool is `acp/fs.rs:145`, which already does a proper ranged read.

Fixing `:54` is worth doing as the reference implementation, but **it is not the leak.** The leaks are elsewhere, and one is strictly worse.

### What is actually live

| Site | Problem | Fix |
|---|---|---|
| **`developer/tree.rs:208`**<br>`count_file_lines`, from `collect_tree:182` | Reads **every file in the walk** whole, just to count newlines. Highest fan-out in the codebase — one `tree` call on a repo holding one multi-GB log reads that log entirely. No size gate. | Metadata gate, or count `\n` over a `BufReader` without building a `String`. ~8 lines. |
| **`developer/edit.rs:118`**<br>`file_edit_with_cwd`, from `developer/mod.rs:233` | Live, and has **no limit at all**. Whole read, then `string_replace` allocates a second full copy — peak ≈ 2× file size. Strictly worse than `:54`, which at least truncates. On a failed match it then runs `count_lines_before:227` (full `char_indices` scan) and `build_file_preview:259` (another full `lines()` split). | Gate on `metadata().len()` before reading; return "file too large to edit, use shell". |
| `acp/fs.rs:136` | Identical shape, live edit path, used by `acp_edit:216`. | Same change, same commit. |
| `developer/shell.rs:587,655` | Output streams into an **unbounded** mpsc, drains to a `Vec`, builds three full strings in `interleave:770-794`, *then* applies `OUTPUT_LIMIT_BYTES`. `cat huge.log` holds it 3–4× in RAM. | Move the existing ceiling upstream into `collect_tagged_lines:797`; stream past the limit to `save_full_output:938`. |
| `session/diagnostics.rs:252,259` | `read_tail` / `read_capped` read whole files then take the tail. Targets are CLI and **LLM logs** — typically the largest files on the machine. `read_capped` also does a full `chars()` walk twice. | Rewrite over `BufReader`. Two small functions, five call sites. |
| `analyze/mod.rs:129` | No size gate in all of `analyze/`; `SIZE_LIMIT` (`format.rs:8`) caps only printed output, after parsing. Tree-sitter's AST is several× source size. | Metadata gate. |
| `acp/server.rs:675` | `read_resource_link` reads a **client-supplied `file://` URI** whole and embeds it into the prompt. | Cap. Externally influenced — hardening, not performance. |
| `agents/extension_manager.rs:458` | Drains MCP subprocess stderr unbounded for the life of the connection. Hostile or chatty server grows it without limit. | `.take(N)`. |

Lower priority, all one-liners: `hints/import_files.rs:514` (route through the `ExpansionBudget` the nested path at `:398` already respects), `commands/session.rs:349` (copy the `session/legacy.rs:41-56` gate verbatim), `apps.rs:207` (apply the existing `GUEST_HTML_MAX_BYTES`), `read_recipe_file_content.rs:69` (route through `read_source_file` as `:29` already does), `review/handler.rs:742,869`, and the `computercontroller` DOCX/PDF tools (container formats — gate on decompressed size too).

### The pattern worth internalising

> Every limit enforced **before** the read sits next to a `metadata()` call or a `.take()`.
> Every limit enforced **after** sits downstream of a bare `read_to_string`.

That single distinction separates the safe readers from the dangerous ones, and it is the cheapest review heuristic for this work.

**Reuse, don't reinvent:** `image.rs:219` (stat first, then bounded read — exactly the right shape), `skills/supporting_files.rs:111` (`read_utf8_with_limit`), `session/legacy.rs:41` (the workspace's only true streaming line reader). `summarize.rs:364` is **already correct** — it gates on `metadata.len()` before reading, so it needs no work despite looking like a suspect.

**Do not lean on `large_response_handler.rs`.** It fires only once the full string exists (`agent.rs:720`, `ops_toolcalling.rs:257`), and its own `chars().count()` double-scan (`:26`, `:33`) plus temp-file copy add cost on precisely the inputs already hurting. Last line of defence, not a guard.

### The ranged reader is smaller than it looked

There is no `seek`, no `mmap`, and no line-offset index anywhere in `crates/`, which made this look like the one piece of genuinely new machinery. It mostly isn't.

**A persistent index is not needed for correctness.** A single `BufReader` pass that discards lines before `start` and stops at `start + limit` gives constant memory immediately, with zero new data structures.

Three functions, one module:

- `read_range(path, line, limit)` — the `BufReader` pass, matching the `acp_read_text_file` contract (`acp/fs.rs:29`) so local and ACP readers stay one implementation.
- `count_lines(path)` — `BufReader` over 64 KiB chunks counting `\n`.
- `read_bounded(path, max)` — should **consolidate** the three near-identical copies already at `image.rs:231`, `images.rs:205`, and `schedule_tool.rs:35` rather than becoming a fourth.

Build the `BTreeMap<line, byte_offset>` index only if repeated reads of the same large file show up as measured cost; it slots in behind `read_range`'s signature later without touching a single caller.

### The Meta techniques

| Technique | What it is | Applied to INDRA |
|---|---|---|
| **EdenFS** | Virtual filesystem that materialises content on demand, by range, never whole-file | The ranged reader above. Reading line 2,000,000 of a 40 GB log becomes a seek, not a load. |
| **Haystack** | Packs many small objects into few large append-only blobs with an offset index, avoiding per-file inode and seek overhead | Millions of document chunks become a handful of pack files plus `(pack_id, offset, len)` rows. Solves the small-file problem at TB scale. |
| **Content-defined chunking** | Rolling-hash boundaries, so inserting a paragraph shifts one chunk instead of all of them; the content hash is the identity | Deduplicates Rev A/B/C of the same SOP. Refinery document revisions overlap enormously — likely the single biggest storage win. |
| **Zstandard** | Meta's compressor, with *trained dictionaries* for small records | A dictionary trained per document type. Standard-form engineering text compresses far better with a dictionary. |
| **Watchman** | Persistent file-watching with a cookie-based change list | Incremental ingest against a watermark. Re-scanning a plant archive nightly is not viable; re-scanning what changed is. |
| **FAISS** | Meta's vector index (IVF/PQ) for large-scale ANN search | Tier-3 retrieval, **only once FTS5 measurably stops being enough**. Adding it earlier buys complexity, not recall. |
| **RocksDB** | LSM-tree store tuned for write-heavy workloads | **Hold.** Do not adopt unless SQLite write contention is measured as a real bottleneck. |

The through-line: Meta's large-data work is mostly about **not reading what you do not need** — by range, by hash, by watermark. Applied here, that principle is worth more than any individual library.

---

## Build order

Ordered by dependency, not preference.

| # | Phase | Depends on |
|---|---|---|
| **00** | **Make it compile, then make room.** Two `include_str!` paths; pnpm workspace globs; free disk; correct `AGENTS.md`, root `Justfile`, `Dockerfile`, `flake.nix`; restore `.indrahints` loading. | Blocks everything. Mostly one-line fixes. |
| **01** | **Stop the live whole-file reads.** Metadata gates on `tree.rs:208`, `edit.rs:118`, `acp/fs.rs:136`; move the shell ceiling upstream; stream `diagnostics.rs`. Reuse the `image.rs:219` pattern. | Standalone. Small diffs, immediate effect. |
| **01b** | **The ranged reader.** Three-function module above. | Needs 01. Pays off when a `read` tool is exposed. |
| **01c** | **Untrusted-input gates.** `acp/server.rs:675`, `extension_manager.rs:458`. | Independent. One-line `.take()` gates. |
| **02** | **Local speed (Part A).** A4 cache key first, then `fit_params`, KV quantization. | Independent. Makes every later phase pleasant to test. |
| **03** | **Tiered model routing (Part B).** Both agent loops. | Needs 02 — residency prevents reload stalls. |
| **04** | **Storage stack (Part D).** Chunking, packing, compression, watermarks. | Needs the Cortex schema. |
| **05** | **Connectors (Part C).** `DataSource` trait, then SMB first. | Needs 04 for documents, scopes for permissions. |
| **06** | **Entity resolver.** Tag resolution and cross-source fan-out. | Needs all four connectors. |

### Parallelisation

Part D splits into six file-disjoint domains, safe to fan out in one round:

1. `developer/edit.rs` + `acp/fs.rs` + the new reader module — **must land first**
2. `developer/tree.rs`
3. `session/diagnostics.rs` + `doctor.rs`
4. `developer/shell.rs` + `shell_output_streaming.rs`
5. `hints/import_files.rs` + `agents/extension_manager.rs` + `acp/server.rs`
6. `indra-mcp/src/computercontroller/*`

Domains 2–6 are stat-gate work with no dependency on domain 1.

---

## Where the risk actually sits

**SharePoint ACL mirroring** is the highest-consequence item in this plan. Getting model routing wrong produces a slow or mediocre answer. Getting permission mirroring wrong shows a contractor somebody's disciplinary file. Build it test-first, with the denial cases written before the access cases, and fail closed — an unresolved principal sees nothing, never everything.

**Four connectors is genuinely four projects.** The `DataSource` trait makes them tractable, not simultaneous. SMB first is not a hedge — it is the one that runs fully air-gapped and validates the abstraction before the harder three commit to it.

---

## What enterprises actually expect

| Expectation | Where it lands |
|---|---|
| Data never leaves the premises — provably | `INDRA_SOVEREIGN=1`, local inference, egress inspector, packet-capture verification |
| Every answer traceable to a source | Cortex `cortex_observations` provenance |
| People see only what their role permits | `cortex_grants`, fail-closed filtering |
| Revocation is immediate | `TrustBook` re-read per connection, already fail-closed |
| Institutional memory survives staff turnover | The graph outlives sessions and the people who made them |
| Auditability of agent actions | Partly present (`permission/`, `security/`) — full trail **not in this plan** |
| Predictable cost | Local models, no per-token vendor billing |

---

## Not in this plan

Stated so they are not discovered late:

- **A full audit trail** of every agent run.
- **Live multi-person co-presence** in one session. The `goose-roaming` README defers this explicitly — it is not expressible over 1:1 ACP and needs a purpose-built multi-party protocol (subscribe / snapshot / broadcast / steer with an explicit controller).
- **OCR or vision for scanned P&ID drawings.** The `PidAnalysisSpecialist` stub still correctly reports this as unavailable.

---

*Findings sourced from three read-only audits of the tree at `crates/indra*`. No `cargo` command was run; nothing here is compile-verified.*
