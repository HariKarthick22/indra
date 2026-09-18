---
name: scanned-document-ocr
description: How to process scanned/handwritten industrial documents (inspection reports, P&IDs, engineering drawings, photographs) fully on-device — OCR pipeline stages, handling low-quality scans and handwriting, extracting structured data (symbols, tags, line numbers) from a P&ID, and confidence-scoring extractions so low-confidence text is flagged for human review instead of silently guessed. Use when building or reviewing INDRA's document-extraction pipeline, or when a specialist needs OCR'd text with provenance.
---

## What exists today, grounded in real code

A working OCR pipeline already exists in `crates/indra-mcp/src/sovereign/`:

- **`ingest.rs::ocr_page(image: &Path) -> Result<Vec<OcrLine>>`** shells out to the local `tesseract` binary (`tesseract <image> stdout tsv`) and parses its TSV output. Each `OcrLine` carries `text: String`, `bbox: SourceBox`, and `confidence: f32` — confidence comes straight from Tesseract's own per-word confidence column, minimized across the words merged into a line (`last.confidence.min(conf)`), so a line's reported confidence is never higher than its weakest word.
- **`provenance.rs`** defines the shared location/citation types: `SourceBox { page, x, y, w, h }` and `Citation { document_id, text, boxes: Vec<SourceBox> }`.
- **`deliverable.rs::build_approval_note`** consumes `Finding { text, severity, citation }` and renders a `.docx` where every finding's paragraph is followed by an italic "Source: {document_id} — p.{page} @ ({x},{y})" line — so citation survives all the way into the generated deliverable, not just the extraction step.
- **`computercontroller/inspection_tool.rs`** wires this into two MCP tools: `ocr_extract_lines` (calls `ocr_page`, returns numbered lines each showing page/x/y/w/h/text) and `generate_inspection_approval_note` (builds the `.docx`, rejecting an empty findings list). Tool names and wiring are in `crates/indra-mcp/src/computercontroller/mod.rs` (search `ocr_extract_lines`, `generate_inspection_approval_note`).
- **Test fixture and coverage:** `crates/indra-mcp/tests/fixtures/sample_inspection.png`, exercised by `crates/indra-mcp/tests/sovereign_ingest.rs` (asserts every returned line has a non-degenerate bounding box and non-empty text) and `crates/indra-mcp/tests/sovereign_approval_note.rs` (asserts a real nonzero-size `.docx` is produced and every finding carries at least one source box).
- **Live verification script:** `indra-self-test.yaml`'s Phase 3F ("OCR-Grounded Approval Note", A2/A4) is the judge-facing script — run `ocr_extract_lines` on the fixture, build a note using only text it actually returned, confirm a nonzero `.docx` exists.

**What does not exist yet, say so plainly:** PDF rasterization (multi-page scanned PDF → per-page images) has no dependency or code anywhere in the workspace — confirmed by checking every `Cargo.toml`. There is no dedicated handwriting-recognition pass distinct from Tesseract's own (weak) handwriting support. There is no P&ID symbol classifier (valve types, instrument bubbles, line-type classification) — today's pipeline extracts text lines with positions, not vector/symbol structure. A vision-language model with a `mmproj_path`-paired GGUF checkpoint (per `crates/indra/src/sovereign/model_registry.rs`'s `vision: bool` capability flag) is the intended second half of the pipeline for genuine diagram/photo understanding, but the code wiring that text extraction and vision-model description together into one structured output does not exist yet.

## Pipeline stages to design or extend

1. **Rasterize.** For a scanned PDF, produce one image per page at a DPI high enough for Tesseract and a vision model to read small text (test at 300 DPI as a baseline; P&ID tag text is often smaller than body text and needs headroom). This stage does not exist yet — build it deliberately with a vetted crate (`cargo add`), don't shell out to an unvetted external binary beyond Tesseract itself.
2. **OCR pass.** Reuse `ocr_page` per rasterized page rather than inventing a second OCR call path — it already returns line-level text, bounding box, and confidence.
3. **Vision-model pass (for diagrams/photos).** Feed the same page image to a vision-capable local model (routed via `select_model` with `Requirements { vision: true, .. }` — see the `multi-model-routing` skill) for diagram/photo description that OCR alone can't produce (symbol shapes, layout relationships). Route through `crates/indra-local-inference/src/multimodal.rs`'s existing image-extraction path, which already hard-rejects `http(s)://` image URLs — never add a code path that fetches an image by URL instead of local bytes.
4. **Merge into one structured artifact** per page/document: OCR lines with confidence and position, plus vision-model description, plus (for a P&ID) an inventory of tags/line numbers/symbols once that extraction exists. Keep OCR'd text and vision-model narrative distinguishable in the output — don't merge them into one undifferentiated blob a downstream specialist might over-trust.

## Confidence scoring and human review

- Treat Tesseract's per-line confidence (already returned by `ocr_page`) as the floor signal for "is this text trustworthy." Pick and document an explicit threshold (e.g. below some confidence, mark `Needs-Human-Review`) rather than passing every line through unfiltered.
- **Never let a low-confidence or garbled extraction get silently guessed into a clean-looking value.** If a tag number reads as `P-1O1` (letter O vs digit 0 ambiguity) at low confidence, surface the raw OCR text and its bounding box for a human to resolve against the physical document — do not normalize it to `P-101` on the model's own initiative.
- For a P&ID review workflow specifically, classify every finding by severity including a dedicated `Needs-Human-Review` bucket (mirroring the review structure the `pid-drawing-reviewer` agent already uses) for garbled tag text, illegible handwritten markup, or a symbol with no confident classification — and never mark a document "clear" while any `Needs-Human-Review` item remains open.
- When no vision-capable model is registered at all, the pipeline must fail loudly (mirroring `select_model`'s existing hard-disqualification for a missing capability) — never silently fall back to text-only extraction that looks confident but is blind to diagram content.

## Handling low-quality scans and handwriting

- A page with mixed print and handwriting (a printed form with a handwritten margin note) should produce two distinguishable confidence bands in the output, not one merged text blob — Tesseract's OCR confidence is a reasonable proxy for "this is probably print," and a separate low-confidence/handwriting flag should be attached rather than treating handwritten regions as equally reliable as printed text.
- For dense small-scale content (P&ID instrument bubbles, tag text) that a general vision model reads unreliably at native resolution, crop and feed higher-resolution tiles rather than the whole page at once — `crates/indra/src/agents/platform_extensions/developer/image.rs`'s `ImageTool`/`CropParams` already provides this zoom mechanism for the agent to reuse.
- Rotated or skewed pages should be corrected (deskew) before the OCR pass where possible; where correction isn't confident, lower reported confidence rather than passing skewed-page OCR output through at face value.

## Worked example

Given `sample_inspection.png`: call `ocr_extract_lines`, get back numbered lines each showing page/x/y/w/h/text. A line reading "Wall thickness: 6.1 mm" at high confidence becomes a citable finding. A line reading "P-1?1A" at low confidence gets flagged `Needs-Human-Review` with its raw text and bounding box rather than resolved to a guessed tag number — a human confirms it against the physical report before `generate_inspection_approval_note` cites it.
