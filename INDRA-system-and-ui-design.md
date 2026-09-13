# INDRA — System & Interface Design

**Sovereign agentic workbench · SIH 26117**
Companion to `2026-09-13-sih-26117-sovereign-workbench.md`. That document specifies the Rust backend. This one specifies everything a person sees, plus the contract between the two.

---

## 0. Decisions, up front

| # | Decision | Why |
|---|---|---|
| 1 | The interface is **achromatic**. Black, ash, white. The only colour in the entire application is state signal (blocked / degraded / verified) and the focus ring. | The user's documents *are* the colour — P&IDs, scanned reports, highlighted OCR boxes. A grey workbench makes the drawing the brightest thing on screen. This is also the requested Cursor-like direction. |
| 2 | Every agent decision is **rendered, not summarised**. Plan, replan, model choice, tool arguments, sandbox state, egress state. | The product's entire claim is "you can trust this because it cannot phone home". A claim you cannot see is a claim you cannot sell to a refinery safety officer. |
| 3 | Session context is a **visible, byte-counted ledger of references** — never a transcript. Target ≤ 8 kB per session, hard cap 32 kB. | Matches the tiered-memory decision already made. Also makes compaction honest: the user sees what was dropped. |
| 4 | The memory graph is an **input surface**, not a viewer. Lasso a subgraph → it scopes the next question's retrieval. | This is the single strongest idea here. A read-only graph is a screensaver. |
| 5 | Boot animation is **SVG + the boot log**, ~8 kB, not the 1.6 MB mp4. Each beat of the animation is a real system check. | Decoration that does a job. Also keeps the size claim true. |
| 6 | Ship the desktop client on **Tauri v2**, not Electron. | See §2.4. Electron alone is ~150 MB of Chromium and breaks the "MB not GB" claim on its own. |
| 7 | First run is a **3-step setup**, not a login. Returning users never see an auth screen — they see a 400 ms restore. | Sovereign means there is no account server to log in to. Identity is the plant IdP or a local profile. |

**One thing to correct before it becomes a promise you can't keep.** "Everything in KB" is true for the *session record* and false for the *knowledge index*. Embeddings for 100k document chunks are ~150 MB at 384-dim fp32, and models are gigabytes. Three separate numbers, three separate claims — §2.4 states all three so nobody is caught out on stage.

---

## 1. Who this is for

Three people will use it, and they want different things from the same screen.

| Persona | Job to be done | What makes them abandon it |
|---|---|---|
| **Inspection engineer** (primary) | "Read this scanned NDT report and tell me if P-101 is fit for service — and show me where in the scan you got that." | An answer without a page and a box. One unsourced number and they stop trusting the whole tool. |
| **Plant IT / security officer** (gatekeeper) | "Prove to me this thing never reaches the internet, and that it cannot write to the historian." | Any ambiguity about egress. They are the reason A5 is demoed first. |
| **Department head** (evaluator) | "What did the system do last week, who approved it, on what evidence?" | No audit trail; agent actions that look like magic. |

**Assumptions worth testing before Day 4** — cheap to check, expensive to get wrong:
1. Engineers want the citation *inline in the sentence*, not in a footnote list. (Test: two paper mockups, 5 minutes each.)
2. The plan/tool trace is reassuring at first and noise by week two → hence three density modes (§6.2), defaulting to Normal, remembered per user.
3. Nobody will read a graph unless it does something. Hence lasso-to-scope.

---

## 2. System design

### 2.1 Layers

```
┌──────────────────────────────────────────────────────────────┐
│  CLIENT  Tauri v2 + system WebView          ~9 MB            │
│  ┌────────────┬─────────────┬───────────────┬─────────────┐  │
│  │ Shell      │ Stream      │ Trace         │ Constellat. │  │
│  │ rail/pal.  │ renderer    │ plan+tools    │ canvas 2D   │  │
│  └────────────┴─────────────┴───────────────┴─────────────┘  │
│                    ▲  IndraEvent stream (§2.2)               │
└────────────────────┼─────────────────────────────────────────┘
                     │ ACP over local socket · loopback only
┌────────────────────┼─────────────────────────────────────────┐
│  RUNTIME  crates/indra                     ~34 MB            │
│  ┌──────────────────────────────────────────────────────┐    │
│  │ OperationRouter → Specialist → model_registry        │    │
│  │            ↓                        ↓                 │    │
│  │      tool fabric            sovereign::sandbox        │    │
│  │            ↓                  (net: none)             │    │
│  │      egress_inspector  ── every socket, always        │    │
│  └──────────────────────────────────────────────────────┘    │
├───────────────────────────────────────────────────────────────┤
│  MEMORY                                                       │
│  L1 session record   ≤ 32 kB   zstd(CBOR), references only    │
│  L2 local index      ~MB–GB    chunks, embeddings, graph      │
│  L3 plant server     source of truth, read-only by default    │
├───────────────────────────────────────────────────────────────┤
│  MODELS  user-managed store, GB, outside the app bundle       │
└───────────────────────────────────────────────────────────────┘
```

### 2.2 The event contract

Build this first. It is the seam that lets the interface and the Rust backend be built in parallel, and it is what makes planning and tool use visible without the UI guessing.

One append-only, ordered stream per turn. Every event carries `turn_id` and a monotonic `seq`.

