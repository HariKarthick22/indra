---
name: offline-rag-retrieval
description: How to design and evaluate a fully offline retrieval-augmented-generation pipeline grounded in an organization's own manuals, SOPs, and correspondence — local embedding generation, local vector storage, chunking strategy for technical documents, citing source document plus page/location in answers, and verifying no network call occurs at any pipeline stage. Use when building document ingestion/retrieval for INDRA or reviewing whether a change risks ungrounded or uncited answers.
---

## What exists today, and what does not

**Real, load-bearing primitive:** `crates/indra-local-inference/src/embedding.rs` exposes `embed_text(model_path: &Path, text: &str) -> Result<Vec<f32>>`. It loads a GGUF embedding model (doc comment names `nomic-embed-text`-class models) via llama.cpp, running it with `LlamaPoolingType::Mean` pooling over a process-wide `OnceLock<LlamaBackend>` (llama.cpp's backend init is non-reentrant — it must be initialized exactly once per process). `EMBEDDING_DIM: usize = 768` is documented as advisory only; use the returned vector's actual length, not the constant, when a specific model's real dimension matters. Its only current caller is the smoke test `crates/indra-local-inference/tests/embedding_smoke.rs`.

**Does not exist yet:** chunking, document ingestion, vector storage/similarity search, retrieval-as-a-tool wiring into the agent loop, and citation formatting. There is no vector-store or PDF-handling dependency in any workspace `Cargo.toml`. Say this plainly when scoping work here — design and build it, don't hunt for it.

**Do not confuse this with Skills.** `crates/indra/src/skills/mod.rs` and `crates/indra/src/sources.rs` implement INDRA's Skills system — `SKILL.md`-based reusable *procedures* discovered from `.agents/skills/` directories. That teaches the agent how to do a task. A knowledge base grounds the agent's answers in the org's own *documents* (an SOP, a board deck, a correspondence thread). These are different mechanisms; a proposal to store SOPs as Skills is a category error — redirect it here instead.

**Taxonomy fit:** `crates/indra/src/sovereign/model_registry.rs`'s `Domain` enum already includes `Domain::Document`, and `IndraOperation::InspectionAnalysis` (`crates/indra/src/agents/specialists/operation.rs`) is classified under it — document-grounded retrieval has a natural home in the existing operation taxonomy (see the `multi-model-routing` skill) rather than needing an invented classification.

## Building the pipeline

1. **Chunk with document structure, not blind windows.** Technical/engineering documents (SOPs, manuals, inspection procedures) have real structure — numbered sections, subsections, tables, figure captions. Split on section/paragraph boundaries first, fall back to a token-count window only within an oversized section. Every chunk must carry metadata sufficient to cite later: source document id, section/heading, and page number (or, for a scanned document, the page and bounding box — see the `scanned-document-ocr` skill's `SourceBox`/OCR-line format, which already produces exactly this shape via `crates/indra-mcp/src/sovereign/ingest.rs::ocr_page`).

2. **Embed every chunk through `embed_text`, never a second path.** Do not call an external embedding API and do not hand-roll a second local embedding routine — route everything through the existing primitive so there is one place that enforces "local model only."

3. **Choose local vector storage deliberately.** No vector-store crate exists in the workspace yet. For a realistic single-refinery manual/SOP corpus, brute-force cosine similarity over an in-memory or on-disk array of `Vec<f32>` may be sufficient and simpler to audit than an ANN index — justify whichever choice is made against the actual expected corpus size rather than defaulting to the first library found. Add any new dependency with `cargo add` per `AGENTS.md`.

4. **Expose retrieval as a callable tool, not a session-start step.** Follow the platform-extension pattern under `crates/indra/src/agents/platform_extensions/` (e.g. `todo.rs`, `summon.rs` for the `InitializeResult`/`ServerCapabilities`/tool-schema shape) so the agent can call retrieval mid-task, through the existing tool-calling operation (`crates/indra/src/agents/state_machine/ops_toolcalling.rs`). A knowledge base only consulted once at the start of a session does not meet "grounds itself... via a local knowledge base" for genuinely multi-step agentic work.

5. **Cite source document and location in every grounded answer.** A retrieved chunk must carry its `document_id` and page/section back through generation so a downstream approval note or calculation can say "per SOP-114 §3.2" rather than presenting a retrieved fact as unattributed model knowledge. `crates/indra-mcp/src/sovereign/provenance.rs`'s `Citation { document_id, text, boxes: Vec<SourceBox> }` is the existing citation shape used by the approval-note pipeline (`crates/indra-mcp/src/sovereign/deliverable.rs::build_approval_note`) — reuse this structure for text-document retrieval too rather than inventing a second citation format.

6. **Handle staleness.** A revised SOP replacing an older one must invalidate or supersede the old chunks, not just accumulate alongside them — citing a withdrawn procedure in a refinery safety context is worse than citing nothing.

## Verifying no network call at any stage

Every stage — model loading, embedding, similarity search, retrieval — must be checkable as local-only:

- **Ingestion/embedding:** confirm the embedding model path is a local file, never a URL or a "download if missing" fallback triggered at query time. Mirror the same hard-rejection pattern `crates/indra-local-inference/src/multimodal.rs` uses for `extract_images_from_messages` (which explicitly rejects `http://`/`https://` image URLs with an error naming that remote URLs aren't supported for local inference) — make "local file only" a hard constraint in the ingestion tool's interface, not a convention that can be silently bypassed.
- **Similarity search/reranking:** confirm no HTTP client is constructed anywhere on this path; a brute-force or on-disk index has no legitimate reason to open a socket.
- **Live verification:** use the `air-gap-verification` skill's `EgressLog`/`trigger_probe` mechanism (`crates/indra/src/security/egress_inspector.rs`, exercised in `crates/indra/tests/sovereign_egress_proof.rs`) to confirm zero recorded outbound attempts during an ingestion-and-query run.
- **Failure mode:** when the configured embedding model is missing, ingestion and retrieval must fail with a clear, actionable error naming the missing model/path — never silently skip grounding and let the model answer ungrounded while looking equally confident.

## Worked example

A user asks "what's the minimum wall thickness for the Unit 3 cooling water line per our SOP?" The retrieval tool embeds the query with `embed_text`, does cosine similarity against pre-embedded SOP chunks, returns the top-k chunks each carrying `document_id: "SOP-114"` and a page/section, and the agent's answer cites "per SOP-114 §3.2 (p.4)" rather than stating the figure as background knowledge. If no embedding model is configured, the tool returns an error naming the missing model rather than letting the agent answer from training data while looking grounded.
