---
name: multi-model-routing
description: How to design, extend, and review a task-classifier plus capability-matching router for a multi-open-weight-model backend — classifying a request's task type, matching it to a model's declared capabilities, handling fallback when the preferred model isn't loaded or hardware is tight, and onboarding a new model without touching selection logic. Use when adding a model to INDRA's sovereign registry, editing the operation classifier, or diagnosing a misrouted task.
---

## Where this lives in INDRA

The routing logic is real, working Rust, not a design placeholder:

- `crates/indra/src/sovereign/model_registry.rs` — `ModelDescriptor`, `Capabilities`, `Requirements`, `Fit`, `select_model()`, `validate_backend()`.
- `crates/indra/src/agents/specialists/operation.rs` — `IndraOperation` (the task-type enum) and its `requirements()` method.
- `crates/indra/src/agents/specialists/router.rs` — `OperationRouter::classify()`, a keyword-based prompt classifier.

The two concerns are deliberately split: `router.rs` decides *what kind of task this is*, `model_registry.rs` decides *which model can do it*. Never let one leak into the other — the classifier should never know about specific models, and `select_model` should never know about prompt text.

## The routing pipeline, step by step

1. **Classify the task.** `OperationRouter::classify(prompt)` lower-cases the prompt and checks keyword sets in a fixed priority order (inspection → P&ID → engineering calculation → code generation → default `GeneralChat`). Order matters: a prompt containing both "calculate" and "inspection" resolves to whichever branch is checked first. When adding a new `IndraOperation`, insert its keyword check at the correct priority relative to existing ones and add a test in `crates/indra/src/agents/specialists/tests.rs` for the ambiguous case, not just the clean case.

2. **State the task's capability floor.** Each `IndraOperation` variant declares its `Requirements` — `vision: bool`, `tool_calling: ToolCalling` (`Reliable`/`Unreliable`/`None`), `domain: Domain` (`Code`/`Document`/`Engineering`/`General`), `min_context_tokens: u32`. This is a floor, not a preference: a task requiring vision must never be handed to a non-vision model, no matter how good that model otherwise is.

3. **Filter by hard-disqualifying capability gaps.** `select_model` first filters the registry to models where `req.vision` implies `m.capabilities.vision`, and `m.capabilities.context_tokens >= req.min_context_tokens`. If the filtered set is empty, `select_model` returns `Err` — it does **not** fall back to a non-vision or too-small model. This is the single most important invariant in the router: a capability gap is a hard failure, surfaced to the caller, never silently degraded.

4. **Rank survivors and assess hardware fit.** Among capability-qualified candidates, `select_model` sorts by domain match, tool-calling match, whether the model would run `Degraded` on this machine, then by context window (larger wins ties). `assess()` compares `min_vram_mb` against available VRAM (or total RAM on a CPU-only box) and returns `Fit::Comfortable`, `Fit::Tight { warning }`, or `Fit::Degraded { warning }` — a hardware shortfall downgrades the fit and attaches a human-readable warning, but the model is still selected and returned, not hidden. This is the fallback behavior: a model too large for available VRAM still gets picked (with CPU offload implied), so a smaller demo machine never sees an empty candidate list for a reason that isn't a genuine capability gap.

5. **Make the choice explainable.** `Selection.reason` is a plain-English string naming the operation's domain/tool-calling requirement and the chosen model's declared domains. `IndraOperation::selection_trace()` wraps this into a `(reason, warning)` pair a session can surface to the user. Never let a selection go unexplained — a refinery engineer or SIH judge auditing a decision needs to see why a model was picked, not just that one was.

## Adding a new open-weight model

Onboarding must be pure data — a new `ModelDescriptor` entry, zero changes to `select_model` or `OperationRouter::classify`:

1. Determine the model's real `Backend` — `Llamacpp`, `Mlx`, or `LocalHttp { endpoint }` for a model served over loopback/private-LAN by vLLM/Ollama/TGI. Any `LocalHttp` endpoint must pass `validate_backend()`, which rejects anything but loopback or RFC1918/ULA/link-local addresses — a domain name is refused outright since DNS resolution happens later and can't be checked at registration time. A `LocalHttp` endpoint pointing off-box is an air-gap violation; cross-check with the air-gap-verification skill before accepting one.
2. Fill `Capabilities` honestly: `vision` only `true` if the model was actually validated with an image, `tool_calling: Reliable` only if it's been exercised against the real sandbox tool-calling loop (not assumed from a model card), `context_tokens` from the model's real configured window, `domains` from what it's actually good at, not what's hoped.
3. Set `min_vram_mb` from a real measurement, and `mmproj_path` when the model needs a paired llama.cpp multimodal projector file for vision.
4. Add the descriptor to the registry and write a test that runs `select_model` for every `IndraOperation` this model should — and should not — win, confirming the outcome without touching selection code.

If adding a model requires editing `select_model` or the classifier, that's a design smell — the taxonomy (`Backend`/`ToolCalling`/`Domain`/vision/`context_tokens`) is missing a dimension the new model needs, and that gap belongs in `Capabilities`/`Requirements`, not a special case in the algorithm.

## Diagnosing a misrouting bug

Trace a handful of representative prompts through `OperationRouter::classify` first (inspection text, a P&ID request, a calculation request, ambiguous general chat) to isolate whether the bug is classifier keyword drift or a `select_model` capability/hardware issue. If a scanned document went to a non-vision model and hallucinated, check whether `IndraOperation::InspectionAnalysis.requirements().vision` is still `true` and whether the registry actually has a vision-capable, correctly-declared model — a `false` capability claim in the registry is a data-quality bug, never patch around it with model-specific logic in `select_model`.

## Worked example

Prompt: "calculate wall thickness from this scanned P&ID reading." This genuinely spans two operations (`EngineeringCalculation` and `PidAnalysis`). `classify()` will pick whichever keyword branch runs first — do not try to encode multi-operation logic into `classify`; that's a multi-step orchestration problem, not a single routing decision, and belongs with whatever coordinates specialist hand-offs.