```ts
type IndraEvent =
  // boot — drives the animation beats in §5.1
  | { t:"boot.step",  id:BootStepId, label:string,
      state:"pending"|"running"|"ok"|"warn"|"fail", detail?:string }

  // planning — the agent thinking out loud, structurally
  | { t:"turn.start",     turn_id:string, prompt_digest:string }
  | { t:"plan.proposed",  steps:Step[] }
  | { t:"plan.revised",   after_step:string, steps:Step[], reason:string }
  | { t:"step.state",     step_id:string,
      state:"running"|"done"|"skipped"|"failed", ms?:number }

  // model routing — A1 made visible
  | { t:"model.selected", model_id:string, reason:string,
      fit:{ kind:"comfortable"|"tight"|"degraded", warning?:string } }

  // tools — argument-level transparency
  | { t:"tool.call",   call_id:string, step_id:string, name:string,
      args:Json, sandbox:{ backend:"docker"|"unshare"|"none", net:"none" } }
  | { t:"tool.result", call_id:string, ok:boolean, ms:number,
      summary:string, bytes:number, citations?:BoxRef[] }

  // generation
  | { t:"text.delta", text:string }
  | { t:"citation",   span:[number,number], ref:BoxRef }

  // context — the ledger in §6.3
  | { t:"context.delta",     session_bytes:number, budget_bytes:number,
      added:Ref[], dropped:Ref[] }
  | { t:"context.compacted", from_turns:number, to_bytes:number,
      kept:Ref[], dropped:Ref[] }

  // second brain — makes the constellation live
  | { t:"memory.touch", node_ids:string[], op:"read"|"cache"|"evict"|"stale" }

  // sovereignty and verification
  | { t:"egress.attempt",   url:string, blocked:true, at:string }
  | { t:"verify.recompute", claim:number, computed:number,
      verdict:"agrees"|"disagrees" }
  | { t:"guard.blocked",    resource:string, reason:"sealed"|"acl"|"approval" }

  | { t:"turn.end", ms:number, tokens:{in:number,out:number} };

type Step   = { id:string; label:string; tool?:string };
type BoxRef = { doc_id:string; version:string; page:number;
                bbox:[number,number,number,number]; conf:number };
type Ref    = { kind:"doc"|"chunk"|"decision"|"artifact"|"fact";
                id:string; bytes:number; label:string };
```

`BoxRef` is exactly the box-level provenance from Task 5 of the build plan. `fit.warning` is the string the registry produces in Task 2 — the interface renders it **verbatim**, never paraphrased, because the plan requires that and because a paraphrased hardware warning is a lie with extra steps.

### 2.3 Session context model

A session record is a list of references and a few short strings. Never a transcript.

```
SessionRecord {
  id, started_at, actor{employee_id, role, dept},
  scope:      [doc_id@version, ...]          // what's in play
  facts:      [{text ≤ 140 chars, ref}]      // pinned, cited
  decisions:  [{text ≤ 200 chars, at, by}]
  artifacts:  [{path, sha256, produced_by}]
  open:       [{question ≤ 140 chars}]
  turns:      [{digest, ms, tool_names[]}]   // no bodies
}
```

Encoded as CBOR, zstd level 9. Typical realistic session: 3.5–8 kB. Hard cap 32 kB, enforced by compaction, which the user sees (§6.3).

At 8 kB average, 10,000 sessions is **80 MB**. That is the number to quote.

### 2.4 Size budget — the honest table

Three claims, kept apart.

| | Component | Size | Notes |
|---|---|---|---|
| **App** | Rust runtime, release + LTO + strip | 30–38 MB | `opt-level="z"` on cold crates, `panic="abort"` |
| | Tauri client (system WebView) | 8–12 MB | Electron would be **~150 MB** here |
| | UI bundle (JS+CSS, no framework runtime beyond preact-sized) | ≤ 400 kB | |
| | Fonts, subset woff2, 2 families × 3 weights | ~180 kB | Latin + Devanagari subset for i18n |
| | Icon sprite, inline SVG | ~14 kB | no icon font |
| | Boot animation, SVG paths + JS | ~8 kB | vs 1.6 MB for the mp4 |
| | Tesseract binary + eng traineddata | 15–20 MB | `tessdata_fast` |
| | **App total** | **≈ 75–95 MB** | *MB, not GB — claim holds* |
| **Session data** | per session | 3.5–8 kB | 10k sessions ≈ 80 MB |
| **Knowledge index** | per 1,000 chunks | ~1.6 MB | embeddings 384-dim fp32 + graph + metadata |
| | 100k chunks (a mid-size plant) | ~160 MB | drops to ~45 MB with int8 quantised vectors |
| **Models** | 3B–7B quantised, per model | 2–5 GB | **outside the bundle**, user-managed, never shipped |

Say it on stage in exactly this order: *the application is under 100 MB, a session costs kilobytes, the models are yours and live in your directory.* That sequence pre-empts the "but the models are 4 GB" question instead of being ambushed by it.

If Electron must stay for schedule reasons, the honest line becomes "≈ 230 MB". Still MB. Still true. Tauri is better; do not lie about which one shipped.

### 2.5 Identity and session lifecycle

There is no account server. Two modes, chosen at first run.

**Standalone** — a local profile, passphrase-derived key (Argon2id), wraps the sealed-store key. No network anything.
**Plant** — OIDC against the on-prem IdP (Keycloak et al.), which supplies Employee ID → Department → Role → Manager. RBAC comes from there; the agent never decides permissions.

```
cold start
   │
   ├─ no profile ────► BOOT ANIM ──► SETUP (3 steps) ──► WORK
   │
   ├─ profile + valid keychain token ──► BOOT ANIM ──► WORK
   │                                     (animation cut short the
   │                                      moment checks finish)
   │
   └─ profile + expired/locked ────────► BOOT ANIM ──► LOCK OVERLAY
                                          (workspace visible, blurred,
                                           behind it — context not lost)
```

The refresh token lives in the OS keychain and is device-bound; the sealed store key wraps to it, so a copied data directory is inert on another machine. That property is worth one sentence in the demo.

### 2.6 System prompt

The behaviour the interface depends on has to be guaranteed in the prompt, or the UI ends up rendering promises the model doesn't keep.

