# Widget Manifest & Refresh Implementation Plan

## 1. Manifest Format & Build Integration

- Choose a canonical JSON manifest path (e.g., `assets/widgets.json`) generated alongside widget bundles.
- Define schema fields:
  - Required: `id`, `title`, `templateUri`, `invoking`, `invoked`, `html`, `responseText`, `schemaVersion` (for forward/backward compatibility)
  - **Schema Versioning**: Start with `"schemaVersion": "1.0.0"` to allow future evolution
- Update `build-all.mts` (and related scripts) to emit the manifest after building all widgets, ensuring stable ordering for deterministic diffs.
- **Atomic Write Strategy**:
  - Write manifest to temporary file: `assets/widgets.json.tmp`
  - Validate all referenced assets exist on disk
  - Only after validation, atomically rename `widgets.json.tmp` → `widgets.json`
  - On validation failure, clean up temp file and fail the build with clear error message
- **Asset Validation**: Before finalizing manifest, verify that all `html`, `js`, `css` paths exist and are readable
- For development these are paths to the local filesystem. But for production, these are fully-qualified URLs on CDNs.
- Document the manifest schema for reuse across other MCP servers (Node, Python) to keep parity.

## 2. Rust Types & Manifest Loader

- Create `widgets_manifest.rs` with serde-deserializable structs mirroring the manifest schema:
  ```rust
  #[derive(Deserialize, Serialize, Clone)]
  struct WidgetManifest {
      schema_version: String,  // For validation
      widgets: Vec<WidgetEntry>,
  }
  ```
- Implement a loader that reads and validates the manifest during server startup, surfacing clear errors when the file is missing or malformed.
- **Schema Version Validation**: Check that `schemaVersion` is supported (e.g., `1.x.x`), warn/error on unknown versions
- Store the manifest in a `RwLock<Arc<WidgetsRegistry>>`, exposing lookup helpers by widget id or resource URI.
- Update `widgets.rs` to proxy existing public functions (`get_widget_by_id`, `get_widget_by_uri`, etc.) through the manifest-backed registry, preserving the external API.
- **Bootstrap Behavior**: On startup, if manifest file is missing, initialize with an empty `WidgetsRegistry` (don't fail startup). Log a warning that no widgets are available until manifest is loaded.

## 3. Refresh Hook & Hot Reload

### Security Model for Refresh Endpoint

- **Authentication**: Require a bearer token in the `Authorization` header
  - Token format: `Bearer <secret>` where `<secret>` is read from env var `WIDGETS_REFRESH_TOKEN`
  - If env var is not set, disable the refresh endpoint entirely (return 404) to prevent accidental exposure
  - Token should be a cryptographically secure random value (e.g., 32+ bytes, base64-encoded)
  - Document recommended generation: `openssl rand -base64 32`
- **Rate Limiting**: Implement token-bucket or fixed-window rate limiting
  - Default: max 10 refresh requests per minute per source IP
  - Configurable via env var `WIDGETS_REFRESH_RATE_LIMIT` (default: `10/60s`)
  - Return `429 Too Many Requests` with `Retry-After` header when exceeded
  - Log rate limit violations for security monitoring
- **Request Validation**: Only accept `POST` requests with no body required

### Refresh Implementation

- Add internal endpoint: `POST /internal/widgets/refresh`
- On refresh:
  1. Validate authentication token and rate limit
  2. Re-read the manifest from disk using **atomic read strategy**:
     - Read file content fully before parsing
     - If file doesn't exist or is malformed, preserve current registry
  3. Validate schema version compatibility
  4. Validate all asset paths exist on disk
  5. Build new registry object
  6. **Atomic Swap**: Use `RwLock::write()` to replace registry only after full validation succeeds
  7. Log success with widget count and manifest timestamp
- **Failure Handling**:
  - If manifest is missing: Return 503, keep existing registry, log warning
  - If manifest is malformed: Return 400, keep existing registry, log parse error
  - If assets are missing: Return 400, keep existing registry, log missing paths
  - If schema version is unsupported: Return 400, keep existing registry, log version mismatch
- Return structured JSON response:
  ```json
  {
    "success": true,
    "widgets_loaded": 10,
    "schema_version": "1.0.0",
    "manifest_timestamp": "2025-10-15T10:30:00Z"
  }
  ```
- For development, write a script that first runs `build-all.mts` via `pnpm build` and when that run successfully, send a curl command to the endpoint on localhost to trigger a refresh.

## 4. Error Handling & Observability

### Error Handling Strategy

- Emit structured tracing logs during load/refresh with manifest path, widget count, and any skipped entries.
- Handle invalid entries gracefully—log warnings and retain the last known-good registry if reload fails.
- **No Known-Good Registry Behavior** (critical for initial startup):
  - On first startup, if manifest file is missing or invalid:
    - Initialize with an **empty registry** (don't crash the server)
    - Log a clear WARNING: "No widgets available - manifest not found at {path}"
    - Set internal state: `registry_initialized = false`
    - Respond to widget lookups with empty results (graceful degradation)
    - Return 503 Service Unavailable for `/internal/widgets/refresh` with message: "Manifest has never been successfully loaded"
  - On subsequent refresh failures:
    - Keep the last successful registry in memory
    - Log ERROR: "Refresh failed - retaining previous registry with {count} widgets"
    - Return 400 with detailed error message in refresh response
  - **Health Check Indicator**: Track `last_successful_load` timestamp and `registry_widget_count`
    - Expose in diagnostics endpoint to distinguish between "never loaded" vs. "loaded but stale"

### Observability

- Consider exposing a health or diagnostics endpoint/metric (e.g., current manifest version timestamp) for operators.
- Diagnostics endpoint: `GET /internal/widgets/status` (no auth required, safe for health checks)
  ```json
  {
    "registry_initialized": true,
    "widgets_count": 10,
    "schema_version": "1.0.0",
    "last_successful_load": "2025-10-15T10:30:00Z",
    "manifest_path": "assets/widgets.json",
    "manifest_exists": true
  }
  ```

## 5. Shared Utilities & Tests

- Consolidate metadata augmentation helpers so manifest-driven data stays consistent across code paths.
- Write unit tests with fixture manifests to validate parsing, lookup behavior, and metadata augmentation.
- Add integration tests that modify the manifest on disk, trigger the refresh hook, and assert that new widgets appear without restarting the server.
- Extend documentation (e.g., `AGENTS.md` or a dedicated doc) outlining how to regenerate assets, manifest expectations, and refresh workflow.
