# Storybook OpenAI Toolbar Implementation Plan

## Goals
- Build a custom Storybook addon (toolbar + panel) that controls a shared `MockOpenAiProvider`.
- Expose full coverage of `window.openai` globals and imperative APIs for interactive testing.
- Ensure helpers remain reusable from stories, play functions, and automated tests.

## Scope Overview
- Author a robust `MockOpenAiProvider` and helper API in `src/testing/openai-storybook.ts`.
- Implement a toolbar addon in `.storybook/openai-addon/` that drives the mock.
- Add a state inspector panel to visualize host globals and API call history.
- Wire the addon into Storybook configuration and document usage.

## Detailed Steps

### 1. Create Mock Host Utilities (`src/testing/openai-storybook.ts`)
- Define TypeScript types mirroring `OpenAiGlobals`, `OpenAiHostApi` (infer from `src/types.ts`).
- Maintain an internal `currentGlobals` object and `listeners` array for change subscriptions.
- Implement helpers:
  - `installOpenAiMock(initial?: Partial<Globals & Api>)` – defines `window.openai`, sets defaults, registers spies for imperative methods.
  - `updateOpenAiGlobals(patch: Partial<Globals>)` – shallow merge, dispatch `new SetGlobalsEvent({ detail: { globals: currentGlobals } })`.
  - `invokeHostMethod(name, args)` – centrally record calls, execute the real mock implementation, and notify listeners (used by panel logging).
  - `getHostSnapshot()` – returns current globals + call history.
  - `resetHostState()` – clears history and restores defaults.
  - `subscribe(listener)` – pushes listener into array, returns unsubscribe function.
- Export `MockOpenAiProvider` React component that installs/uninstalls the mock inside `useEffect`.
- Provide utility hooks (optional): `useOpenAiSnapshot` (wraps `subscribe`) for React consumers.

### 2. Storybook Global Integration
- Update `.storybook/preview.tsx`:
  - Import `MockOpenAiProvider`.
  - Wrap all stories via `decorators`.
  - Initialize default globals (`toolOutput`, `displayMode`, etc.) from `fixtures.ts`.
  - Define `globalTypes` entries for high-frequency controls (theme, displayMode, locale) that interact with toolbar state.
- Ensure `.storybook/tsconfig.json` points to `src/testing` for typed imports.

### 3. Author the Addon Shell
- Create `.storybook/openai-addon/register.ts` that imports `addons` from `@storybook/manager-api`.
- Add manifest in `.storybook/main.ts` via `addons: [..., path.resolve("./.storybook/openai-addon")]`.
- Inside `register.ts`, call `addons.register()` and `addons.add()` with:
  - Toolbar tool (type `TOOL`) – icon (e.g., `globe`), label `OpenAI`.
  - Panel (type `PANEL`) – titled `OpenAI State`.
- Create a shared `channel` helper (using `addons.getChannel()`) to communicate between manager (toolbar/panel UI) and preview.

### 4. Toolbar UI Implementation
- In `.storybook/openai-addon/Toolbar.tsx`:
  - Render dropdowns/buttons for:
    - `displayMode` selection (`inline`/`pip`/`fullscreen`).
    - Theme toggle (`light`/`dark`).
    - Device type (`desktop`/`mobile`) adjusting `userAgent`.
    - Quick actions (buttons) for imperative APIs:
      - `requestDisplayMode("fullscreen")`
      - `sendFollowUpMessage("Sample prompt")`
      - `reset state`.
  - Publish events on change via `channel.emit("openai:updateGlobals", payload)` or `"openai:callMethod"`.
- Support JSON edit modal:
  - Trigger button opens a `Modal` component (Storybook UI) letting user edit `toolOutput`/`toolInput` JSON.
  - On submit, emit update event.

### 5. Panel UI Implementation
- In `.storybook/openai-addon/Panel.tsx`:
  - Subscribe to `channel.on("openai:snapshot", ...)` to receive updated state payloads.
  - Render read-only sections:
    - Current globals (pretty JSON).
    - Widget state.
    - Recent API calls (table with method, args, timestamp, result/error).
  - Provide clear logs button (`channel.emit("openai:reset")`).
- Ensure panel updates when preview broadcasts snapshot changes (Step 6).

### 6. Preview Side Wiring
- Create `.storybook/openai-addon/preview-hooks.ts` to run within preview iframe:
  - Import `addons` from `@storybook/preview-api` and call `addons.getChannel()` once.
  - Call `installOpenAiMock()` during initialization.
  - Register listeners:
    - `channel.on("openai:updateGlobals", updateOpenAiGlobals)`.
    - `channel.on("openai:callMethod", ({ name, args }))` -> execute via `invokeHostMethod`.
    - `channel.on("openai:reset", resetHostState)`.
  - Subscribe to host state (`subscribe`) and broadcast via `channel.emit("openai:snapshot", getHostSnapshot())`.
- Auto-import this module in `.storybook/preview.tsx` (side-effect) so it runs before stories mount.

### 7. Type-Safe Event Schema
- Create `src/testing/openai-addon-channel.ts` exporting TypeScript constants for event names and payload shapes.
- Use shared file across manager + preview to avoid typos and ensure type safety.

### 8. Testing the Addon
- Add Storybook stories exercising toolbar actions (e.g., story showing open panel).
- Write Jest / Vitest tests (if feasible) for mock helpers:
  - Ensure `updateOpenAiGlobals` dispatches events and updates snapshot.
  - Verify imperative calls log correctly.
- Add an interaction test using `play` function to call `channel.emit` and assert UI updates via Testing Library.

### 9. Documentation
- Update `docs/storybook-integration-plan.md` to reference new toolbar capability.
- Add README section summarizing usage:
  - How to open toolbar/panel.
  - List of controls & actions.
  - Instructions for extending with new host APIs.
- Provide usage examples (screenshots optional).

### 10. Follow-up Enhancements (Optional)
- Persist toolbar settings between reloads using `manager.store`.
- Allow saved presets of host state.
- Add validation and error messaging for invalid JSON edits.
- Integrate call speed throttling or failure simulation toggles (`requestDisplayMode` rejection scenarios).

## Deliverables
- `src/testing/openai-storybook.ts` (helpers + provider).
- `.storybook/openai-addon/` directory with toolbar, panel, and shared utilities.
- Updated Storybook configuration (`main.ts`, `preview.tsx`, `tsconfig.json`).
- Documentation updates (this plan, integration plan refresh, README snippet).
- Optional tests validating helper logic.

## Manual QA Checklist
- Start Storybook via `pnpm storybook` and confirm the toolbar icon appears with expected label.
- Toggle each toolbar control (theme, display mode, device type) and verify stories respond, inspecting both the rendered widget and the panel snapshot.
- Use the JSON editor to update `toolOutput` and ensure changes render immediately and are logged in the panel history.
- Trigger imperative buttons (`requestDisplayMode`, `sendFollowUpMessage`, etc.) and confirm the panel records each call with timestamp and result.
- Clear logs through the panel and confirm history resets while globals remain intact.
- Switch between multiple stories to confirm host state persists as designed (or resets, according to spec).
- Reload Storybook and verify default state initialization and channel communication still work (no console errors).

## Timeline Estimate
- Mock utilities: 1 day (with type alignment and tests).
- Toolbar + panel addon: 1–1.5 days (UI + communication).
- Storybook integration + polishing: 0.5 day.
- Documentation and tests: 0.5 day.
- Total: ~3 days of focused effort.