```text
You are INDRA, an agentic assistant running entirely on this machine,
inside a plant that has chosen not to send its data anywhere.

OPERATING TRUTH
- You have no network access. Not restricted — absent. Never suggest
  looking something up online, checking a vendor site, or verifying
  against an external source. Those actions do not exist here.
- Your knowledge of this plant comes only from the indexed corpus and
  the tools you are given. If neither covers the question, say so and
  name what document would answer it.

EVIDENCE
- Every factual claim about a document carries a citation to the exact
  region it came from: document, version, page, bounding box.
- You may not cite a region you did not read. If OCR confidence for a
  region is below 0.60, quote it and mark it uncertain rather than
  smoothing it into a confident sentence.
- If two sources disagree, present both with their citations and say
  which is more recent. Do not resolve the conflict silently.

NUMBERS
- Any number you derive, rather than read, must be computed by running
  code in the sandbox. State the formula, run it, report the result.
  A number you produced from reasoning alone is a draft, and you say so.

AUTHORITY
- You have read access. Any change to plant data is a proposal: state
  the current value, the proposed value, the evidence, and the role
  required to approve it. Then stop. You do not apply it.
- You never infer whether someone is allowed to see something. If a
  tool returns a permission error, report it plainly. Do not work around
  it, and do not summarise what you could not read.

HONESTY ABOUT YOURSELF
- If the model serving this request was selected with a degraded fit
  warning, or lacks a capability the task wanted, say so in your first
  sentence.
- When you are unsure, the sentence begins with what you don't know.

STYLE
- Plain sentences. The reader is an engineer under time pressure.
- Lead with the finding, then the evidence, then the caveat.
- No preamble, no restating the question, no closing summary.
```
---

## 3. Design system

### 3.1 Direction

Three words: **quiet, dense, provable.**

Quiet — the chrome recedes so scanned drawings and OCR overlays are the loudest thing on screen. Dense — this is a workbench used for hours, not a landing page; 14 px base, tight leading, no card padding for its own sake. Provable — every surface that makes a claim (a citation, a model choice, a sandbox run) has a place to show its receipt without opening a modal.

The mark helps. A ring above a stadium: a head above a body, and also a lowercase **i**. It draws in two strokes, survives 16 px, and the boot animation is a figure building itself out of those two strokes.

```
logo geometry, 24-unit grid, stroke 2.6, round caps
  ring     cx 12  cy 5.4   r 3.5      + centre dot r 1.15 (filled)
  stadium  x 8.5  y 10.6   w 7  h 11  rx 3.5
  gap between them: 1.7 units — never compress it
```

### 3.2 Colour

Neutral ramp. No hue in the greys — a tinted near-black is the tell of a generated theme, and here it would also fight the greyscale of a scanned drawing.

**Ash Dark** (default)

| Token | Hex | Use |
|---|---|---|
| `--bg` | `#0A0A0A` | app canvas |
| `--surface` | `#121212` | panels, rail, top bar |
| `--raised` | `#1A1A1A` | tool rows, cards, inputs |
| `--hover` | `#232323` | hover fill |
| `--line` | `#2C2C2C` | hairlines, dividers |
| `--line-strong` | `#3A3A3A` | active borders |
| `--text-faint` | `#5C5C5C` | disabled, timestamps |
| `--text-dim` | `#8A8A8A` | secondary, labels |
| `--text` | `#D6D6D6` | body |
| `--text-hi` | `#FFFFFF` | headings, active nav, the caret |

**Chalk Light**

| Token | Hex | Use |
|---|---|---|
| `--bg` | `#FAFAFA` | app canvas |
| `--surface` | `#FFFFFF` | panels |
| `--raised` | `#F4F4F4` | cards |
| `--hover` | `#EDEDED` | hover fill |
| `--line` | `#E2E2E2` | hairlines |
| `--line-strong` | `#CFCFCF` | active borders |
| `--text-faint` | `#A0A0A0` | disabled |
| `--text-dim` | `#6B6B6B` | secondary |
| `--text` | `#1F1F1F` | body |
| `--text-hi` | `#000000` | headings |

**Signal** — the only chroma in the product. Same hues in both themes, lightness adjusted.

| Token | Dark | Light | Meaning — one meaning each, never reused decoratively |
|---|---|---|---|
| `--sealed` | `#5E9E77` | `#2F7A4E` | verified · sealed · recomputation agrees · zero egress |
| `--degraded` | `#C7943F` | `#9A6B18` | hardware shortfall · low OCR confidence · stale cache |
| `--blocked` | `#C4675A` | `#A83B2C` | egress attempt blocked · write guard fired · ACL denied |
| `--focus` | `#4C7DF0` | `#2E5FD0` | focus ring and text caret only. Never a fill, never a button. |

`--focus` is the logo's blue. It appears on exactly two things, so when it appears you know where you are.

**Rule:** signal colours never appear as a background fill larger than 2 px. They are a left border, a dot, a rule, or text. A red panel makes a workbench feel like an incident; a 2 px red rule makes it feel like a record.

### 3.3 Typography

Not Inter. Two families, both free, both subsettable.

| Role | Family | Fallback |
|---|---|---|
| UI + body | **Geist Sans** | `ui-sans-serif, system-ui` |
| Code, data, IDs, byte counts | **Geist Mono** | `ui-monospace, SFMono-Regular` |

Scale — compact, for density. Line height in parentheses.

| Token | px | Use |
|---|---|---|
| `--t-11` | 11 (16) | byte counts, timestamps, tool durations — mono |
| `--t-12` | 12 (18) | labels, tool rows, ledger cells |
| `--t-13` | 13 (20) | secondary body, side inspector |
| `--t-14` | 14 (22) | **base** — chat body, everything default |
| `--t-16` | 16 (24) | user's own message, section heads |
| `--t-20` | 20 (28) | screen titles |
| `--t-28` | 28 (34) | first-run headings only |

Weights: 400 body, 500 labels and active nav, 600 headings. Nothing heavier — bold in a grey UI reads as an alarm.

Measure caps at 78ch in the transcript. Numerals are tabular everywhere (`font-variant-numeric: tabular-nums`) so the context gauge doesn't jitter as it counts.

**Avoid:** all-caps tracked labels, single accented words in headings, `→` glued onto button text. All three are on the generic list and all three would look wrong here anyway.

### 3.4 Space, radius, line, elevation

Space scale, 4 px base: `2 4 6 8 12 16 24 32 48 64`. Nothing between.

Radius carries hierarchy rather than being one value everywhere:

| Token | px | On |
|---|---|---|
| `--r-sm` | 3 | inputs, chips, tool rows |
| `--r-md` | 6 | panels, cards, dialogs |
| `--r-lg` | 10 | the composer, the one focal surface |
| `--r-full` | 999 | avatars, status dots |

