# Storybook Integration Plan

## Objectives
- Stand up a Storybook workspace that renders widgets from `src/` with a controllable `window.openai` mock so we can visually and interactively verify behaviour.
- Enable automated interaction tests against the same stories to use Storybook as an integration harness.
- Keep the configuration aligned with the existing Vite + React 19 toolchain and Tailwind setup.

## 1. Add Storybook Dependencies
- Install Storybook packages with pnpm:
- `pnpm add -D @storybook/react @storybook/react-vite @storybook/test @storybook/addon-essentials @storybook/addon-interactions storybook @storybook/addon-a11y`
  - Add `@storybook/blocks` if we want MDX docs later.
- Add optional testing helpers:
- `pnpm add -D @storybook/test-runner @storybook/addon-viewport`
  - `pnpm add -D vitest @testing-library/react @testing-library/user-event @testing-library/jest-dom` (reuseable for unit tests).
- Update `package.json` scripts:
  - `"storybook": "storybook dev --port 6006"`
  - `"build:storybook": "storybook build"`
  - `"test:storybook": "storybook test --ci"` (for the test runner) and optionally `"test:storybook:watch"` for local iteration.

## 2. Scaffold Storybook Directory
- Create `.storybook/main.ts` and `.storybook/preview.tsx`.
- `main.ts`:
  - Use Vite builder: `framework: { name: "@storybook/react-vite", options: {} }`.
  - Point to stories: `stories: ["../src/**/*.stories.@(tsx|mdx)"]`.
  - Extend Vite config by merging with `vite.config.mts` so aliases, Tailwind, and shared plugins stay consistent (`import { mergeConfig }` from `vite`).
  - Ensure the JSX runtime matches React 19 (automatic).
  - Register addons: `["@storybook/addon-essentials", "@storybook/addon-interactions", "@storybook/addon-a11y", "@storybook/addon-viewport"]`.
  - Configure `features: { experimentalRSC: false }` to avoid RSC since widgets expect browser APIs.
- `preview.tsx`:
  - Import global CSS (Tailwind base or widget-specific CSS) via `import "../src/index.css";` (create if needed).
  - Export decorators to install the `window.openai` mock provider (defined in Step 3).
  - Set Storybook parameters (layout, controls, interactions).
- Add `.storybook/tsconfig.json` that extends the root `tsconfig.json` to ensure path aliases remain valid.

