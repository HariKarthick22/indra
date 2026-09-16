# INDRA Interface Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build every INDRA interface surface — eight screens, their components, their animations, and the skill/slash-command affordances — inside the existing Electron client.

**Architecture:** React components in `ui/desktop/src/components/indra-shell/`, styled with inline styles referencing the CSS custom properties in `indra-tokens.css` (the convention the already-merged shell components established — not Tailwind classes). All agent-derived UI state arrives as `IndraEvent` values already emitted by the Rust backend through `MessageMetadata` operation notes; the UI never polls and never invents state. Screens are pure presentation over that event stream plus ACP calls for configuration.

**Tech Stack:** React 19, TypeScript, Electron, ACP (`@agentclientprotocol/sdk` + generated `@aaif/goose-acp-client`), CSS custom properties, Canvas 2D for the constellation.

**Spec:** [`INDRA-system-and-ui-design.md`](../../../INDRA-system-and-ui-design.md) — read it before any task. This plan implements it; where they disagree, the spec wins except for the two documented deviations below.

---

## Global Constraints

Copied from the spec. Every task's requirements implicitly include this section.

- **Electron, not Tauri.** Spec §0 decision 6 recommends Tauri v2; that migration is explicitly **out of scope**. Build inside `ui/desktop/`. The spec's ~9 MB client size claim does not hold here — do not repeat it.
- **Achromatic.** The only chroma in the product is `--sealed`, `--degraded`, `--blocked`, `--focus`. No other colour, ever.
- **Signal colours never fill an area larger than 2px.** A left border, a dot, a rule, or text. Never a panel background. (§3.2)
- **`--focus` appears on exactly two things:** the focus ring and the text caret. Never a fill, never a button. (§3.2)
- **Fit warnings render verbatim.** Never paraphrased, never truncated, never behind a disclosure. (§2.2, §6.2)
- **Type:** Geist Sans / Geist Mono. Base `--t-14`. Weights 400/500/600 only — nothing heavier. Tabular numerals everywhere numbers change. (§3.3)
- **Space scale, 4px base:** `2 4 6 8 12 16 24 32 48 64`. Nothing between. (§3.4)
- **Radius carries hierarchy:** `--r-sm` 3px inputs/chips/tool rows · `--r-md` 6px panels/cards/dialogs · `--r-lg` 10px composer only · `--r-full` avatars/dots. (§3.4)
- **Exactly two shadows exist** (`--shadow-pop`, on the palette and dialogs). Cards do not float. Borders do the work shadows do elsewhere. (§3.4)
- **No section entrance animations, no card hover lifts.** Everything except boot is a response to a user action. (§3.5)
- **`prefers-reduced-motion: reduce` is honoured in every task that animates.** Boot becomes a static mark with a checklist; stream reveal becomes instant per word; panels cut instead of slide. (§3.5)
- **Avoid:** all-caps tracked labels, single accented words in headings, `→` glued onto button text. (§3.3)
- **Styling convention:** inline `style={{}}` referencing `var(--token)`, matching `IndraRail.tsx`/`IndraTopBar.tsx`. Do not introduce Tailwind classes into `indra-shell/`.
- **Focus ring:** 2px `--focus` at 2px offset on every interactive element. Never removed for mouse users. (§8)
- **Accessibility:** signal colour is never the sole carrier of meaning — blocked also has an icon and the word, degraded also has ▲, sealed also has ⛨. (§8)
- **Tests:** Vitest + Testing Library, colocated as `*.test.tsx`, matching `IndraShell.test.tsx`.
- **Run `cd ui/desktop && pnpm run typecheck` before every commit.** Never commit type errors.

## Documented deviations from the spec

Two, both deliberate, both approved:

1. **Electron over Tauri** — as above.
2. **Models arrive two ways, not one.** Spec §5.2 Step 2 shows only a directory scan of local GGUF files. A company model server reached over the LAN has no file size and no local path — it has an endpoint and a reachability state. The model table therefore supports both row kinds as peers (Task 8). This extends the spec rather than contradicting it, and follows the backend's `Backend::LocalHttp` variant, whose `validate_backend` already permits private LAN addresses.

## Current state — verified, not assumed

Already built and merged; **do not rebuild**:

- `ui/desktop/src/styles/indra-tokens.css` — all colour, type, space, radius, motion tokens. Ash Dark on `:root`, Chalk Light on `.light`.
- `ui/desktop/src/components/indra-shell/IndraShell.tsx` — flex shell, 78ch centred main. Props: `active`, `onSelect`, `sessionTitle`, `onSessionTitleChange`, `sessionBytes`, `budgetBytes`, `onToggleTheme`, `children`.
- `IndraRail.tsx` — 48px rail, five destinations, `IndraRailDestination = 'work'|'memory'|'sources'|'trace'|'sovereignty'`. Icons are explicitly placeholder art.
- `IndraTopBar.tsx` — 36px bar. Props: `sessionTitle`, `onSessionTitleChange`, `sessionBytes`, `budgetBytes`, `onToggleTheme`.
- `crates/indra/src/events/types.rs` — `IndraEvent` Rust enum, `#[serde(tag = "t")]`, variants `plan.proposed`, `plan.revised`, `step.state`, `model.selected`, `tool.call`, `tool.result`, `citation`, `context.delta`, `context.compacted`, `memory.touch`, `egress.attempt`, `verify.recompute`, `guard.blocked`.
- `crates/indra/src/events/mod.rs` — `emit(metadata, event)` writes the event to operation note `("indra", "indra_event")`.
- `agent.rs` and `state_machine/ops_llm.rs` both emit `ModelSelected`.
- `crates/indra/src/slash_commands/skill_slash_command.rs` — `list_commands(working_dir)`, `resolve_command(...)`. Skills already become slash commands; the UI presents them, it does not register them.

**Known blocker:** the shell components are standalone and **unwired** — commit `540e6e6` states they are not integrated into app routing because of pre-existing broken imports from an incomplete goose→indra rename. Task 1 resolves this before anything else.

**Gaps in the Rust event enum** versus spec §2.2, relevant to later tasks: no `boot.step`, `turn.start`, `text.delta`, `turn.end` variants, and `ModelSelected.fit` is `serde_json::Value` rather than a typed `{kind, warning}`. Tasks 2 and 12 handle this.

---

## Phases

| Phase | Tasks | Delivers | Safe to cut? |
|---|---|---|---|
| A — Foundation | 1–3 | Shell wired into routing, typed event stream in TS, reduced-motion hook | No — everything depends on it |
| B — Work screen | 4–7 | Stream renderer, plan card, tool rows, citation chips | No — carries A2 and A4 |
| C — Models | 8–9 | Model table (both row kinds), model switch chip | No — carries A1 |
| D — Sovereignty | 10–11 | Sovereignty screen, probe box, per-tool egress badge | **Never cut** — carries A5 |
| E — Context | 12–13 | Context gauge, ledger sheet, compaction divider | Yes, last resort |
| F — Sources | 14–15 | Document library, OCR overlay reader | No — carries A4's visible half |
| G — Trace | 16 | Run history, replay, approvals queue | Yes |
| H — Memory | 17–19 | Ledger tab, constellation, timeline | **Cut in this order:** Timeline → Constellation → keep Ledger |
| I — Boot & setup | 20–22 | Boot animation, three-step setup | Boot → static mark if slipping |
| J — Palette & skills | 23–24 | ⌘K palette, skill/slash-command surfacing | No — this is how skills get used |
| K — Appearance | 25 | Theme panel, live preview | Yes, beyond dark/light toggle |

**Cut order if you slip** (spec §10): Timeline → Constellation → Appearance beyond dark/light → boot animation. **Never cut** fit warnings, citation chips, or the per-tool egress badge — those three carry three of the five acceptance criteria.

---

## Phase A — Foundation

### Task 1: Wire the shell into app routing

**Files:**
- Modify: `ui/desktop/src/App.tsx`
- Create: `ui/desktop/src/components/indra-shell/IndraWorkspace.tsx`
- Test: `ui/desktop/src/components/indra-shell/IndraWorkspace.test.tsx`

**Interfaces:**
- Consumes: `IndraShell`, `IndraRailDestination` from `./index`.
- Produces: `IndraWorkspace` — holds the active destination and session title state, renders `IndraShell` around a destination switch. Later screen tasks mount into its `renderDestination` switch.

**Before you start:** commit `540e6e6` says routing integration was blocked by broken imports from the incomplete rename. Find out whether that is still true before changing anything:

```bash
cd ui/desktop && pnpm run typecheck 2>&1 | head -40
```

If it reports errors in files you are not touching, fix only those that block importing `indra-shell` into `App.tsx`. Leave unrelated pre-existing errors alone and note them in the commit message.

- [ ] **Step 1: Write the failing test**

```tsx
// ui/desktop/src/components/indra-shell/IndraWorkspace.test.tsx
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { IndraWorkspace } from './IndraWorkspace';

describe('IndraWorkspace', () => {
  it('starts on Work and switches destination when a rail item is clicked', async () => {
    const user = userEvent.setup();
    render(<IndraWorkspace />);

    expect(screen.getByRole('button', { name: 'Work' })).toHaveAttribute('aria-current', 'page');

    await user.click(screen.getByRole('button', { name: 'Sovereignty' }));

    expect(screen.getByRole('button', { name: 'Sovereignty' })).toHaveAttribute(
      'aria-current',
      'page'
    );
    expect(screen.getByRole('button', { name: 'Work' })).not.toHaveAttribute('aria-current');
  });
});
```

- [ ] **Step 2: Run it and watch it fail**

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/IndraWorkspace.test.tsx
```
Expected: FAIL — cannot resolve `./IndraWorkspace`.

- [ ] **Step 3: Implement**

```tsx
// ui/desktop/src/components/indra-shell/IndraWorkspace.tsx
import { useState } from 'react';
import { IndraShell } from './IndraShell';
import type { IndraRailDestination } from './IndraRail';

const DESTINATION_TITLES: Record<IndraRailDestination, string> = {
  work: 'Work',
  memory: 'Memory',
  sources: 'Sources',
  trace: 'Trace',
  sovereignty: 'Sovereignty',
};