Borders do the work shadows do elsewhere. `1px solid var(--line)` at rest, `--line-strong` when active. Exactly two shadows exist: `--shadow-pop` for the command palette and dialogs (`0 12px 32px rgba(0,0,0,.45)`), and nothing else. Cards do not float.

### 3.5 Motion

| Token | ms | Easing | Use |
|---|---|---|---|
| `--m-tap` | 90 | `cubic-bezier(.2,0,.4,1)` | press, toggle |
| `--m-ui` | 160 | `cubic-bezier(.2,0,0,1)` | hover, disclosure, panel slide |
| `--m-enter` | 240 | `cubic-bezier(.16,1,.3,1)` | dialog, inspector, plan card |
| `--m-glyph` | 120 | `linear` | per-character stream reveal (§6.1) |
| `--m-boot` | 2200 | scripted | the one orchestrated moment |

Everything except the boot sequence is a response to something the person did. No section entrance animations, no card hover lifts.

`prefers-reduced-motion: reduce` → boot becomes a static mark with a text checklist; stream reveal becomes instant per word; panels cut instead of slide. Nothing is lost, only the choreography.

### 3.6 Theming engine

Requirement: fonts and colours changeable by the user. Implementation: everything above is a CSS custom property on `:root`; a theme is an override object.

```json
{ "name":"Ash Dark", "base":"dark",
  "font":{"ui":"Geist Sans","mono":"Geist Mono","scale":1.0,"ligatures":false},
  "density":"compact",
  "overrides":{"--focus":"#4C7DF0"} }
```

Under 1 kB. Serialisable to a text blob, so a plant can paste a house theme into a field and hit apply — no rebuild, no theme store, no network.

Appearance panel controls: **base** (Ash Dark / Chalk Light / High Contrast / OLED true-black) · **UI font** (Geist / IBM Plex Sans / system) · **Mono font** (Geist Mono / JetBrains Mono / system) · **Scale** (0.9 / 1.0 / 1.1 / 1.25) · **Density** (compact / comfortable) · **Mono ligatures** · **Motion** (full / reduced / none) · **Stream speed** (calm 45 c/s / normal 65 / instant) · **Trace density** (quiet / normal / trace).

Every control previews live against the current screen. No "restart to apply", ever.

---

## 4. Shell

Minimal navigation, maximal working area. Two fixed elements, both thin.

```
┌──┬───────────────────────────────────────────────────────────┐
│  │ P-101 fit-for-service            4.1 kB/32 kB   ⌘K   ◐ │ 36px
│⬤ ├───────────────────────────────────────────────────────────┤
│  │                                                           │
│▣ │                                                           │
│  │              working area — nothing fixed                 │
│◈ │              78ch measure, centred, scrolls               │
│  │                                                           │
│⊞ │                                                           │
│  │                                                           │
│⛨ │                                                           │
│  │   ┌───────────────────────────────────────────────────┐   │
│  │   │  Ask about P-101…                        ⏎        │   │
│──│   └───────────────────────────────────────────────────┘   │
│◍ │      qwen-vl-7b · sealed · 3 sources in scope             │
└──┴───────────────────────────────────────────────────────────┘
 48px
```

**Left rail, 48 px, icon-only, five destinations.** Work ⬤ · Memory ▣ · Sources ◈ · Trace ⊞ · Sovereignty ⛨. Profile ◍ pinned to the bottom. Active state is `--text-hi` plus a 2 px left bar; there is no expanded label mode, because the palette is the real navigation and a labelled sidebar would just be a permanently-open menu.

**Top bar, 36 px.** Session title (editable in place) · context gauge (§6.3) · palette hint · theme toggle. Nothing else earns 36 px of permanent screen.

**⌘K palette.** Navigate, switch session, jump to a document, run a saved recipe, change theme, scope retrieval to a saved subgraph. Fuzzy over one flat index. This is why five icons suffice.

**Panels are transient.** The trace rail, the source inspector, the context ledger all open as a right-side sheet 380–520 px wide, draggable, dismissed with `Esc`. Only one at a time. The transcript never reflows narrower than 62ch; below that the sheet overlays instead of pushing.
---

## 5. Screens

### 5.1 Boot — the one orchestrated moment

Rebuilt from the reference animation as SVG, and wired to real checks. The figure assembles the mark; each beat is a `boot.step` event resolving.

| Beat | ms | On screen | Real check |
|---|---|---|---|
| 1 | 0–500 | Empty ground plane. Figure walks in from the left. | process start, config read |
| 2 | 500–1000 | Figure pushes the **stadium** upright into centre. | hardware probe · `HardwareProfile::probe()` |
| 3 | 1000–1500 | Figure rolls the **ring** in from the right. | model registry scan, capability match |
| 4 | 1500–1900 | Ring lifts, settles above the stadium. Mark complete. | sealed store mounted, keychain unwrapped |
| 5 | 1900–2200 | Figure walks out. Mark holds, then scales down into the top-left rail position. | egress inspector armed — **zero attempts** |

Under the mark, one line of mono at `--t-12`, `--text-dim`, replaced per beat. No progress bar; the animation *is* the progress bar.

If checks finish before 2200 ms, the sequence fast-forwards to beat 5 rather than stalling — a boot animation that outlasts the boot is theatre, and this crowd will notice. If a check fails, the figure stops, the failing glyph tints `--blocked`, and the line becomes the actual error with a Retry / Continue anyway pair. A `warn` (e.g. degraded model fit) tints `--degraded` and continues; the warning is carried into the workspace as a dismissible strip.

Reduced motion: mark drawn statically, five checklist lines resolving in place.

Assets: two `<path>` elements for the mark, one 9-keyframe figure sprite, ~8 kB total. No video, no Lottie runtime.

### 5.2 First run — setup, three steps

Centred column, 520 px, `--t-28` headings, generous space — the only screen in the app that breathes. Left rail hidden; there is nowhere else to go yet.

**Step 1 · Who is using this**

Two cards, not a form.

