---
name: document-generation
description: How to generate real Word/Excel/PPT deliverables (not markdown renamed with an office extension) from agent findings — PSU/refinery approval-note structure, engineering-calculation presentation with steps/units/assumptions shown, and how to render .docx/.xlsx/.pptx files programmatically and fully offline in this Rust codebase. Use when an agent's findings need to become a real file a refinery engineer can open, or when reviewing/extending INDRA's document-rendering code.
---

## This codebase renders documents in Rust, not Python

There is no `python-docx`/`python-pptx`/`openpyxl` here — the equivalent, real, working code is:

- **`.docx`:** `docx_rs` crate. See `crates/indra-mcp/src/computercontroller/docx_tool.rs` (a general-purpose document editor: append/replace/insert-structured/add-image update modes, `DocxStyle` for bold/italic/underline/size/color/alignment mapped onto real `docx_rs::Run`/`Paragraph` constructs) and the narrower, purpose-built `crates/indra-mcp/src/sovereign/deliverable.rs::build_approval_note`, which is the actual approval-note renderer used by the OCR-to-note pipeline.
- **`.xlsx`:** `umya_spreadsheet` crate. See `crates/indra-mcp/src/computercontroller/xlsx_tool.rs` (`XlsxTool`, worksheet listing, cell/range read-write, `MAX_EXCEL_ROWS`/`MAX_EXCEL_COLUMNS`/`MAX_RANGE_CELLS` guards against runaway sizes).
- **`.pdf`:** `crates/indra-mcp/src/computercontroller/pdf_tool.rs` — check this file before assuming PDF generation doesn't exist.
- **`.pptx`:** no PPT-rendering code currently exists in the workspace (confirmed by checking `computercontroller/` and every `Cargo.toml` for a pptx-capable crate) — say this plainly if asked to build one, and evaluate/vendor a crate deliberately with `cargo add` per `AGENTS.md` rather than shelling out to an unvetted converter.

If asked for a Word/Excel deliverable, reach for these real modules first rather than inventing a new rendering path.

## The approval-note pattern (real, working, PSU-structured)

`crates/indra-mcp/src/sovereign/deliverable.rs::build_approval_note(findings: &[Finding], out: &Path)` is the concrete reference implementation:

```rust
pub struct Finding {
    pub text: String,
    pub severity: Severity,       // Normal | Monitor | CriticalActionRequired
    pub citation: Citation,       // document_id, quoted text, source boxes (page/x/y/w/h)
}
```

For each finding it writes a bold `[{severity_label}]` prefix paragraph followed by the finding text, then an italic "Source: {document_id} — p.{page} @ ({x},{y}); ..." paragraph listing every citation box. This is the pattern to follow for any structured-finding-to-document renderer: **severity/label, content, then a visible, non-optional source citation paragraph** — never a finding with no traceable citation.

A fuller PSU approval note (per the `approval-note-drafter` agent's structure, which this rendering step should implement, not reinterpret) has more sections than the current minimal renderer covers:

1. Header block — note number, date, originating department, subject, reference documents.
2. Body captions in fixed order — Background, Observation/Findings, Risk/Safety (if applicable), Recommendation, Financial Implication (if applicable), Approval Sought (a single unambiguous action sentence).
3. Routing/signature panel — Prepared by / Reviewed by / Recommended by / Approved by, each with a designation line.

When extending `build_approval_note` toward this fuller structure, keep the rendering step dumb: it takes already-drafted section text and structure and lays it out with real heading styles and a real table for the routing panel — it does not draft or infer content. Content drafting is a separate concern (see the `approval-note-drafter` agent).

## Engineering-calculation presentation

A calculation deliverable must show its work, not just a final number — render each step as a visible row or paragraph:

1. Assumption(s) stated explicitly.
2. Formula shown symbolically.
3. Substituted values with units.
4. Intermediate result(s), with units carried through.
5. Final result, with units, and (where relevant) the applicable code/standard reference.

For Excel, put each step on its own row with a units column, and prefer real spreadsheet formulas (`umya_spreadsheet` supports writing formula strings, e.g. via `CellValue.formula`) over pre-computed literals — a reviewing engineer should be able to click a cell and see the formula, not a frozen number. Never emit `#REF!` or a formula that fails to evaluate; validate by re-reading the generated file back (`XlsxTool::new` + read) before reporting success.

## Practical generation checklist

1. **Map structured content to native constructs**, not styled text: headings → real heading styles, tables → real table objects, spreadsheet numbers → real formulas — this is what keeps the output editable downstream in Word/Excel/LibreOffice.
2. **Never fabricate a citation, unit, or value.** Where the source content is incomplete or ambiguous, insert an explicit, visually flagged placeholder (`[VERIFY: source value missing]`) in the document itself rather than silently guessing or blocking generation outright.
3. **Generate locally, then validate.** Write the file, then run a lightweight local check that it opens/parses correctly (round-trip through the same crate that wrote it, e.g. re-open with `XlsxTool::new` or a `docx_rs` read) before reporting success — a nonzero exit code alone is not proof the file is valid. `crates/indra-mcp/tests/sovereign_approval_note.rs` is the reference pattern: it asserts the output file exists and has nonzero size, and asserts every finding carries at least one source box before considering the pipeline correct.
4. **Preserve traceability.** Carry source-agent, generation timestamp, and input-document references either in document metadata or a visible footer line, so a human reviewer can trace the deliverable back to its inputs.
5. **Revision discipline.** Regenerating an already-issued document should increment a revision/version marker and preserve the original naming convention, not overwrite the prior revision in place.
6. **Report the file, don't paste the document.** Return the output path, format, and a one-line content summary — never inline the full generated document body into a chat response when a file was the actual deliverable.

## Worked example

Findings from an inspection report (`Finding { text: "Wall thickness below minimum", severity: CriticalActionRequired, citation: Citation { document_id: "NDT_2026_08.pdf", text: "measured wall thickness 6.1 mm", boxes: [SourceBox { page: 1, x: 100, y: 220, w: 400, h: 24 }] } }`) go into `build_approval_note`, producing a `.docx` with a bold `[Critical Action Required] Wall thickness below minimum` paragraph followed by an italic `Source: NDT_2026_08.pdf — p.1 @ (100,220)` line — exactly the pattern exercised by `crates/indra-mcp/tests/sovereign_approval_note.rs`.