export function IndraWorkspace() {
  const [active, setActive] = useState<IndraRailDestination>('work');
  const [sessionTitle, setSessionTitle] = useState('New session');

  const toggleTheme = () => {
    document.documentElement.classList.toggle('light');
  };

  return (
    <IndraShell
      active={active}
      onSelect={setActive}
      sessionTitle={sessionTitle}
      onSessionTitleChange={setSessionTitle}
      sessionBytes={0}
      budgetBytes={32768}
      onToggleTheme={toggleTheme}
    >
      <h1 style={{ fontSize: 'var(--t-20)', fontWeight: 600, color: 'var(--text-hi)' }}>
        {DESTINATION_TITLES[active]}
      </h1>
    </IndraShell>
  );
}
```

Export it from `index.ts`:

```ts
export { IndraWorkspace } from './IndraWorkspace';
```

- [ ] **Step 4: Run and confirm pass**

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/IndraWorkspace.test.tsx && pnpm run typecheck
```
Expected: 1 passed, typecheck clean.

- [ ] **Step 5: Commit**

```bash
git add ui/desktop/src/components/indra-shell/
git commit -m "feat(ui): wire INDRA shell into a routable workspace"
```

---

### Task 2: Typed event stream in TypeScript

**Files:**
- Create: `ui/desktop/src/indra/events.ts`
- Create: `ui/desktop/src/indra/useIndraEvents.ts`
- Test: `ui/desktop/src/indra/events.test.ts`

**Interfaces:**
- Consumes: operation notes on message metadata, namespace `"indra"`, key `"indra_event"` (written by `crates/indra/src/events/mod.rs`).
- Produces:
  - `type IndraEvent` — discriminated union on `t`, mirroring the Rust enum.
  - `parseIndraEvent(value: unknown): IndraEvent | null` — returns `null` for anything unrecognised rather than throwing.
  - `useIndraEvents(messages): IndraEvent[]` — extracts events in order from a message list.

Every later phase consumes these. The union must mirror the **Rust** enum (the source of truth), not the spec's TS sketch, where they differ.

- [ ] **Step 1: Write the failing test**

```ts
// ui/desktop/src/indra/events.test.ts
import { describe, expect, it } from 'vitest';
import { parseIndraEvent } from './events';

describe('parseIndraEvent', () => {
  it('parses a model.selected event with a degraded fit', () => {
    const event = parseIndraEvent({
      t: 'model.selected',
      model_id: 'llama-3.2-vision-11b',
      reason: 'vision required',
      fit: { kind: 'degraded', warning: '6800 MB needed, 5200 MB available' },
    });

    expect(event).toEqual({
      t: 'model.selected',
      model_id: 'llama-3.2-vision-11b',
      reason: 'vision required',
      fit: { kind: 'degraded', warning: '6800 MB needed, 5200 MB available' },
    });
  });

  it('parses an egress.attempt event', () => {
    const event = parseIndraEvent({
      t: 'egress.attempt',
      url: 'https://example.com',
      blocked: true,
      at: '2026-09-16T14:02:31Z',
    });
    expect(event?.t).toBe('egress.attempt');
  });

  it('returns null for an unknown tag rather than throwing', () => {
    expect(parseIndraEvent({ t: 'not.a.real.event' })).toBeNull();
  });

  it('returns null for a non-object', () => {
    expect(parseIndraEvent('nope')).toBeNull();
    expect(parseIndraEvent(null)).toBeNull();
  });
});
```

- [ ] **Step 2: Run it and watch it fail**

```bash
cd ui/desktop && pnpm vitest run src/indra/events.test.ts
```
Expected: FAIL — cannot resolve `./events`.

- [ ] **Step 3: Implement the union**

```ts
// ui/desktop/src/indra/events.ts
export interface SourceBox {
  doc_id: string;
  version: string;
  page: number;
  bbox: [number, number, number, number];
  conf: number;
}

export interface PlanStep {
  id: string;
  label: string;
  tool: string | null;
}

export type FitKind = 'comfortable' | 'tight' | 'degraded';

export interface Fit {
  kind: FitKind;
  warning?: string;
}

export type IndraEvent =
  | { t: 'plan.proposed'; steps: PlanStep[] }
  | { t: 'plan.revised'; after_step: string; steps: PlanStep[]; reason: string }
  | { t: 'step.state'; step_id: string; state: string; ms?: number }
  | { t: 'model.selected'; model_id: string; reason: string; fit: Fit }
  | {
      t: 'tool.call';
      call_id: string;
      step_id: string;
      name: string;
      args: unknown;
      sandbox_backend: string;
    }
  | {
      t: 'tool.result';
      call_id: string;
      ok: boolean;
      ms: number;
      summary: string;
      bytes: number;
      citations: SourceBox[];
    }
  | { t: 'citation'; span: [number, number]; source: SourceBox }
  | { t: 'context.delta'; session_bytes: number; budget_bytes: number }
  | { t: 'context.compacted'; from_turns: number; to_bytes: number; dropped_count: number }
  | { t: 'memory.touch'; node_ids: string[]; op: string }
  | { t: 'egress.attempt'; url: string; blocked: boolean; at: string }
  | { t: 'verify.recompute'; claim: number; computed: number; verdict: string }
  | { t: 'guard.blocked'; resource: string; reason: string };

const KNOWN_TAGS: ReadonlySet<string> = new Set([
  'plan.proposed',
  'plan.revised',
  'step.state',
  'model.selected',
  'tool.call',
  'tool.result',
  'citation',
  'context.delta',
  'context.compacted',
  'memory.touch',
  'egress.attempt',
  'verify.recompute',
  'guard.blocked',
]);

// An unrecognised tag means the Rust enum gained a variant this build does not
// know about. Dropping it keeps an older UI usable against a newer backend.
export function parseIndraEvent(value: unknown): IndraEvent | null {
  if (typeof value !== 'object' || value === null) return null;
  const tag = (value as { t?: unknown }).t;
  if (typeof tag !== 'string' || !KNOWN_TAGS.has(tag)) return null;
  return value as IndraEvent;
}
```

- [ ] **Step 4: Run and confirm pass**

```bash
cd ui/desktop && pnpm vitest run src/indra/events.test.ts && pnpm run typecheck
```
Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add ui/desktop/src/indra/
git commit -m "feat(ui): add typed IndraEvent union and parser"
```

- [ ] **Step 6: Add the extraction hook**

Find how operation notes are read in the existing UI before writing this — `ToolCallWithResponse.tsx` and `IndraMessage.tsx` both read `operation` metadata already:

```bash
grep -n "operation" ui/desktop/src/components/IndraMessage.tsx | head
```

Write `useIndraEvents.ts` to walk a message list, read the `("indra", "indra_event")` note off each message's metadata using the same accessor those components use, run each through `parseIndraEvent`, and return the non-null results in order. Test it with two messages carrying one event each, asserting both come back in order.

- [ ] **Step 7: Commit**

```bash
cd ui/desktop && pnpm run typecheck
git add ui/desktop/src/indra/
git commit -m "feat(ui): extract IndraEvents from message metadata"
```

---

### Task 3: Reduced-motion hook

**Files:**
- Create: `ui/desktop/src/indra/useReducedMotion.ts`
- Test: `ui/desktop/src/indra/useReducedMotion.test.ts`

**Interfaces:**
- Produces: `useReducedMotion(): boolean`. Every animating task (5, 6, 9, 13, 18, 20) consumes it.

Build this once, here, so no later task reimplements it or silently skips the requirement.

- [ ] **Step 1: Write the failing test**

```ts
// ui/desktop/src/indra/useReducedMotion.test.ts
import { renderHook } from '@testing-library/react';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { useReducedMotion } from './useReducedMotion';

function mockMatchMedia(matches: boolean) {
  vi.stubGlobal(
    'matchMedia',
    vi.fn().mockImplementation((query: string) => ({
      matches,
      media: query,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    }))
  );
}

describe('useReducedMotion', () => {
  beforeEach(() => vi.unstubAllGlobals());

  it('is true when the user asked for reduced motion', () => {
    mockMatchMedia(true);
    expect(renderHook(() => useReducedMotion()).result.current).toBe(true);
  });

  it('is false otherwise', () => {
    mockMatchMedia(false);
    expect(renderHook(() => useReducedMotion()).result.current).toBe(false);
  });
});
```

- [ ] **Step 2: Run and watch it fail**

```bash
cd ui/desktop && pnpm vitest run src/indra/useReducedMotion.test.ts
```

- [ ] **Step 3: Implement**

```ts
// ui/desktop/src/indra/useReducedMotion.ts
import { useEffect, useState } from 'react';

const QUERY = '(prefers-reduced-motion: reduce)';

export function useReducedMotion(): boolean {
  const [reduced, setReduced] = useState(() => window.matchMedia(QUERY).matches);

  useEffect(() => {
    const mq = window.matchMedia(QUERY);
    const onChange = (e: MediaQueryListEvent) => setReduced(e.matches);
    mq.addEventListener('change', onChange);
    return () => mq.removeEventListener('change', onChange);
  }, []);

  return reduced;
}
```

- [ ] **Step 4: Run, typecheck, commit**

```bash
cd ui/desktop && pnpm vitest run src/indra/useReducedMotion.test.ts && pnpm run typecheck
git add ui/desktop/src/indra/
git commit -m "feat(ui): add reduced-motion hook"
```

---

## Phase B — Work screen

### Task 4: Stream renderer with jitter buffer

**Files:**
- Create: `ui/desktop/src/indra/stream/useJitterBuffer.ts`
- Create: `ui/desktop/src/components/indra-shell/StreamText.tsx`
- Test: `ui/desktop/src/indra/stream/useJitterBuffer.test.ts`

**Interfaces:**
- Consumes: `useReducedMotion` (Task 3).
- Produces: `useJitterBuffer({ text, speed }): { committed: string; animating: string }` and `<StreamText text speed />`.

This is spec §6.1 and it is the single most visible piece of craft in the product. Read §6.1 in full first. The rules that matter and are easy to get wrong:

- Drain rate: `target = clamp(base + (queue.length - 60) * 1.4, base * 0.6, 400)` chars/sec, `base = 65` (Calm 45 · Normal 65 · Instant ∞).
- Reveal unit is the **grapheme cluster**, never the code unit — use `Intl.Segmenter`. Never split an emoji or a Devanagari conjunct.
- A word never sits half-drawn at rest: if the queue empties mid-word, commit the rest of that word immediately.
- Entrance is **opacity and blur only**. No `translateY` — vertical motion forces a line repaint and produces exactly the jitter this removes.
- Only the ~120 still-animating characters are wrapped in spans; each unwraps to plain text when its animation ends. A 4,000-word answer must not be 20,000 live DOM nodes.
- Stall > 900 ms swaps the caret for a three-dot breathing indicator.
- Reduced motion → commit per word, instantly, no blur.

- [ ] **Step 1: Write the failing tests**

```ts
// ui/desktop/src/indra/stream/useJitterBuffer.test.ts
import { describe, expect, it } from 'vitest';
import { drainRate, segmentGraphemes, completeWord } from './useJitterBuffer';