```
┌──────────────────────────┐  ┌──────────────────────────┐
│ Plant sign-in            │  │ Standalone profile       │
│                          │  │                          │
│ Uses your plant's        │  │ One machine, one         │
│ identity server. Your    │  │ passphrase. No identity  │
│ role and department      │  │ server. You get full     │
│ decide what INDRA can    │  │ access to whatever is    │
│ read.                    │  │ on this disk.            │
│                          │  │                          │
│ Recommended        [Use] │  │                    [Use] │
└──────────────────────────┘  └──────────────────────────┘
```

Plant path: discovery URL → browser-less device code against the on-prem IdP → shows the resolved claims for confirmation (`Karthick R · Inspection · Engineer II · reports to A. Nair`) before continuing. Standalone path: passphrase + confirm, with an honest strength meter and a plain warning that a forgotten passphrase means the sealed store is unrecoverable — because it is.

**Step 2 · Where your models live**

Directory picker. On selection, the registry scans and renders a live table — this is A1, shown before the user has asked anything.

```
  MODEL                 SIZE    VISION  TOOLS      CONTEXT   FIT
  qwen2.5-coder-7b-q4   4.1 GB    —     reliable    32k      ✓ comfortable
  llama-3.2-vision-11b  6.8 GB    ✓     unreliable  16k      ▲ 6800 MB needed,
                                                               5200 MB available —
                                                               offloading to CPU,
                                                               expect substantially
                                                               slower generation
  nomic-embed-v1.5      274 MB    —     none         8k      ✓ comfortable
```

The warning string is printed exactly as the registry returned it. Empty directory → empty state that names the two files it needs and where to get them offline, never a dead end.

**Step 3 · Prove it is sealed**

The trust moment, and it is interactive rather than a claim.

```
  Sovereignty check

  ⛨  Egress inspector armed          0 attempts since start

      INDRA cannot reach the network. Test it yourself:

      ┌─────────────────────────────────────────────┐
      │ https://example.com                     [→] │
      └─────────────────────────────────────────────┘

      ─── 14:02:31  https://example.com
          BLOCKED at socket · recorded · attempt #1
```

The user types any URL. It gets blocked, logged, and appears in a permanent record they can revisit on the Sovereignty page. Then: **Start working**.

### 5.3 Returning and locked

Returning: boot animation (usually cut short at ~700 ms) → last workspace restored, scroll position included. No auth screen. Ever.

Locked (policy timeout, lid close, `⌘L`): workspace stays on screen behind a 12 px backdrop blur, dimmed to 45%. A 320 px card asks for passphrase or IdP re-auth. Nothing is unloaded, nothing scrolls away, and the person sees the work they are returning to. Wrong passphrase shakes 4 px once and states attempts remaining; after the limit, the sealed key is dropped from memory and the app returns to cold boot rather than pretending.

### 5.4 Work

The default screen. Transcript centred at 78ch, composer pinned at the bottom of the column, everything else summoned.

```
   ┌─────────────────────────────────────────────────────────┐
   │  You                                            14:02   │
   │  Is P-101 fit for service? Use the August NDT.          │
   ├─────────────────────────────────────────────────────────┤
   │  ▣  Plan                                    3 steps  ⌄  │
   │     ✓  Locate August NDT report            read   0.3s  │
   │     ✓  Extract wall-thickness readings      ocr   4.1s  │
   │     ●  Compare against minimum allowable   calc         │
   ├─────────────────────────────────────────────────────────┤
   │  ⟶ ocr_document  NDT_2026_08.pdf                  ⌄     │
   │    38 regions · 4 below 0.60 confidence · sandboxed,    │
   │    no network                                    4.1s   │
   ├─────────────────────────────────────────────────────────┤
   │  llama-3.2-vision selected — vision required.           │
   │  ▲ 6800 MB needed, 5200 MB available — offloading to    │
   │    CPU, expect substantially slower generation          │
   ├─────────────────────────────────────────────────────────┤
   │  INDRA                                                  │
   │  P-101 is below minimum allowable at two locations.     │
   │  The August NDT records 6.2 mm at grid E-4 ⟦1⟧ against  │
   │  a minimum allowable of 7.1 mm ⟦2⟧. I computed the      │
   │  remaining life at 1.4 years ⟦calc⟧ — recomputed in the │
   │  sandbox and it agrees. ▌                               │
   └─────────────────────────────────────────────────────────┘
```

Citation chips `⟦1⟧` sit inline in the sentence, not in a footer. Hover previews a 200 px crop of the source region; click opens the source inspector on that page with the box outlined. `⟦calc⟧` opens the code that ran and its recomputation verdict.

Composer: `--r-lg`, 1 px `--line`, `--line-strong` on focus plus a 2 px `--focus` ring. Under it, a status line in mono `--t-11`: active model · sandbox state · number of documents in scope. Attach, scope-picker, and run-recipe live as three ghost icons inside the composer's right edge.

Streaming rules and the trace rail are specified in §6.

### 5.5 Memory — the second brain, interactive

The screen the brief is most specific about: not a static viewer. Three coordinated views over one graph, switched without leaving the page. Selection, filters, and scope persist across the switch.

```
 ┌ Constellation ┬ Ledger ┬ Timeline ┐        482 nodes · 1,914 edges
 │                                                                   │
 │        ○ ─── ⬤ P-101 ─── ○        filter ┌───────────────────┐   │
 │       ╱      ╱  ╲       ╲                │ dept   Inspection │   │
 │      ○      ⬤    ◍       ○               │ state  cached     │   │
 │            ╱      ╲                      │ since  90 days    │   │
 │           ⬤        ○                     └───────────────────┘   │
 │                                                                   │
 │   ⬤ cached   ○ indexed   ◍ evicted   ◐ stale                     │
 └───────────────────────────────────────────────────────────────────┘
```

**Constellation.** Canvas 2D, force-directed, quadtree hit-testing — DOM/SVG dies past ~2k nodes and a plant corpus is 50k. Encoding is functional, never decorative:

| Channel | Encodes |
|---|---|
| radius | how often this node has been cited across sessions |
| fill | cache state — cached / indexed-only / evicted / stale after a server version change |
| ring | classification, from the ACL that came with the object |
| edge weight | relationship strength from the knowledge graph |
| edge style | solid = derived-from · dashed = references · dotted = superseded-by |

