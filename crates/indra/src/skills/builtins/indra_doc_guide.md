---
name: indra-doc-guide
description: Reference official INDRA documentation when one is configured (INDRA_DOCS_ROOT) to create, configure, or explain INDRA-specific features like recipes, extensions, sessions, and providers. When no docs root is configured, answer from the repository's own README/AGENTS.md and source instead of guessing at syntax.
---

Use this skill when working with **INDRA-specific features**:
- Creating or editing recipes
- Configuring extensions or providers
- Explaining how INDRA features work
- Any INDRA configuration or setup task

Do NOT use this skill for:
- General coding tasks unrelated to INDRA
- Running existing recipes (just run them directly)

The docs root for this session is `{{INDRA_DOCS_ROOT}}`.

**If this value is empty or unset:** INDRA has no external docs site configured
for this installation. Do NOT fall back to any other agent's documentation
site under any circumstance — a different project's docs describe different
config syntax and will produce wrong output. Instead, answer from the
repository's own `README.md`, `AGENTS.md`, and the relevant source under
`crates/` (e.g. the `Specialist` trait and providers) that ship with this
checkout, and say plainly when something isn't documented rather than
guessing at field names or syntax.

**If this value is set** (a local filesystem path or an HTTP(S) URL), treat it
as `<docs-root>` and follow the steps below.

## Steps (COMPLETE ALL BEFORE RESPONDING, only when a docs root is configured)
1. **Read official docs**
   - Read the doc map from `<docs-root>/indra-docs-map.md`
   - Search the doc map for pages relevant to the user's topic and get the paths for these pages
   - Use the EXACT paths from the doc map. For example:
   - If doc map shows: `docs/guides/sessions/session-management.md`
   - Read: `<docs-root>/docs/guides/sessions/session-management.md`
   - Do NOT modify or guess paths.
   - **ONLY read paths that are explicitly listed in the doc map - do not guess or infer paths**
   - Read multiple docs in parallel and save to temp files
   - Use the temp files for subsequent searches instead of re-reading

2. **Create/modify content**
   - For INDRA configuration files:
      - Consult schema/field reference documentation first
      - **Search the docs to extract the complete schema for each element you plan to use**
      - Extract example snippets to understand usage patterns
      - Create your configuration based on reference specs, following example patterns
      - **⚠️ STOP: Before showing the user, verify output content MUST match the schema and reference in the configured docs root:**
         - [ ] Field names match exactly as shown in docs
         - [ ] Required fields/properties are present
         - [ ] Value formats match examples (YAML/JSON syntax, data types, etc.)
      - **If ANY verification fails, revise and repeat this step until ALL verifications pass**
      - **DO NOT present unverified output to the user**

3. **MANDATORY VERIFICATION - CHECK ALL THESE ITEMS BEFORE STEP 4**
   Before writing your final answer:
   - [ ] You MUST NOT rely on training data or assumptions for any INDRA-specific fields, values, names, syntax, or commands.
   - [ ] **Did you include "How to Use", CLI commands, or usage instructions?**
      - If YES and user didn't ask for it → **REMOVE IT NOW**
      - If YES and user asked for it → verify exact commands from the docs before including
   - [ ] List all INDRA-specific items in your answer (commands, fields, syntax, values, how to use, explanations, etc.)
   - [ ] For each item, verify it is correct according to the docs. If not found, either read the relevant docs NOW and verify, or remove it (if user asked for it, state "I could not find documentation for [X]").

4. **Provide your answer and include a "Verification Completed" section**
   - For EACH INDRA-specific item in your response, cite the specific doc file where you verified it

5. **List documentation links**
   - Only include docs actually used
   - Link to the configured docs root, even if you read the docs from a local path. Never expose local filesystem paths.
   - Remove `.md` suffix from URLs