describe('drainRate', () => {
  it('sits at base when the queue is small', () => {
    expect(drainRate(65, 60)).toBeCloseTo(65);
  });

  it('accelerates as the queue grows', () => {
    expect(drainRate(65, 160)).toBeCloseTo(205);
  });

  it('never exceeds 400 chars/sec', () => {
    expect(drainRate(65, 100_000)).toBe(400);
  });

  it('never drops below 60% of base', () => {
    expect(drainRate(65, 0)).toBeCloseTo(39);
  });
});

describe('segmentGraphemes', () => {
  it('keeps an emoji whole', () => {
    expect(segmentGraphemes('a👍b')).toEqual(['a', '👍', 'b']);
  });

  it('keeps a Devanagari conjunct whole', () => {
    expect(segmentGraphemes('क्ष')).toEqual(['क्ष']);
  });
});

describe('completeWord', () => {
  it('extends to the end of the current word', () => {
    expect(completeWord('hello wor', 'ld and more')).toBe('ld');
  });

  it('returns empty when already at a boundary', () => {
    expect(completeWord('hello ', 'world')).toBe('');
  });
});
```

- [ ] **Step 2: Run and watch it fail**

```bash
cd ui/desktop && pnpm vitest run src/indra/stream/useJitterBuffer.test.ts
```

- [ ] **Step 3: Implement the pure functions**

```ts
// ui/desktop/src/indra/stream/useJitterBuffer.ts
const MAX_RATE = 400;

export function drainRate(base: number, queueLength: number): number {
  const target = base + (queueLength - 60) * 1.4;
  return Math.min(Math.max(target, base * 0.6), MAX_RATE);
}

const segmenter = new Intl.Segmenter(undefined, { granularity: 'grapheme' });

export function segmentGraphemes(text: string): string[] {
  return Array.from(segmenter.segment(text), (s) => s.segment);
}

// When the queue empties mid-word, the remaining characters of that word commit
// immediately — a word half-drawn at rest reads as a rendering bug.
export function completeWord(committed: string, remaining: string): string {
  if (committed.length === 0 || /\s$/.test(committed)) return '';
  const boundary = remaining.search(/\s/);
  return boundary === -1 ? remaining : remaining.slice(0, boundary);
}
```

- [ ] **Step 4: Run and confirm pass**

```bash
cd ui/desktop && pnpm vitest run src/indra/stream/useJitterBuffer.test.ts
```
Expected: 8 passed.

- [ ] **Step 5: Commit**

```bash
git add ui/desktop/src/indra/stream/
git commit -m "feat(ui): add stream jitter-buffer primitives"
```

- [ ] **Step 6: Build the rAF loop and StreamText**

Add `useJitterBuffer({ text, speed })` to the same file: hold a grapheme queue, drain on `requestAnimationFrame` at `drainRate(...)`, apply `completeWord` when the queue empties, and return `{ committed, animating }`. Honour `useReducedMotion` by committing whole words with no animation. Cancel the frame on unmount.

Then `StreamText.tsx` renders `committed` as plain text and wraps only `animating` graphemes in `<span className="g">`, each removing its own span on `animationend`. Add to `indra-tokens.css`:

```css
@keyframes glyph-in {
  from { opacity: 0; filter: blur(2.5px); }
  to   { opacity: 1; filter: blur(0); }
}
.g { animation: glyph-in var(--m-glyph) linear both; }
@media (prefers-reduced-motion: reduce) {
  .g { animation: none; }
}
```

Caret: 2px block, `--text-hi`, riding the last committed glyph — solid while streaming, 1.06s blink when idle, hidden when the turn ends.

- [ ] **Step 7: Commit**

```bash
cd ui/desktop && pnpm run typecheck
git add ui/desktop/src/indra/stream/ ui/desktop/src/components/indra-shell/StreamText.tsx ui/desktop/src/styles/indra-tokens.css
git commit -m "feat(ui): add per-grapheme stream renderer"
```

---

### Task 5: Plan card

**Files:**
- Create: `ui/desktop/src/components/indra-shell/PlanCard.tsx`
- Test: `ui/desktop/src/components/indra-shell/PlanCard.test.tsx`

**Interfaces:**
- Consumes: `PlanStep`, `IndraEvent` (Task 2); `useReducedMotion` (Task 3).
- Produces: `<PlanCard steps stepStates revision />` where `stepStates: Record<string, {state: string; ms?: number}>`.

Spec §6.2. Step states: `queued` (dim, hollow dot) · `running` (dot fills, label to `--text`) · `done` (✓, duration in mono) · `skipped` (struck, reason on hover) · `failed` (`--blocked` left rule).

**Replanning is shown, not hidden** — this is the point of the component. On `plan.revised`, new steps insert with a 240 ms height animation and a `--focus` left tick that fades over 2 s; superseded steps strike through and dim rather than vanishing.

- [ ] **Step 1: Write the failing test**

```tsx
// ui/desktop/src/components/indra-shell/PlanCard.test.tsx
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { PlanCard } from './PlanCard';

const steps = [
  { id: 's1', label: 'Locate August NDT report', tool: 'read' },
  { id: 's2', label: 'Extract wall-thickness readings', tool: 'ocr' },
];

describe('PlanCard', () => {
  it('shows each step and the step count', () => {
    render(<PlanCard steps={steps} stepStates={{}} />);
    expect(screen.getByText('Locate August NDT report')).toBeInTheDocument();
    expect(screen.getByText(/2 steps/)).toBeInTheDocument();
  });

  it('shows a duration in monospace for a completed step', () => {
    render(<PlanCard steps={steps} stepStates={{ s1: { state: 'done', ms: 300 } }} />);
    expect(screen.getByText('0.3s')).toBeInTheDocument();
  });

  it('keeps a superseded step visible rather than removing it', () => {
    render(
      <PlanCard
        steps={steps}
        stepStates={{ s1: { state: 'skipped' } }}
        supersededIds={['s1']}
      />
    );
    const step = screen.getByText('Locate August NDT report');
    expect(step).toBeInTheDocument();
    expect(step).toHaveStyle({ textDecoration: 'line-through' });
  });
});
```

- [ ] **Step 2: Run and watch it fail**

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/PlanCard.test.tsx
```

- [ ] **Step 3: Implement**

Render a header (`▣ Plan · N steps`, chevron) and one row per step. Row height 32px collapsed. Dot is `--r-full`, 6px: hollow `--line-strong` when queued, filled `--text-dim` when running, replaced by ✓ when done. Duration right-aligned, `--font-mono`, `--t-11`, `font-variant-numeric: tabular-nums`. Failed rows get a 2px `--blocked` left rule — never a red fill. Superseded rows get `textDecoration: 'line-through'` and `color: var(--text-faint)`.

- [ ] **Step 4: Run, typecheck, commit**

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/PlanCard.test.tsx && pnpm run typecheck
git add ui/desktop/src/components/indra-shell/PlanCard.tsx ui/desktop/src/components/indra-shell/PlanCard.test.tsx
git commit -m "feat(ui): add plan card with visible replanning"
```

---

### Task 6: Tool row with the egress badge

**Files:**
- Create: `ui/desktop/src/components/indra-shell/ToolRow.tsx`
- Test: `ui/desktop/src/components/indra-shell/ToolRow.test.tsx`

**Interfaces:**
- Consumes: `IndraEvent` `tool.call` / `tool.result` (Task 2).
- Produces: `<ToolRow call result expanded onToggle />`.

Spec §6.2. **The sandbox/egress badge on every tool row is the INDRA-specific move** — it turns the sovereignty claim from a page you visit once into a fact repeated on every action. This is one of the three things the spec says never to cut.

Collapsed, one line: `⟶ ocr_document  NDT_2026_08.pdf   38 regions · sandboxed, no network   4.1s  ⌄`
Expanded: folded JSON arguments (copyable), result summary, bytes returned, sandbox backend, egress state, citations produced.

- [ ] **Step 1: Write the failing test**

```tsx
// ui/desktop/src/components/indra-shell/ToolRow.test.tsx
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { ToolRow } from './ToolRow';

const call = {
  t: 'tool.call' as const,
  call_id: 'c1',
  step_id: 's1',
  name: 'ocr_document',
  args: { path: 'NDT_2026_08.pdf' },
  sandbox_backend: 'docker',
};

const result = {
  t: 'tool.result' as const,
  call_id: 'c1',
  ok: true,
  ms: 4100,
  summary: '38 regions',
  bytes: 8192,
  citations: [],
};