**Live.** During an agent turn, `memory.touch` events pulse the nodes being read, in real time, 400 ms decay. Watching the second brain get read is the demo moment for this page and costs almost nothing to build once the event stream exists.

**Lasso to scope.** Drag a selection → a floating bar: `14 nodes selected · Ask about this scope · Save as view · Export refs`. Choosing *Ask* jumps to Work with retrieval scoped to that subgraph and a chip in the composer showing it. This is what makes the page an instrument instead of a poster.

**Ledger.** Virtualised table for the people who distrust graphs — every node with id, version, hash prefix, bytes, ACL, last cited, sessions citing. Sortable, and every column filterable. Same selection model as the constellation.

**Timeline.** Horizontal, one lane per document. Version bumps, cache fills, invalidations, citations. Answers "when did this change and did anything we concluded depend on the old version" — which is the question an auditor actually asks. When a superseded version was cited by a past session, that citation renders in `--degraded` and links to the affected session.

Inspector (right sheet, 480 px) for any selected node: chunk previews with their embeddings' nearest neighbours, version and hash, ACL and who granted it, cache age, evict / pin / refresh actions, and the list of sessions that cited it.

### 5.6 Sources

Document library and the reading surface. Grid or list. Each item shows type, version, indexed state, classification, and page count.

The reader is where box-level provenance pays off: original scan rendered at full fidelity, OCR boxes as a toggleable overlay, confidence rendered as border weight, sub-0.60 boxes outlined `--degraded`. Click a box → every claim in every session that cited it. Reverse provenance, and it takes one screen once `BoxRef` exists.

Sealed resources carry a lock in the corner and a tooltip that names the policy, not a generic "no permission".

### 5.7 Trace and approvals

Runs, filterable by date, actor, specialist, tool, verdict. A run expands into its full event stream — the same events the Work screen rendered live, replayable. Export as signed JSON for the plant's audit system.

Approvals queue: every proposed change the write guard stopped. Each shows current value, proposed value, evidence with citations, the role required, and Approve / Reject / Request evidence. Nothing is applied without a name attached.

### 5.8 Sovereignty

The gatekeeper's screen, and the one to open the demo with.

```
   ⛨  SEALED                       0 outbound attempts · 14d 6h uptime

   ┌ Egress ─────────────────────────────────────────────────────┐
   │  Every socket is inspected. Loopback to 127.0.0.1:11434 is   │
   │  the only permitted destination, validated at config time.   │
   │                                                              │
   │  Test it:  ┌──────────────────────────────────┐  [Probe]    │
   │            │ https://…                        │             │
   │                                                              │
   │  09-13 14:02  https://example.com   BLOCKED  socket  user    │
   │  09-11 09:41  https://api.openai.com BLOCKED socket  config  │
   └──────────────────────────────────────────────────────────────┘

   ┌ Sandbox ──────────┐ ┌ Models ───────────┐ ┌ Sealed ─────────┐
   │ docker · net none │ │ 3 loaded          │ │ 12 resources    │
   │ last run 4.1s ✓   │ │ 1 degraded fit ▲  │ │ 0 writes ever   │
   └───────────────────┘ └───────────────────┘ └─────────────────┘
```

The probe box is deliberately prominent. Handing the sceptic the weapon and watching it fail is more persuasive than any badge.

### 5.9 Appearance

Left column of controls (§3.6), right side a live preview showing a chat exchange, a tool row, and a code block — the three things whose legibility actually matters. Changes apply instantly and persist per user.
---

## 6. Signature interactions

These four are what make the product feel like itself. Everything else is competent chrome.

### 6.1 Stream rendering

The brief asks for text that arrives beautifully — organic, not the jerky burst-render most clients ship. The reason most clients look bad is that they paint whatever the model emits, whenever it emits it, and model output is bursty. The fix is a jitter buffer.

**Pipeline.** `text.delta` → append to a character queue → a rAF loop drains at a *smoothed* rate → committed graphemes render.

**Drain rate.**

```
target = clamp(base + (queue.length - 60) * 1.4, base * 0.6, 400)   // chars/sec
base   = 65   (Calm 45 · Normal 65 · Instant ∞)
```

Steady at the user's chosen pace when the model keeps up; accelerates when the queue grows so the UI never lags the model by more than ~1 s; slows rather than stalling when the model pauses to think. A stall of >900 ms swaps the caret for a three-dot breathing indicator until deltas resume.

**Reveal unit is the grapheme cluster**, not the token. Never split an emoji, a Devanagari conjunct, or a UTF-8 sequence. The word-boundary rule: a word never appears half-drawn on screen at rest — if the queue empties mid-word, the remaining characters of that word commit immediately.

**Per-character entrance** — this is the softness the brief is after:

```css
@keyframes glyph-in {
  from { opacity: 0; filter: blur(2.5px); }
  to   { opacity: 1; filter: blur(0);     }
}
.g { animation: glyph-in var(--m-glyph) linear both; }
```

Opacity and blur only. **No `translateY`** — vertical motion on a character forces a repaint of the line and produces exactly the jitter this is meant to remove. Characters are wrapped in spans only for the ~120 chars still animating; each unwraps into plain text when its animation ends, so a 4,000-word answer is not 20,000 live DOM nodes.

**Caret.** 2 px block, `--text-hi`, riding the last committed glyph. Solid while streaming; 1.06 s blink when idle; hidden when the turn ends.

**Layout stability.** Markdown parses incrementally, but a block only *commits its chrome* when its delimiter closes — a code fence renders as plain mono text until the closing fence arrives, then gains its border, background, and language chip in one 160 ms transition. Otherwise the answer visibly rebuilds itself as it writes, which is the ugliest thing a chat UI does. Tables buffer entirely and appear complete. `text-wrap: pretty` on paragraphs; reserved `min-height` on the message container so the composer never hops.

**Interruption.** `Esc` stops generation, keeps everything already committed, and marks the message `stopped` with the partial trace intact. Never delete text the user already read.

### 6.2 Plan and tool trace