## 3. Build an OpenAI Host Mock
- Create `src/testing/openai-storybook.ts`:
  - Define the shared mock openai object implementing the full host surface described in the [OpenAI Apps SDK reference](https://developers.openai.com/apps-sdk/reference). Mirror both the *globals* contract and the *imperative APIs* so widgets behave identically to the ChatGPT host.
    - **Host-provided globals (`OpenAiGlobals` values)**
      - `theme: "light" | "dark"` – keep in sync with tailwind/light/dark variants; expose control so stories can toggle.
      - `userAgent` – populate realistic defaults (`{ device: { type: "desktop" }, capabilities: { hover: true, touch: false } }`), but allow overrides to simulate mobile/touch contexts.
      - `locale: string` – feed BCP‑47 locale codes to test localisation or number formatting.
      - `maxHeight: number` – widgets may clamp layouts to this value; let tests tweak it to exercise overflow handling.
      - `displayMode: "pip" | "inline" | "fullscreen"` – Storybook controls should drive this to validate responsive states (pip/inline/fullscreen).
      - `safeArea: { insets: { top/right/bottom/left: number } }` – default to zeros, but support notch-style padding stories.
      - `toolInput` – the original tool invocation payload; initialise with per-story fixtures where components expect request context.
      - `toolOutput` – current tool response data (primary props); treat as mutable JSON to hydrate widgets.
      - `toolResponseMetadata` – optional metadata envelope; persist whatever shape stories/tests need.
      - `widgetState` – host-persisted widget state, kept in sync with `setWidgetState`.
      - `setWidgetState(state): Promise<void>` – although part of `OpenAiGlobals`, it is the bridge back to the host. Mock by updating the stored `widgetState`, dispatching `SetGlobalsEvent`, and resolving a promise to mimic async acknowledgement.
    - **Command APIs (imperative host methods)**
      - `callTool(name, args): Promise<{ result: string }>` – stub with a vi/jest mock; support per-story overrides to script responses.
      - `sendFollowUpMessage({ prompt }): Promise<void>` – capture invocations for assertions or Storybook actions panel.
      - `openExternal({ href }): void` – optionally forward to `window.open` in dev, but default to a spy.
      - `requestDisplayMode({ mode }): Promise<{ mode: DisplayMode }>` – echo the requested mode by default; provide hooks to reject/alter the response for failure scenarios.
  - Expose reusable helpers:
    - `installOpenAiMock(initial?: Partial<OpenAiGlobals & API>)` – define `window.openai` with sensible defaults and reset any spies.
    - `updateOpenAiGlobals(partial: Partial<OpenAiGlobals>)` – mutate the backing object and dispatch `new SetGlobalsEvent({ detail: { globals: partial } })` so hooks subscribed via `useSyncExternalStore` detect changes.
    - `simulateToolResponse(partial)` (optional) – convenience wrapper around `updateOpenAiGlobals` for common tool output updates.
    - `uninstallOpenAiMock()` – remove the property if a story/test needs isolation.
- Export a React `MockOpenAiProvider`:
  ```tsx
  export const MockOpenAiProvider: React.FC<{ globals?: Partial<OpenAiGlobals> }> = ({ globals, children }) => {
    useEffect(() => {
      installOpenAiMock();
      if (globals) updateOpenAiGlobals(globals);
      return () => uninstallOpenAiMock();
    }, [globals]);
    return <>{children}</>;
  };
  ```
- In `.storybook/preview.tsx`, wrap every story with the provider via a global decorator and expose Storybook global controls to tweak select fields (e.g., `displayMode`, `theme`, `widgetState` JSON, `toolOutput` JSON).
- Optionally add a toolbar button to simulate host events (`preview.tsx`: `globalTypes` + custom toolbar icon).

## 4. Adapt Widgets for Reuse in Stories
- Refactor widget entry files (`src/**/index.tsx|jsx`) so the core component is exportable without immediately touching the DOM:
  - Export the component from the entry module and guard the `createRoot(...).render(...)` call behind a `typeof document !== "undefined"` check so imports from Storybook/tests don't mount automatically.
  - Prefer named + default exports (`export function WidgetApp…` and `export default WidgetApp`) so stories have a stable import surface.
- Ensure each widget exports either a default `App` component or story-friendly hooks to load sample data.
- Create central fixtures (`src/testing/fixtures.ts`) for inputs like `markers` so stories can import them without duplicating data loading.

## 5. Author Stories
- For each widget directory:
  - Add `ComponentName.stories.tsx` next to the widget component.
  - Provide default story that renders with realistic tool input/output via the OpenAI mock helper:
    ```tsx
    const meta: Meta<typeof PizzazListApp> = {
      component: PizzazListApp,
      decorators: [
        (Story) => (
          <MockOpenAiProvider globals={{ toolOutput: sampleOutput }}>
            <Story />
          </MockOpenAiProvider>
        ),
      ],
    };
    ```
  - Define additional stories for edge cases: empty lists, alternate themes, fullscreen display mode.
- Use controls knobs where appropriate (e.g., `args` with `widgetState` to simulate updates) and map interactions to the mock helper so `args` changes update `window.openai`.

## 6. Interaction & Visual Testing
- Enable the interactions addon (`@storybook/addon-interactions`) and write `play` functions inside stories to simulate user flows:
  ```tsx
  export const SaveList = {
    play: async ({ canvasElement, step }) => {
      const canvas = within(canvasElement);
      await step("Click Save", async () => {
        await userEvent.click(canvas.getByText("Save List"));
      });
    },
  };
  ```
- Configure `@storybook/test-runner`:
  - Add `storyblok/test-runner` to `package.json`.
  - Create `test-runner.ts` for custom setup if we need to polyfill APIs (e.g., `ResizeObserver`).
  - Add a pnpm script `storybook:test` that runs `pnpm storybook test --ci`.
- For higher-fidelity visual checks, consider:
  - `pnpm add -D @storybook/telemetry playwright`.
  - Add a Playwright suite that hits the built Storybook via `@storybook/test-runner/playwright`.
  - Optional: integrate Chromatic or Loki for visual diffs after base harness works.

## 7. Continuous Integration Hooks
- Update CI workflow (if present) to cache `node_modules/.pnpm` and run:
  - `pnpm install`
  - `pnpm run build` (existing)
  - `pnpm run build:storybook`
  - `pnpm run storybook:test` (headless interaction tests).
- Fail CI if Stories or tests break to keep widgets aligned with host expectations.

## 8. Developer Ergonomics
- Document usage in `README.md`:
  - How to start Storybook.
  - How to tweak OpenAI globals inside the toolbar.
  - How to write new stories and share fixtures.
- Optionally add a `pnpm run storybook:ci` script that builds and uploads to Chromatic (if adopted).
- Provide VS Code launch configuration for Storybook dev server and attach instructions for debugging play functions.

## 9. Future Enhancements
- Add Storybook docs pages (MDX) explaining the integration between widgets and MCP servers.
- Wire Storybook stories into MCP server responses during local development to ensure output templates stay in sync.
- Explore generating snapshots from Storybook to use as golden references for MCP `_meta.openai/outputTemplate` payloads.
