# Widget Manifest & Refresh Implementation Plan

## 1. Manifest Format & Build Integration
- Choose a canonical JSON manifest path (e.g., `assets/widgets.json`) generated alongside widget bundles.
- Define schema fields: `id`, `label`, `html`, `js`, `css`, and optional metadata such as `description`, `thumbnail`, and `version`.
- Update `build-all.mts` (and related scripts) to emit the manifest after building all widgets, ensuring stable ordering for deterministic diffs.
- Document the manifest schema for reuse across other MCP servers (Node, Python) to keep parity.

## 2. Rust Types & Manifest Loader
- Create `widgets_manifest.rs` with serde-deserializable structs mirroring the manifest schema.
- Implement a loader that reads and validates the manifest during server startup, surfacing clear errors when the file is missing or malformed.
- Store the manifest in a `RwLock<Arc<WidgetsRegistry>>`, exposing lookup helpers by widget id or resource URI.
- Update `widgets.rs` to proxy existing public functions (`get_widget_by_id`, `get_widget_by_uri`, etc.) through the manifest-backed registry, preserving the external API.

## 3. Refresh Hook & Hot Reload
- Add an internal endpoint (e.g., `POST /internal/widgets/refresh`) or command channel guarded by config/env token to trigger manifest reloads.
- On refresh, re-read the manifest, rebuild the registry, validate entries, and swap the shared registry inside the `RwLock`.
- Return structured JSON responses with success/failure details and log diagnostics for visibility.
- Optionally, gate a filesystem watcher (using `notify` + tokio task) behind a feature flag to auto-refresh on manifest changes.

## 4. Error Handling & Observability
- Emit structured tracing logs during load/refresh with manifest path, widget count, and any skipped entries.
- Handle invalid entries gracefully—log warnings and retain the last known-good registry if reload fails.
- Consider exposing a health or diagnostics endpoint/metric (e.g., current manifest version timestamp) for operators.

## 5. Shared Utilities & Tests
- Consolidate metadata augmentation helpers so manifest-driven data stays consistent across code paths.
- Write unit tests with fixture manifests to validate parsing, lookup behavior, and metadata augmentation.
- Add integration tests that modify the manifest on disk, trigger the refresh hook, and assert that new widgets appear without restarting the server.
- Extend documentation (e.g., `AGENTS.md` or a dedicated doc) outlining how to regenerate assets, manifest expectations, and refresh workflow.

## 6. Migration & Rollout
- Provide a compatibility fallback: if the manifest is absent during rollout, load the legacy static `WIDGETS` list to avoid downtime.
- Coordinate updates with other server implementations so they consume the shared manifest file.
- Validate end-to-end: rebuild widget assets, trigger the refresh hook in a non-production environment, and ensure MCP responses reflect updated widget metadata without restarting the process.