Planning and tool selection are rendered structurally, from `plan.*`, `step.*`, `tool.*`, and `model.selected`.

**Plan card** appears on `plan.proposed`, above the answer, collapsed to a header plus step rows. States: `queued` (dim, hollow dot) · `running` (dot fills, label to `--text`) · `done` (✓, duration in mono) · `skipped` (struck, reason on hover) · `failed` (`--blocked` left rule).

**Replanning is shown, not hidden.** On `plan.revised`, new steps insert with a 240 ms height animation and a `--focus` left tick that fades over 2 s; superseded steps strike through and dim rather than vanishing. A user who watches the agent change its mind trusts it more than one who watches steps appear from nowhere — but only if the change is legible.

**Tool row.** One line at rest:

```
⟶ ocr_document  NDT_2026_08.pdf   38 regions · sandboxed, no network   4.1s  ⌄
```

Expanded: full arguments (folded JSON, copyable), result summary, bytes returned, sandbox backend, egress state, and the citations produced. The sandbox/egress badge on every single tool row is the INDRA-specific move — it turns the sovereignty claim from a page you visit once into a fact repeated on every action.

**Model switch chip** on `model.selected` when the model differs from the previous turn: `qwen-coder → llama-3.2-vision · vision required`. A `degraded` fit renders the registry's warning string verbatim below it on a `--degraded` left rule. Never paraphrased, never truncated, never behind a disclosure.

**Density**, remembered per user:

| Mode | Shows |
|---|---|
| Quiet | one summary line per turn: `3 steps · 2 tools · 4.4s` — expandable |
| **Normal** (default) | plan card + collapsed tool rows + model chips |
| Trace | all of the above expanded, plus timings, token counts, raw events |

### 6.3 The context ledger

Session context is visible and byte-counted. This is a differentiator that costs one panel.

**Gauge** in the top bar, mono, tabular: `4.1 kB / 32 kB`. A 60 px hairline bar fills as the record grows. It ticks on `context.delta`, animating 160 ms — enough to notice, not enough to distract. Past 70% it takes `--degraded`; at cap it holds and compaction runs.

**Ledger sheet** (click the gauge) shows the record for what it is — references, not a transcript.

```
  SESSION CONTEXT                              4,118 B / 32,768 B

  Scope                                                    1,204 B
    NDT_2026_08.pdf            v3   pages 1,4,7      pinned  412 B
    Pump_Inspection_SOP.pdf    v1   §4.2                     398 B
    Vibration_2026.xlsx        v7   sheet 2                  394 B

  Facts                                                      840 B
    Wall thickness 6.2 mm at grid E-4       ⟦NDT p4⟧  pinned 156 B
    Minimum allowable 7.1 mm                ⟦SOP §4.2⟧       148 B

  Decisions                                                  512 B
    Use ASME B31.3 remaining-life method            14:04    198 B

  Artifacts                                                  388 B
    Approval_Note.docx           sha 7f3a…                   198 B

  Turns (digests only, no bodies)                          1,174 B
```

Every row has a byte count. Every row can be pinned (survives compaction) or dropped (removed now, freeing bytes). Hovering a fact highlights its citation in the transcript.

**Compaction is an event in the transcript, not a silent trim.** When the record hits budget, a divider appears inline:

```
 ─────────────  Compacted · 38 turns → 1.2 kB  ─────────────
                6 references kept · 2 dropped        [ show ]
```

Expanding lists exactly what was dropped, with a restore action per item. A system that quietly forgets is a system nobody plans around; a system that says what it forgot and lets you put it back is one people learn to drive.

### 6.4 Citation → box

Inline chip `⟦1⟧`, `--text-dim` at rest, `--text-hi` with a `--focus` underline on hover. Hover after 220 ms → floating 240 px crop of the exact `bbox`, with page number and OCR confidence. Click → Sources reader opens on that page, the box outlined and pulsed once, transcript stays alive in a 40% split so the reader keeps their place.

Confidence below 0.60: chip carries a small `--degraded` dot, the preview says so, and the sentence containing it inherits nothing — the model was instructed to hedge in words rather than the UI hedging on its behalf.

A claim with no citation gets no chip, and in Trace density a `--degraded` marker in the gutter flags the sentence as uncited. That marker is the most useful QA tool you will build.

---

## 7. Handoff specs

### 7.1 Components

| Component | Variants | States | Notes |
|---|---|---|---|
| Button | primary (filled `--text-hi` on `--bg`), secondary (border), ghost | default, hover, active, focus-visible, disabled, loading | 28 px compact / 32 px default height. Loading replaces the label with a 12 px spinner and locks width. |
| Input | text, search, passphrase | default, focus, error, disabled, readonly | Error is a 1 px `--blocked` border plus a `--t-12` message below. Never a red fill. |
| Tool row | default, expanded | queued, running, ok, failed | Height 32 px collapsed. Chevron rotates 160 ms. |
| Plan card | — | proposed, running, revised, complete, failed | Left rule 2 px indicates aggregate state. |
| Citation chip | inline, gutter | default, hover, active, low-confidence | Baseline-aligned, never taller than the line box. |
| Context gauge | — | ok, near-cap, at-cap | Tabular numerals, mandatory. |
| Node (constellation) | doc, chunk, asset, decision | indexed, cached, evicted, stale, selected, touched | Canvas-drawn; hit target 12 px minimum regardless of visual radius. |
| Sheet | right, 380 / 480 / 520 px | opening, open, closing | `Esc` closes. One at a time. Focus trapped while open. |
| Palette | — | closed, open, filtering, empty | Opens in 120 ms, no backdrop animation. |
| Empty state | screen, panel, list | — | Always names the next action. Never an illustration. |

### 7.2 Motion table