describe('ToolRow', () => {
  it('always states the sandbox and network posture', () => {
    render(<ToolRow call={call} result={result} expanded={false} onToggle={vi.fn()} />);
    expect(screen.getByText(/sandboxed, no network/)).toBeInTheDocument();
  });

  it('shows the tool name and duration', () => {
    render(<ToolRow call={call} result={result} expanded={false} onToggle={vi.fn()} />);
    expect(screen.getByText('ocr_document')).toBeInTheDocument();
    expect(screen.getByText('4.1s')).toBeInTheDocument();
  });

  it('reveals arguments only when expanded', async () => {
    const onToggle = vi.fn();
    const { rerender } = render(
      <ToolRow call={call} result={result} expanded={false} onToggle={onToggle} />
    );
    expect(screen.queryByText(/NDT_2026_08\.pdf/)).not.toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: /ocr_document/ }));
    expect(onToggle).toHaveBeenCalledOnce();

    rerender(<ToolRow call={call} result={result} expanded onToggle={onToggle} />);
    expect(screen.getByText(/NDT_2026_08\.pdf/)).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run and watch it fail**

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/ToolRow.test.tsx
```

- [ ] **Step 3: Implement**

Collapsed height 32px, `--r-sm`, background `--raised`. Chevron rotates over `--m-ui`. The badge text is derived from `sandbox_backend`: anything other than `'none'` renders `sandboxed, no network` with a `--sealed` dot; `'none'` renders `not sandboxed` with a `--degraded` dot. Failed results get a 2px `--blocked` left rule and the word `failed` — never colour alone.

- [ ] **Step 4: Run, typecheck, commit**

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/ToolRow.test.tsx && pnpm run typecheck
git add ui/desktop/src/components/indra-shell/ToolRow.tsx ui/desktop/src/components/indra-shell/ToolRow.test.tsx
git commit -m "feat(ui): add tool row with per-call sandbox and egress badge"
```

---

### Task 7: Citation chip and source preview

**Files:**
- Create: `ui/desktop/src/components/indra-shell/CitationChip.tsx`
- Test: `ui/desktop/src/components/indra-shell/CitationChip.test.tsx`

**Interfaces:**
- Consumes: `SourceBox` (Task 2).
- Produces: `<CitationChip index source onOpen />`.

Spec §6.4. Inline `⟦1⟧`, `--text-dim` at rest, `--text-hi` with a `--focus` underline on hover. Hover after **220 ms** → floating 240px crop of the exact bbox with page number and OCR confidence. Click → opens the Sources reader (Task 15) on that page.

Confidence below **0.60**: chip carries a small `--degraded` dot and the preview says so. The sentence itself inherits nothing — the model was instructed to hedge in words rather than the UI hedging on its behalf.

- [ ] **Step 1: Write the failing test**

```tsx
// ui/desktop/src/components/indra-shell/CitationChip.test.tsx
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { CitationChip } from './CitationChip';

const source = {
  doc_id: 'NDT_2026_08.pdf',
  version: 'v3',
  page: 4,
  bbox: [100, 220, 400, 24] as [number, number, number, number],
  conf: 0.91,
};

describe('CitationChip', () => {
  it('renders the bracketed index', () => {
    render(<CitationChip index={1} source={source} onOpen={vi.fn()} />);
    expect(screen.getByRole('button', { name: /citation 1/i })).toHaveTextContent('⟦1⟧');
  });

  it('marks a low-confidence citation', () => {
    render(<CitationChip index={2} source={{ ...source, conf: 0.42 }} onOpen={vi.fn()} />);
    expect(screen.getByLabelText(/low confidence/i)).toBeInTheDocument();
  });

  it('does not mark a confident citation', () => {
    render(<CitationChip index={3} source={source} onOpen={vi.fn()} />);
    expect(screen.queryByLabelText(/low confidence/i)).not.toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run and watch it fail**

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/CitationChip.test.tsx
```

- [ ] **Step 3: Implement**

Baseline-aligned, never taller than the line box. `aria-label` reads `Citation {index}, page {page}` plus `, low confidence` under 0.60. The low-confidence dot is 4px `--r-full` `--degraded` with its own `aria-label`, so colour is not the sole carrier. Hover preview appears after 220 ms via `setTimeout`, cleared on mouse leave; fade + 4px rise over `--m-ui`, suppressed under reduced motion.

- [ ] **Step 4: Run, typecheck, commit**

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/CitationChip.test.tsx && pnpm run typecheck
git add ui/desktop/src/components/indra-shell/CitationChip.tsx ui/desktop/src/components/indra-shell/CitationChip.test.tsx
git commit -m "feat(ui): add inline citation chip with confidence marking"
```

---

## Phase C — Models

### Task 8: Model table with both acquisition paths

**Files:**
- Create: `ui/desktop/src/components/indra-shell/ModelTable.tsx`
- Test: `ui/desktop/src/components/indra-shell/ModelTable.test.tsx`

**Interfaces:**
- Produces:
  ```ts
  export type ModelSource =
    | { kind: 'local'; path: string; bytes: number }
    | { kind: 'served'; endpoint: string; reachable: boolean };

  export interface ModelRow {
    id: string;
    source: ModelSource;
    vision: boolean;
    toolCalling: 'reliable' | 'unreliable' | 'none';
    contextTokens: number;
    fit: Fit;              // from Task 2
  }
  ```
  `<ModelTable rows />`.

This is the deviation from spec §5.2 Step 2, and the reason for it: a model served from `10.4.2.15:8000` has no file size and no local path. Columns: `MODEL · SOURCE · VISION · TOOLS · CONTEXT · FIT`. Local rows render size in the SOURCE cell; served rows render the endpoint and a reachability dot (`--sealed` reachable, `--blocked` unreachable, each with a word, never colour alone).

**The fit warning renders verbatim, wrapped across lines, never truncated and never behind a disclosure.** This is a global constraint and this table is where it matters most.

- [ ] **Step 1: Write the failing test**

```tsx
// ui/desktop/src/components/indra-shell/ModelTable.test.tsx
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { ModelTable, type ModelRow } from './ModelTable';

const localRow: ModelRow = {
  id: 'qwen2.5-coder-7b-q4',
  source: { kind: 'local', path: '/models/qwen.gguf', bytes: 4_100_000_000 },
  vision: false,
  toolCalling: 'reliable',
  contextTokens: 32768,
  fit: { kind: 'comfortable' },
};

const servedRow: ModelRow = {
  id: 'plant-llama-70b',
  source: { kind: 'served', endpoint: 'http://10.4.2.15:8000/v1', reachable: true },
  vision: true,
  toolCalling: 'unreliable',
  contextTokens: 16384,
  fit: {
    kind: 'degraded',
    warning: '6800 MB needed, 5200 MB available — offloading to CPU, expect substantially slower generation',
  },
};

describe('ModelTable', () => {
  it('shows a size for a local model', () => {
    render(<ModelTable rows={[localRow]} />);
    expect(screen.getByText('4.1 GB')).toBeInTheDocument();
  });

  it('shows the endpoint for a served model instead of a size', () => {
    render(<ModelTable rows={[servedRow]} />);
    expect(screen.getByText('http://10.4.2.15:8000/v1')).toBeInTheDocument();
  });

  it('renders the fit warning verbatim and in full', () => {
    render(<ModelTable rows={[servedRow]} />);
    expect(
      screen.getByText(
        '6800 MB needed, 5200 MB available — offloading to CPU, expect substantially slower generation'
      )
    ).toBeInTheDocument();
  });

  it('states reachability in words, not only colour', () => {
    render(
      <ModelTable
        rows={[
          {
            ...servedRow,
            source: { kind: 'served', endpoint: 'http://10.4.2.16:8000/v1', reachable: false },
          },
        ]}
      />
    );
    expect(screen.getByText(/unreachable/i)).toBeInTheDocument();
  });

  it('names the next action when empty rather than dead-ending', () => {
    render(<ModelTable rows={[]} />);
    expect(screen.getByText(/add a model/i)).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run and watch it fail**

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/ModelTable.test.tsx
```

- [ ] **Step 3: Implement**

Header cells `--t-12` `--text-dim`, no all-caps tracking (global constraint). Body `--t-13`. Numbers tabular. Fit cell: `✓ comfortable` in `--text-dim`, or `▲` plus the warning string on a 2px `--degraded` left rule — the ▲ is required so colour is not the sole carrier. Empty state names the action ("Add a model — point at a directory of GGUF files, or a model server on your network"), never an illustration.

- [ ] **Step 4: Run, typecheck, commit**

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/ModelTable.test.tsx && pnpm run typecheck
git add ui/desktop/src/components/indra-shell/ModelTable.tsx ui/desktop/src/components/indra-shell/ModelTable.test.tsx
git commit -m "feat(ui): add model table supporting local and served models"
```

---

### Task 9: Model switch chip

**Files:**
- Create: `ui/desktop/src/components/indra-shell/ModelChip.tsx`
- Test: `ui/desktop/src/components/indra-shell/ModelChip.test.tsx`

**Interfaces:**
- Consumes: `model.selected` event and `Fit` (Task 2).
- Produces: `<ModelChip previousModelId modelId reason fit />`.

Spec §6.2: appears **only when the model differs from the previous turn** — `qwen-coder → llama-3.2-vision · vision required`. A degraded fit renders the warning verbatim below it on a `--degraded` left rule.

- [ ] **Step 1: Write the failing test**

```tsx
// ui/desktop/src/components/indra-shell/ModelChip.test.tsx
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { ModelChip } from './ModelChip';

describe('ModelChip', () => {
  it('renders nothing when the model has not changed', () => {
    const { container } = render(
      <ModelChip previousModelId="qwen" modelId="qwen" reason="general" fit={{ kind: 'comfortable' }} />
    );
    expect(container).toBeEmptyDOMElement();
  });

  it('shows the transition and reason when the model changed', () => {
    render(
      <ModelChip
        previousModelId="qwen-coder"
        modelId="llama-3.2-vision"
        reason="vision required"
        fit={{ kind: 'comfortable' }}
      />
    );
    expect(screen.getByText(/qwen-coder/)).toBeInTheDocument();
    expect(screen.getByText(/llama-3\.2-vision/)).toBeInTheDocument();
    expect(screen.getByText(/vision required/)).toBeInTheDocument();
  });

  it('renders a degraded warning verbatim', () => {
    render(
      <ModelChip
        previousModelId="qwen"
        modelId="llama-vision"
        reason="vision required"
        fit={{ kind: 'degraded', warning: '6800 MB needed, 5200 MB available' }}
      />
    );
    expect(screen.getByText('6800 MB needed, 5200 MB available')).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run, implement, verify, commit**

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/ModelChip.test.tsx
```
Implement: return `null` when `previousModelId === modelId`. Otherwise a `--t-12` line, model ids in `--font-mono`, separator `→` as its own element (never glued to button text). Warning block below on a 2px `--degraded` left rule with ▲.

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/ModelChip.test.tsx && pnpm run typecheck
git add ui/desktop/src/components/indra-shell/ModelChip.tsx ui/desktop/src/components/indra-shell/ModelChip.test.tsx
git commit -m "feat(ui): add model switch chip with verbatim fit warning"
```

---

## Phase D — Sovereignty · never cut

### Task 10: Sovereignty screen with the probe box

**Files:**
- Create: `ui/desktop/src/components/indra-shell/SovereigntyScreen.tsx`
- Test: `ui/desktop/src/components/indra-shell/SovereigntyScreen.test.tsx`

**Interfaces:**
- Consumes: `egress.attempt` events (Task 2).
- Produces: `<SovereigntyScreen attempts uptimeSeconds sandboxBackend modelsLoaded degradedCount sealedCount onProbe />` where `onProbe: (url: string) => Promise<void>`.

Spec §5.8. This is the screen the demo opens with. **The probe box is deliberately prominent — handing the sceptic the weapon and watching it fail is more persuasive than any badge.**

Backend already exists: `trigger_probe(url)` and `EgressLog::current()` in `crates/indra/src/security/egress_inspector.rs`. Note its honest semantics — `Ok` means blocked, `Err` means it actually reached the network. Render both truthfully; a networked dev machine reaching out is the correct result for that machine.

- [ ] **Step 1: Write the failing test**

```tsx
// ui/desktop/src/components/indra-shell/SovereigntyScreen.test.tsx
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { SovereigntyScreen } from './SovereigntyScreen';

const base = {
  attempts: [],
  uptimeSeconds: 1_231_200,
  sandboxBackend: 'docker',
  modelsLoaded: 3,
  degradedCount: 1,
  sealedCount: 12,
};

describe('SovereigntyScreen', () => {
  it('leads with the sealed state and attempt count', () => {
    render(<SovereigntyScreen {...base} onProbe={vi.fn()} />);
    expect(screen.getByText(/SEALED/)).toBeInTheDocument();
    expect(screen.getByText(/0 outbound attempts/)).toBeInTheDocument();
  });

  it('sends the typed URL to the probe', async () => {
    const onProbe = vi.fn().mockResolvedValue(undefined);
    render(<SovereigntyScreen {...base} onProbe={onProbe} />);

    await userEvent.type(screen.getByLabelText(/test a url/i), 'https://example.com');
    await userEvent.click(screen.getByRole('button', { name: /probe/i }));

    expect(onProbe).toHaveBeenCalledWith('https://example.com');
  });

  it('lists a blocked attempt with its destination and time', () => {
    render(
      <SovereigntyScreen
        {...base}
        attempts={[{ url: 'https://example.com', blocked: true, at: '2026-09-16T14:02:31Z' }]}
        onProbe={vi.fn()}
      />
    );
    expect(screen.getByText('https://example.com')).toBeInTheDocument();
    expect(screen.getByText(/BLOCKED/)).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run and watch it fail**

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/SovereigntyScreen.test.tsx
```

- [ ] **Step 3: Implement**

Header: ⛨ plus `SEALED` in `--sealed`, attempt count and uptime right-aligned in mono. Egress panel `--r-md`, 1px `--line`. Probe input `--r-sm` with a labelled Probe button. Attempt log is a mono list: time, URL, `BLOCKED`, mechanism, origin. Three summary cards below (Sandbox / Models / Sealed) per the spec's layout. Degraded model count carries ▲.

- [ ] **Step 4: Run, typecheck, commit**

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/SovereigntyScreen.test.tsx && pnpm run typecheck
git add ui/desktop/src/components/indra-shell/SovereigntyScreen.tsx ui/desktop/src/components/indra-shell/SovereigntyScreen.test.tsx
git commit -m "feat(ui): add sovereignty screen with interactive egress probe"
```

---

### Task 11: Wire the probe to the backend over ACP

**Files:**
- Modify: `crates/indra/src/acp/server/custom_dispatch.rs`
- Create: `crates/indra/src/acp/server/sovereignty.rs`
- Create: `ui/desktop/src/acp/sovereignty.ts`
- Test: `crates/indra/tests/acp_sovereignty.rs`

**Interfaces:**
- Consumes: `trigger_probe`, `EgressLog::current()` from `crates/indra/src/security/egress_inspector.rs`.
- Produces: ACP methods `EgressStatusRequest → EgressStatusResponse { attempts, uptime_seconds }` and `EgressProbeRequest { url } → EgressProbeResponse { blocked, reason }`; TS wrappers `acpEgressStatus()`, `acpEgressProbe(url)`.

Follow the existing pattern exactly — read `custom_dispatch.rs` and `providers.rs` first. Each method is a `#[custom_method(RequestType)]` dispatch fn delegating to an `on_*` handler.

- [ ] **Step 1: Write the failing Rust test**

```rust
// crates/indra/tests/acp_sovereignty.rs
use indra::security::egress_inspector::EgressLog;

#[test]
fn egress_log_is_readable_for_the_status_endpoint() {
    let log = EgressLog::current();
    assert!(
        log.attempts.len() < usize::MAX,
        "EgressLog::current() must be callable from the ACP layer"
    );
}
```

- [ ] **Step 2: Run it**

```bash
cd /Users/harikarthick/Desktop/goose-main && source bin/activate-hermit && cargo test -p indra --test acp_sovereignty
```

- [ ] **Step 3: Add the DTOs and handlers, then regenerate the typed client**

Define the request/response structs with the same derives the neighbouring ACP DTOs use, register both dispatch arms, then:

```bash
just generate-acp-types
```

Never hand-write into `ui/indra-acp-client/src/generated/` — AGENTS.md forbids it and the pipeline overwrites it.

- [ ] **Step 4: Verify and commit**

```bash
cargo test -p indra --test acp_sovereignty && cargo clippy -p indra --all-targets -- -D warnings
cd ui/desktop && pnpm run typecheck
git add crates/indra/src/acp/ crates/indra/tests/acp_sovereignty.rs ui/desktop/src/acp/sovereignty.ts crates/indra/acp-schema.json crates/indra/acp-meta.json ui/indra-acp-client/src/generated/
git commit -m "feat(acp): expose egress status and probe to the client"
```

---

## Phase E — Context ledger

### Task 12: Context gauge

**Files:**
- Modify: `ui/desktop/src/components/indra-shell/IndraTopBar.tsx`
- Test: `ui/desktop/src/components/indra-shell/IndraTopBar.test.tsx` (create)

**Interfaces:**
- Consumes: existing `sessionBytes`, `budgetBytes` props — already present, no signature change.
- Produces: gauge behaviour; `onOpenLedger?: () => void` added to `IndraTopBarProps`.

Spec §6.3. Mono, tabular: `4.1 kB / 32 kB`, 60px hairline bar. Ticks on `context.delta` over `--m-ui`. Past 70% it takes `--degraded`; at cap it holds while compaction runs. **Tabular numerals are mandatory** so the gauge does not jitter as it counts.

- [ ] **Step 1: Write the failing test**

```tsx
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { IndraTopBar } from './IndraTopBar';

const base = {
  sessionTitle: 'P-101 fit-for-service',
  onSessionTitleChange: vi.fn(),
  onToggleTheme: vi.fn(),
};

describe('IndraTopBar gauge', () => {
  it('shows used and budget bytes in kB', () => {
    render(<IndraTopBar {...base} sessionBytes={4118} budgetBytes={32768} />);
    expect(screen.getByText(/4\.1 kB/)).toBeInTheDocument();
    expect(screen.getByText(/32\.8 kB/)).toBeInTheDocument();
  });

  it('marks the gauge degraded past 70 percent', () => {
    render(<IndraTopBar {...base} sessionBytes={30000} budgetBytes={32768} />);
    expect(screen.getByRole('progressbar')).toHaveAttribute('data-state', 'near-cap');
  });

  it('uses tabular numerals so the number does not jitter', () => {
    render(<IndraTopBar {...base} sessionBytes={4118} budgetBytes={32768} />);
    expect(screen.getByText(/4\.1 kB/)).toHaveStyle({ fontVariantNumeric: 'tabular-nums' });
  });
});
```

- [ ] **Step 2: Run, implement, verify, commit**

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/IndraTopBar.test.tsx
```
Add `role="progressbar"` with `aria-valuenow`/`aria-valuemax` and `data-state` of `ok` / `near-cap` / `at-cap`.

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/IndraTopBar.test.tsx && pnpm run typecheck
git add ui/desktop/src/components/indra-shell/IndraTopBar.tsx ui/desktop/src/components/indra-shell/IndraTopBar.test.tsx
git commit -m "feat(ui): add byte-counted context gauge to the top bar"
```

---

### Task 13: Ledger sheet and compaction divider

**Files:**
- Create: `ui/desktop/src/components/indra-shell/ContextLedger.tsx`
- Create: `ui/desktop/src/components/indra-shell/CompactionDivider.tsx`
- Create: `ui/desktop/src/components/indra-shell/Sheet.tsx`
- Test: `ui/desktop/src/components/indra-shell/ContextLedger.test.tsx`, `Sheet.test.tsx`

**Interfaces:**
- Produces: `<Sheet open width onClose>` (reused by Tasks 15, 16, 19), `<ContextLedger sections onPin onDrop />`, `<CompactionDivider fromTurns toBytes keptCount droppedCount onShow />`.

`Sheet` is the shared right-side panel: widths 380/480/520px, `Esc` closes, focus trapped while open, **one at a time**, slide 24px + fade over `--m-enter`, cut instead of slide under reduced motion. Build it here since three later tasks depend on it.

Spec §6.3: every row has a byte count; every row can be pinned or dropped. **Compaction is an event in the transcript, not a silent trim** — a system that quietly forgets is one nobody can plan around.

- [ ] **Step 1: Write the failing Sheet test**

```tsx
// ui/desktop/src/components/indra-shell/Sheet.test.tsx
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { Sheet } from './Sheet';

describe('Sheet', () => {
  it('renders nothing when closed', () => {
    const { container } = render(
      <Sheet open={false} width={480} onClose={vi.fn()}>
        <p>content</p>
      </Sheet>
    );
    expect(container).toBeEmptyDOMElement();
  });

  it('closes on Escape', async () => {
    const onClose = vi.fn();
    render(
      <Sheet open width={480} onClose={onClose}>
        <p>content</p>
      </Sheet>
    );
    await userEvent.keyboard('{Escape}');
    expect(onClose).toHaveBeenCalledOnce();
  });
});
```

- [ ] **Step 2: Run, implement Sheet, verify, commit**

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/Sheet.test.tsx
```
```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/Sheet.test.tsx && pnpm run typecheck
git add ui/desktop/src/components/indra-shell/Sheet.tsx ui/desktop/src/components/indra-shell/Sheet.test.tsx
git commit -m "feat(ui): add shared right-side sheet primitive"
```

- [ ] **Step 3: Write the failing ledger test**

```tsx
// ui/desktop/src/components/indra-shell/ContextLedger.test.tsx
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ContextLedger } from './ContextLedger';

const sections = [
  {
    name: 'Scope',
    bytes: 1204,
    rows: [
      { id: 'r1', label: 'NDT_2026_08.pdf', detail: 'v3 pages 1,4,7', bytes: 412, pinned: true },
      { id: 'r2', label: 'Pump_Inspection_SOP.pdf', detail: 'v1 §4.2', bytes: 398, pinned: false },
    ],
  },
];

describe('ContextLedger', () => {
  it('shows a byte count on every row', () => {
    render(<ContextLedger sections={sections} onPin={vi.fn()} onDrop={vi.fn()} />);
    expect(screen.getByText('412 B')).toBeInTheDocument();
    expect(screen.getByText('398 B')).toBeInTheDocument();
  });

  it('shows the section total', () => {
    render(<ContextLedger sections={sections} onPin={vi.fn()} onDrop={vi.fn()} />);
    expect(screen.getByText('1,204 B')).toBeInTheDocument();
  });
});
```

- [ ] **Step 4: Implement, verify, commit**

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/ContextLedger.test.tsx && pnpm run typecheck
git add ui/desktop/src/components/indra-shell/ContextLedger.tsx ui/desktop/src/components/indra-shell/CompactionDivider.tsx ui/desktop/src/components/indra-shell/ContextLedger.test.tsx
git commit -m "feat(ui): add context ledger sheet and compaction divider"
```

---

## Phase F — Sources

### Task 14: Document library

**Files:**
- Create: `ui/desktop/src/components/indra-shell/SourcesLibrary.tsx`
- Test: `ui/desktop/src/components/indra-shell/SourcesLibrary.test.tsx`

**Interfaces:**
- Produces: `<SourcesLibrary documents view onOpen onViewChange />`, `DocumentSummary { id, title, kind, version, indexed, classification, pages, sealed }`.

Spec §5.6. Grid or list toggle. **Sealed resources carry a lock and a tooltip that names the policy, not a generic "no permission"** — a named policy is actionable; a generic denial is a dead end.

- [ ] **Step 1: Write the failing test**

```tsx
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { SourcesLibrary } from './SourcesLibrary';

const documents = [
  { id: 'd1', title: 'NDT_2026_08.pdf', kind: 'pdf', version: 'v3', indexed: true,
    classification: 'Internal', pages: 12, sealed: false },
  { id: 'd2', title: 'Historian_Export.csv', kind: 'csv', version: 'v1', indexed: true,
    classification: 'Restricted', pages: 1, sealed: true, sealedPolicy: 'OT boundary — Purdue L3' },
];

describe('SourcesLibrary', () => {
  it('lists documents with version and page count', () => {
    render(<SourcesLibrary documents={documents} view="list" onOpen={vi.fn()} onViewChange={vi.fn()} />);
    expect(screen.getByText('NDT_2026_08.pdf')).toBeInTheDocument();
    expect(screen.getByText(/12 pages/)).toBeInTheDocument();
  });

  it('names the policy on a sealed document rather than saying no permission', () => {
    render(<SourcesLibrary documents={documents} view="list" onOpen={vi.fn()} onViewChange={vi.fn()} />);
    expect(screen.getByLabelText(/OT boundary — Purdue L3/)).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run, implement, verify, commit**

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/SourcesLibrary.test.tsx && pnpm run typecheck
git add ui/desktop/src/components/indra-shell/SourcesLibrary.tsx ui/desktop/src/components/indra-shell/SourcesLibrary.test.tsx
git commit -m "feat(ui): add sources library with named sealed policies"
```

---

### Task 15: Reader with OCR overlay

**Files:**
- Create: `ui/desktop/src/components/indra-shell/SourceReader.tsx`
- Test: `ui/desktop/src/components/indra-shell/SourceReader.test.tsx`

**Interfaces:**
- Consumes: `SourceBox` (Task 2), `Sheet` (Task 13).
- Produces: `<SourceReader pageImageUrl boxes showOverlay highlightedBoxId onToggleOverlay onBoxClick />`.

Spec §5.6 — where box-level provenance pays off. Original scan at full fidelity; OCR boxes as a **toggleable** overlay; confidence rendered as **border weight**; sub-0.60 boxes outlined `--degraded`. Click a box → every claim in every session that cited it (reverse provenance).

- [ ] **Step 1: Write the failing test**

```tsx
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { SourceReader } from './SourceReader';

const boxes = [
  { doc_id: 'd1', version: 'v3', page: 4, bbox: [10, 20, 100, 24] as [number,number,number,number], conf: 0.94 },
  { doc_id: 'd1', version: 'v3', page: 4, bbox: [10, 60, 100, 24] as [number,number,number,number], conf: 0.41 },
];

describe('SourceReader', () => {
  it('hides the overlay when toggled off', () => {
    render(<SourceReader pageImageUrl="/p4.png" boxes={boxes} showOverlay={false}
      onToggleOverlay={vi.fn()} onBoxClick={vi.fn()} />);
    expect(screen.queryAllByTestId('ocr-box')).toHaveLength(0);
  });

  it('marks a low-confidence box as uncertain in text, not only colour', () => {
    render(<SourceReader pageImageUrl="/p4.png" boxes={boxes} showOverlay
      onToggleOverlay={vi.fn()} onBoxClick={vi.fn()} />);
    expect(screen.getByLabelText(/low confidence/i)).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run, implement, verify, commit**

Border weight scales with confidence (1px at 1.0 → 2px below 0.60). Sub-0.60 boxes use `--degraded` **and** an `aria-label` containing "low confidence".

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/SourceReader.test.tsx && pnpm run typecheck
git add ui/desktop/src/components/indra-shell/SourceReader.tsx ui/desktop/src/components/indra-shell/SourceReader.test.tsx
git commit -m "feat(ui): add source reader with confidence-weighted OCR overlay"
```

---

## Phase G — Trace

### Task 16: Run history, replay, approvals queue

**Files:**
- Create: `ui/desktop/src/components/indra-shell/TraceScreen.tsx`
- Create: `ui/desktop/src/components/indra-shell/ApprovalsQueue.tsx`
- Test: `ui/desktop/src/components/indra-shell/TraceScreen.test.tsx`, `ApprovalsQueue.test.tsx`

**Interfaces:**
- Consumes: `IndraEvent[]` (Task 2), `PlanCard` (Task 5), `ToolRow` (Task 6), `Sheet` (Task 13).
- Produces: `<TraceScreen runs onExport />`, `<ApprovalsQueue proposals onApprove onReject onRequestEvidence />`.

Spec §5.7. A run expands into its full event stream — the same events the Work screen rendered live, replayable, reusing the same components. **Nothing is applied without a name attached**; each proposal shows current value, proposed value, evidence with citations, and the role required.

- [ ] **Step 1: Write the failing approvals test**

```tsx
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ApprovalsQueue } from './ApprovalsQueue';

const proposals = [
  { id: 'p1', resource: 'SAP PM order 4500123', currentValue: 'TECO', proposedValue: 'Reopened',
    requiredRole: 'Maintenance Supervisor', evidence: [] },
];

describe('ApprovalsQueue', () => {
  it('shows current and proposed values and the role required', () => {
    render(<ApprovalsQueue proposals={proposals} onApprove={vi.fn()} onReject={vi.fn()} onRequestEvidence={vi.fn()} />);
    expect(screen.getByText('TECO')).toBeInTheDocument();
    expect(screen.getByText('Reopened')).toBeInTheDocument();
    expect(screen.getByText(/Maintenance Supervisor/)).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run, implement, verify, commit**

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/ && pnpm run typecheck
git add ui/desktop/src/components/indra-shell/TraceScreen.tsx ui/desktop/src/components/indra-shell/ApprovalsQueue.tsx ui/desktop/src/components/indra-shell/TraceScreen.test.tsx ui/desktop/src/components/indra-shell/ApprovalsQueue.test.tsx
git commit -m "feat(ui): add trace screen and approvals queue"
```

---

## Phase H — Memory · cut Timeline first, Constellation second, keep Ledger

### Task 17: Memory ledger tab

**Files:**
- Create: `ui/desktop/src/components/indra-shell/MemoryLedger.tsx`
- Test: `ui/desktop/src/components/indra-shell/MemoryLedger.test.tsx`

**Interfaces:**
- Produces: `<MemoryLedger nodes selectedIds onSelectionChange />`, `MemoryNode { id, label, version, hashPrefix, bytes, acl, lastCited, citingSessions, state }`.

Build this **before** the constellation. Spec §11 is blunt about why: *"a force graph nobody acts on is decoration"* — and the Ledger is also the constellation's full keyboard equivalent (§8), so it is the accessible peer view, not a fallback.

Virtualised table, sortable, every column filterable. Shift-select rows → the same floating action bar the lasso produces.

- [ ] **Step 1: Write the failing test, implement, verify, commit**

```tsx
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { MemoryLedger } from './MemoryLedger';

const nodes = [
  { id: 'n1', label: 'NDT_2026_08.pdf', version: 'v3', hashPrefix: '7f3a', bytes: 412,
    acl: 'Inspection', lastCited: '2026-09-14', citingSessions: 3, state: 'cached' as const },
];

describe('MemoryLedger', () => {
  it('shows hash prefix and citing session count', () => {
    render(<MemoryLedger nodes={nodes} selectedIds={[]} onSelectionChange={vi.fn()} />);
    expect(screen.getByText('7f3a')).toBeInTheDocument();
    expect(screen.getByText('3')).toBeInTheDocument();
  });
});
```

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/MemoryLedger.test.tsx && pnpm run typecheck
git add ui/desktop/src/components/indra-shell/MemoryLedger.tsx ui/desktop/src/components/indra-shell/MemoryLedger.test.tsx
git commit -m "feat(ui): add memory ledger table"
```

---

### Task 18: Constellation canvas with lasso-to-scope

**Files:**
- Create: `ui/desktop/src/components/indra-shell/Constellation.tsx`
- Create: `ui/desktop/src/indra/memory/forceLayout.ts`
- Test: `ui/desktop/src/indra/memory/forceLayout.test.ts`

**Interfaces:**
- Consumes: `memory.touch` events (Task 2), `MemoryNode` (Task 17).
- Produces: `<Constellation nodes edges onLassoSelect />`.

Canvas 2D with quadtree hit-testing — **DOM/SVG dies past ~2k nodes and a plant corpus is 50k**. Encoding is functional, never decorative: radius = citation frequency, fill = cache state, ring = classification, edge weight = relationship strength, edge style = solid derived-from / dashed references / dotted superseded-by.

**Lasso-to-scope is the only reason this view exists** (§11). Drag → floating bar: `N nodes selected · Ask about this scope · Save as view · Export refs`. If lasso gets cut, cut the whole view and ship the Ledger.

LOD (§9): below 0.4 zoom, edges hide and nodes cluster by department; labels only above 0.8 zoom. Hit target minimum 12px regardless of visual radius.

- [ ] **Step 1: Test the pure layout maths first** — force simulation and quadtree hit-testing are testable without a canvas; the rendering is not. Test `forceLayout.ts` in isolation: given nodes and edges, one tick moves connected nodes closer; `hitTest(x, y, nodes)` returns the node within 12px.

- [ ] **Step 2: Implement, verify, commit**

```bash
cd ui/desktop && pnpm vitest run src/indra/memory/forceLayout.test.ts && pnpm run typecheck
git add ui/desktop/src/indra/memory/ ui/desktop/src/components/indra-shell/Constellation.tsx
git commit -m "feat(ui): add memory constellation with lasso-to-scope"
```

---

### Task 19: Memory timeline

**Files:**
- Create: `ui/desktop/src/components/indra-shell/MemoryTimeline.tsx`
- Test: `ui/desktop/src/components/indra-shell/MemoryTimeline.test.tsx`

**Interfaces:**
- Produces: `<MemoryTimeline lanes onCitationClick />`.

**First thing to cut if you slip.** Horizontal, one lane per document: version bumps, cache fills, invalidations, citations. Answers the question an auditor actually asks: *when did this change, and did anything we concluded depend on the old version?* A citation of a superseded version renders in `--degraded` and links to the affected session.

- [ ] **Step 1: Write the failing test, implement, verify, commit**

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/MemoryTimeline.test.tsx && pnpm run typecheck
git add ui/desktop/src/components/indra-shell/MemoryTimeline.tsx ui/desktop/src/components/indra-shell/MemoryTimeline.test.tsx
git commit -m "feat(ui): add memory timeline with superseded-citation marking"
```

---

## Phase I — Boot and setup

### Task 20: Boot animation

**Files:**
- Create: `ui/desktop/src/components/indra-shell/BootSequence.tsx`
- Create: `ui/desktop/src/components/indra-shell/IndraMark.tsx`
- Test: `ui/desktop/src/components/indra-shell/BootSequence.test.tsx`

**Interfaces:**
- Consumes: `useReducedMotion` (Task 3).
- Produces: `<IndraMark size />`, `<BootSequence steps onComplete />` where `steps: { id, label, state: 'pending'|'running'|'ok'|'warn'|'fail', detail? }[]`.

Mark geometry, exactly (§3.1) — a ring above a stadium, which is also a lowercase **i**:

```
24-unit grid, stroke 2.6, round caps
  ring     cx 12  cy 5.4   r 3.5      + centre dot r 1.15 (filled)
  stadium  x 8.5  y 10.6   w 7  h 11  rx 3.5
  gap between them: 1.7 units — never compress it
```

Five beats over 2200ms, each wired to a **real check** (§5.1). Two rules that matter:

- **If checks finish early, fast-forward to beat 5 rather than stalling.** A boot animation that outlasts the boot is theatre, and this audience will notice.
- A `fail` stops the figure, tints the glyph `--blocked`, shows the actual error with Retry / Continue anyway. A `warn` tints `--degraded`, continues, and carries the warning into the workspace as a dismissible strip.

Reduced motion → static mark plus five checklist lines resolving in place.

- [ ] **Step 1: Write the failing test**

```tsx
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { BootSequence } from './BootSequence';

const steps = [
  { id: 'config', label: 'Reading configuration', state: 'ok' as const },
  { id: 'hardware', label: 'Probing hardware', state: 'ok' as const },
  { id: 'models', label: 'Scanning model registry', state: 'warn' as const, detail: '1 model degraded' },
];

describe('BootSequence', () => {
  it('shows the detail text for a warning step rather than hiding it', () => {
    render(<BootSequence steps={steps} onComplete={vi.fn()} />);
    expect(screen.getByText('1 model degraded')).toBeInTheDocument();
  });

  it('offers retry and continue when a step fails', () => {
    render(
      <BootSequence
        steps={[{ id: 'sealed', label: 'Mounting sealed store', state: 'fail', detail: 'keychain locked' }]}
        onComplete={vi.fn()}
      />
    );
    expect(screen.getByRole('button', { name: /retry/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /continue anyway/i })).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run, implement, verify, commit**

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/BootSequence.test.tsx && pnpm run typecheck
git add ui/desktop/src/components/indra-shell/BootSequence.tsx ui/desktop/src/components/indra-shell/IndraMark.tsx ui/desktop/src/components/indra-shell/BootSequence.test.tsx
git commit -m "feat(ui): add boot sequence wired to real system checks"
```

---

### Task 21: Setup steps 1 and 2

**Files:**
- Create: `ui/desktop/src/components/indra-shell/SetupIdentity.tsx`
- Create: `ui/desktop/src/components/indra-shell/SetupModels.tsx`
- Test: both `.test.tsx`

**Interfaces:**
- Consumes: `ModelTable`, `ModelRow` (Task 8).
- Produces: `<SetupIdentity onChoose />`, `<SetupModels rows onAddDirectory onAddServer onNext />`.

Spec §5.2. Centred column 520px, `--t-28` headings, rail hidden — the only screen in the app that breathes. Step 1 is **two cards, not a form**: Plant sign-in vs Standalone profile. The standalone path states plainly that a forgotten passphrase means the sealed store is unrecoverable, **because it is**.

Step 2 gets both acquisition paths (the documented deviation): "Scan a directory" and "Add a model server". Empty directory → an empty state naming the two files needed and the offline copy path, never a dead end.

- [ ] **Step 1: Write failing tests, implement, verify, commit**

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/Setup && pnpm run typecheck
git add ui/desktop/src/components/indra-shell/SetupIdentity.tsx ui/desktop/src/components/indra-shell/SetupModels.tsx ui/desktop/src/components/indra-shell/SetupIdentity.test.tsx ui/desktop/src/components/indra-shell/SetupModels.test.tsx
git commit -m "feat(ui): add setup steps for identity and model sources"
```

---

### Task 22: Setup step 3 — prove it is sealed

**Files:**
- Create: `ui/desktop/src/components/indra-shell/SetupSovereignty.tsx`
- Test: `ui/desktop/src/components/indra-shell/SetupSovereignty.test.tsx`

**Interfaces:**
- Consumes: `acpEgressProbe` (Task 11).
- Produces: `<SetupSovereignty onProbe onStart />`.

The trust moment, and it is **interactive rather than a claim**. The user types any URL, it gets blocked and logged, and the record persists to the Sovereignty page they can revisit.

- [ ] **Step 1: Write the failing test, implement, verify, commit**

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/SetupSovereignty.test.tsx && pnpm run typecheck
git add ui/desktop/src/components/indra-shell/SetupSovereignty.tsx ui/desktop/src/components/indra-shell/SetupSovereignty.test.tsx
git commit -m "feat(ui): add interactive sovereignty proof to first-run setup"
```

---

## Phase J — Palette and skills

### Task 23: ⌘K command palette

**Files:**
- Create: `ui/desktop/src/components/indra-shell/CommandPalette.tsx`
- Create: `ui/desktop/src/indra/paletteIndex.ts`
- Test: both

**Interfaces:**
- Produces: `PaletteEntry { id, label, kind: 'destination'|'session'|'document'|'skill'|'recipe'|'theme'|'scope', hint?, run: () => void }`, `<CommandPalette open entries onClose />`.

Spec §4: *"This is why five icons suffice."* Fuzzy over one flat index. Opens in 120ms, **no backdrop animation**. One of only two places `--shadow-pop` is permitted.

- [ ] **Step 1: Write the failing test**

```tsx
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { CommandPalette } from './CommandPalette';

const entries = [
  { id: 'work', label: 'Go to Work', kind: 'destination' as const, run: vi.fn() },
  { id: 'sov', label: 'Go to Sovereignty', kind: 'destination' as const, run: vi.fn() },
  { id: 'skill-ndt', label: '/ndt-review', kind: 'skill' as const, run: vi.fn() },
];

describe('CommandPalette', () => {
  it('filters entries as you type', async () => {
    render(<CommandPalette open entries={entries} onClose={vi.fn()} />);
    await userEvent.type(screen.getByRole('combobox'), 'sov');
    expect(screen.getByText('Go to Sovereignty')).toBeInTheDocument();
    expect(screen.queryByText('Go to Work')).not.toBeInTheDocument();
  });

  it('finds skills by their slash name', async () => {
    render(<CommandPalette open entries={entries} onClose={vi.fn()} />);
    await userEvent.type(screen.getByRole('combobox'), 'ndt');
    expect(screen.getByText('/ndt-review')).toBeInTheDocument();
  });

  it('names the next action when nothing matches', async () => {
    render(<CommandPalette open entries={entries} onClose={vi.fn()} />);
    await userEvent.type(screen.getByRole('combobox'), 'zzzz');
    expect(screen.getByText(/no matches/i)).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run, implement, verify, commit**

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/CommandPalette.test.tsx && pnpm run typecheck
git add ui/desktop/src/components/indra-shell/CommandPalette.tsx ui/desktop/src/indra/paletteIndex.ts
git commit -m "feat(ui): add command palette over a flat entry index"
```

---

### Task 24: Surface skills as slash commands

**Files:**
- Create: `ui/desktop/src/indra/skills.ts`
- Create: `ui/desktop/src/components/indra-shell/SlashMenu.tsx`
- Test: both

**Interfaces:**
- Consumes: the existing Rust `list_commands(working_dir)` / `resolve_command(...)` in `crates/indra/src/slash_commands/skill_slash_command.rs`, reached over ACP.
- Produces: `useSkillCommands(): SkillCommand[]`, `<SlashMenu query commands onPick />`.

**You are not building a skill registry.** One already exists and already turns installed skills into slash commands. This task surfaces them: typing `/` in the composer opens a filtered menu; each entry also appears in the palette (Task 23) with `kind: 'skill'`.

Check first whether an ACP method already exposes the command list before adding one:

```bash
grep -rn "slash\|list_commands" crates/indra/src/acp/ | head
```

If a method exists, consume it. If not, add one following Task 11's pattern.

- [ ] **Step 1: Write the failing test**

```tsx
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { SlashMenu } from './SlashMenu';

const commands = [
  { name: 'ndt-review', description: 'Review an NDT report against allowables' },
  { name: 'pid-trace', description: 'Trace a line on a P&ID' },
];

describe('SlashMenu', () => {
  it('filters by the typed query', () => {
    render(<SlashMenu query="pid" commands={commands} onPick={vi.fn()} />);
    expect(screen.getByText('/pid-trace')).toBeInTheDocument();
    expect(screen.queryByText('/ndt-review')).not.toBeInTheDocument();
  });

  it('shows each command description', () => {
    render(<SlashMenu query="" commands={commands} onPick={vi.fn()} />);
    expect(screen.getByText('Review an NDT report against allowables')).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run, implement, verify, commit**

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/SlashMenu.test.tsx && pnpm run typecheck
git add ui/desktop/src/indra/skills.ts ui/desktop/src/components/indra-shell/SlashMenu.tsx
git commit -m "feat(ui): surface installed skills as slash commands"
```

---

## Phase K — Appearance

### Task 25: Appearance panel with live preview

**Files:**
- Create: `ui/desktop/src/components/indra-shell/AppearancePanel.tsx`
- Create: `ui/desktop/src/indra/theme.ts`
- Test: both

**Interfaces:**
- Produces: `Theme { name, base, font: { ui, mono, scale, ligatures }, density, overrides }`, `applyTheme(theme)`, `<AppearancePanel theme onChange />`.

Spec §3.6. Controls: base (Ash Dark / Chalk Light / High Contrast / OLED true-black) · UI font · Mono font · Scale (0.9/1.0/1.1/1.25) · Density · Ligatures · Motion · Stream speed · Trace density.

A theme is an override object under 1 kB, serialisable to a text blob **so a plant can paste a house theme into a field and hit apply — no rebuild, no theme store, no network.** Right side is a live preview of a chat exchange, a tool row, and a code block — the three things whose legibility actually matters. **Changes apply instantly; no "restart to apply", ever.**

- [ ] **Step 1: Write the failing test, implement, verify, commit**

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/AppearancePanel.test.tsx && pnpm run typecheck
git add ui/desktop/src/components/indra-shell/AppearancePanel.tsx ui/desktop/src/indra/theme.ts
git commit -m "feat(ui): add appearance panel with pasteable themes"
```

---

### Task 25b: Lock overlay and trace density

**Files:**
- Create: `ui/desktop/src/components/indra-shell/LockOverlay.tsx`
- Create: `ui/desktop/src/indra/traceDensity.ts`
- Test: `ui/desktop/src/components/indra-shell/LockOverlay.test.tsx`

**Interfaces:**
- Consumes: `useReducedMotion` (Task 3).
- Produces: `<LockOverlay open mode attemptsRemaining onUnlock />` where `mode: 'passphrase' | 'idp'`; `type TraceDensity = 'quiet' | 'normal' | 'trace'`, `useTraceDensity(): [TraceDensity, (d: TraceDensity) => void]`.

Two spec requirements that Task 26's keymap and Task 25's Appearance panel both reference but nothing else builds.

**Lock overlay (§5.3).** `⌘L`, policy timeout, or lid close. The workspace **stays on screen behind** a 12px backdrop blur dimmed to 45% — nothing is unloaded, nothing scrolls away, and the person sees the work they are returning to. A 320px card asks for passphrase or IdP re-auth. Wrong passphrase shakes 4px **once** and states attempts remaining; after the limit the sealed key is dropped from memory and the app returns to cold boot **rather than pretending**.

This is the one place blur is permitted — it is a backdrop, not decoration.

**Trace density (§6.2).** Remembered per user, default Normal:

| Mode | Shows |
|---|---|
| Quiet | one summary line per turn: `3 steps · 2 tools · 4.4s` — expandable |
| **Normal** | plan card + collapsed tool rows + model chips |
| Trace | all of the above expanded, plus timings, token counts, raw events |

Trace mode additionally puts a `--degraded` gutter marker on any sentence with no citation. Per §6.4 that marker is *"the most useful QA tool you will build"* — do not skip it.

- [ ] **Step 1: Write the failing test**

```tsx
// ui/desktop/src/components/indra-shell/LockOverlay.test.tsx
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { LockOverlay } from './LockOverlay';

describe('LockOverlay', () => {
  it('renders nothing when unlocked', () => {
    const { container } = render(
      <LockOverlay open={false} mode="passphrase" attemptsRemaining={3} onUnlock={vi.fn()} />
    );
    expect(container).toBeEmptyDOMElement();
  });

  it('states attempts remaining after a failure', () => {
    render(
      <LockOverlay open mode="passphrase" attemptsRemaining={2} onUnlock={vi.fn()} />
    );
    expect(screen.getByText(/2 attempts remaining/i)).toBeInTheDocument();
  });

  it('keeps the workspace behind it rather than unmounting it', () => {
    render(
      <LockOverlay open mode="passphrase" attemptsRemaining={3} onUnlock={vi.fn()}>
        <p>work in progress</p>
      </LockOverlay>
    );
    expect(screen.getByText('work in progress')).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run and watch it fail**

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/LockOverlay.test.tsx
```

- [ ] **Step 3: Implement both**

Backdrop `backdrop-filter: blur(12px)` with `background: rgba(0,0,0,.55)`, animating 0→12px over `--m-enter`; under reduced motion it appears at full blur with no transition. Children render beneath, never unmounted. Focus traps to the card.

`traceDensity.ts` persists the choice to `localStorage` under `indra.traceDensity`, defaulting to `'normal'`.

- [ ] **Step 4: Run, typecheck, commit**

```bash
cd ui/desktop && pnpm vitest run src/components/indra-shell/LockOverlay.test.tsx && pnpm run typecheck
git add ui/desktop/src/components/indra-shell/LockOverlay.tsx ui/desktop/src/components/indra-shell/LockOverlay.test.tsx ui/desktop/src/indra/traceDensity.ts
git commit -m "feat(ui): add lock overlay and per-user trace density"
```

---

## Final integration

### Task 26: Mount every screen and wire the keymap

**Files:**
- Modify: `ui/desktop/src/components/indra-shell/IndraWorkspace.tsx`
- Create: `ui/desktop/src/indra/useKeymap.ts`
- Test: `ui/desktop/src/indra/useKeymap.test.ts`

Replace Task 1's placeholder switch with the real screens. Keymap (§7.3), complete:

`⌘K` palette · `⌘L` lock · `⌘\` toggle right sheet · `⌘⇧C` context ledger · `⌘1-5` rail destinations · `Esc` stop generation, then close sheet, then clear selection · `⌘↵` send · `⇧↵` newline · `⌘F` find in transcript · `[` `]` previous/next citation · `⌘E` export run.

`Esc` is **ordered, not ambiguous** — stop generation first, then close the sheet, then clear selection. Test that ordering explicitly.

- [ ] **Step 1: Test the Esc ordering, implement, verify, commit**

```bash
cd ui/desktop && pnpm vitest run src/indra/useKeymap.test.ts && pnpm run typecheck && pnpm test
git add ui/desktop/src/
git commit -m "feat(ui): mount all screens and wire the full keymap"
```

---

## Verification before calling this done

```bash
cd ui/desktop && pnpm run typecheck && pnpm test
cd /Users/harikarthick/Desktop/goose-main && source bin/activate-hermit && cargo clippy -p indra --all-targets -- -D warnings
```

Then check by hand, because these are the things tests do not catch:

1. **Reduced motion.** Set the OS preference and confirm boot is static, stream commits per word, and sheets cut instead of sliding.
2. **Chalk Light.** Toggle and check every signal colour still reads — light values differ and only a human notices a washed-out `--degraded`.
3. **The fit warning is verbatim.** Compare the rendered string against what the Rust registry produced, character for character.
4. **No signal-colour fills.** Scan every screen for a coloured area larger than 2px. The rule exists so a workbench never feels like an incident.
5. **`--focus` appears on exactly two things** — the focus ring and the caret. Anywhere else is a bug.