| Element | Trigger | Animation | ms | Easing |
|---|---|---|---|---|
| Boot sequence | cold start | scripted, 5 beats | 2200 | scripted |
| Glyph | stream commit | opacity + blur | 120 | linear |
| Plan step → running | `step.state` | dot fill, label lift in colour | 160 | `--m-ui` |
| Plan revision | `plan.revised` | height insert + tick fade | 240 → 2000 | `--m-enter` |
| Tool row expand | click | height auto, chevron rotate | 160 | `--m-ui` |
| Sheet | open | slide 24 px + fade | 240 | `--m-enter` |
| Context gauge | `context.delta` | width + number roll | 160 | `--m-ui` |
| Node pulse | `memory.touch` | radius ×1.6, opacity decay | 400 | ease-out |
| Citation preview | hover 220 ms | fade + 4 px rise | 160 | `--m-ui` |
| Lock overlay | timeout | backdrop blur 0→12 px | 240 | `--m-enter` |

### 7.3 Keyboard

`⌘K` palette · `⌘L` lock · `⌘\` toggle right sheet · `⌘⇧C` context ledger · `⌘1-5` rail destinations · `Esc` stop generation, then close sheet, then clear selection · `⌘↵` send · `⇧↵` newline · `⌘F` find in transcript · `[` `]` previous/next citation · `⌘E` export run.

---

## 8. Accessibility

Contrast: body text `#D6D6D6` on `#0A0A0A` is 13.1:1; `--text-dim` on `--surface` is 5.4:1 — above AA for the 12 px label sizes it is used at. `--text-faint` is used only for non-essential timestamps, which are also in the `title` attribute. Signal colours are never the sole carrier of meaning: blocked also has an icon and the word, degraded also has ▲, sealed also has ⛨.

Focus is always visible: 2 px `--focus` ring at 2 px offset, on every interactive element, never removed on mouse users.

Streaming text lives in an `aria-live="polite"` region that announces at **sentence** granularity, not per character — per-character live regions make screen readers unusable. Plan and tool events announce once on state change with a full sentence: "Step 2 of 3, extract wall thickness readings, complete, 4.1 seconds."

The constellation has a full keyboard equivalent — it is the Ledger tab, which is not a fallback but a peer view with the same selection model and the same lasso equivalent (shift-select rows → same floating bar).

Reduced motion, reduced transparency, and forced-colors modes all supported; forced-colors maps signal tokens to system semantic colours.

Hindi and Tamil UI strings are in scope for the i18n phase — the type scale reserves 22 px line height at 14 px base specifically so Devanagari matras do not clip, and the font stack includes a Noto subset.

## 9. Edge cases

| Situation | Behaviour |
|---|---|
| No models found | Setup step 2 empty state naming the exact files needed and the offline copy path. Cannot proceed — but says why, and how. |
| Every model degraded | Proceed with the warning strip permanently visible in the top bar until dismissed per-session. Never block. |
| OCR returns nothing | "No text found in this scan" plus the render, plus a manual region-select tool. Not an error toast. |
| Tool times out | Row goes `failed` with the elapsed time; plan offers Retry / Skip / Replan; the answer continues without that evidence and says so. |
| Context at cap mid-turn | Compact immediately, show the divider, continue. Never truncate silently. |
| Sealed write attempt | `guard.blocked` renders a `--blocked` row in the trace and a proposal card in Approvals. The agent's own text explains it, because the prompt requires that. |
| 50k-node graph | Canvas LOD: below 0.4 zoom, edges hide and nodes cluster by department; labels appear above 0.8 zoom only. |
| Session restored after crash | Boot, then a strip: "Restored from 14:07. The last turn did not complete." Partial answer retained, marked. |
| Very long answer (>8k words) | Virtualise committed messages; keep the streaming message live. |
| Clock skew / offline for months | Nothing depends on wall-clock validity. Certificate-free by construction. |

---

## 10. Build order

Fits alongside the seven-day backend plan rather than competing with it.

| Day | Backend (existing plan) | Interface |
|---|---|---|
| 1 | build unblock, hardware probe | Tokens, shell, rail, palette. Event contract §2.2 agreed and stubbed with a fixture player so UI work never blocks on Rust. |
| 2 | model registry, A1 | Setup steps 1–2, model table with verbatim fit warnings. Model chip. |
| 3 | sandbox, A3/A5 | Sovereignty screen, probe box, egress log. Setup step 3. |
| 4 | multimodal ingest, A4 | Sources reader, OCR overlay, confidence encoding. Stream renderer §6.1. |
| 5 | agentic path + citations, A2 | Plan card, tool rows, citation chips → box. Work screen complete. |
| 6 | egress proof, recompute | Context ledger + compaction divider. Trace screen. |
| 7 | integration, rehearsal | Memory constellation (ship Ledger tab first; Constellation is the one thing safe to cut). Boot animation. Appearance panel. |

**Cut order if you slip:** Timeline view → Constellation (Ledger covers the job) → Appearance panel beyond dark/light → boot animation (static mark). Never cut the fit warnings, the citation chips, or the per-tool egress badge — those three carry three of the five acceptance criteria into the demo.

---

## 11. Critique of this design

Written before anyone else says it.

**The trace will be noise by week two.** Three density modes is a mitigation, not a fix. Watch which mode people settle into after ten sessions; if it is Quiet, the default is wrong and the plan card should collapse automatically once a turn succeeds.

**The constellation is the most likely thing to be pretty and useless.** It survives only because of lasso-to-scope. If that interaction gets cut for time, cut the whole view with it and ship the Ledger — a force graph nobody acts on is decoration, and this document argued against decoration.

**Achromatic is a real constraint, not just a style.** Four signal colours in a grey field is a narrow vocabulary. The moment someone asks for a fifth state, the temptation will be a fifth hue. Resist it — use position and iconography instead, or the whole rationale collapses.

**Byte counts everywhere could read as anxiety.** Watch whether the context gauge makes people hoard context instead of working. If so, move the number behind a hover and show only the bar.

**Open questions**
1. Does the engineer want citations inline or as a footnote list? Assumed inline. Two paper mockups will settle it in a morning.
2. Should compaction ever be automatic, or always user-triggered with a nudge at 80%? Currently automatic — the safer default, possibly the wrong one for people who plan around what the system knows.
3. Is Tauri acceptable to the plant's IT for signing and deployment, or is the existing Electron client politically fixed? This decides the size claim.
4. Do departments want their own theme (a visible signal of which department's data you are in), or is that a compliance hazard? Genuinely unsure.
