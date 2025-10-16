# Widgets Manifest

The widgets manifest aggregates all build outputs that can be served by MCP servers in a single JSON document. It is generated automatically by `pnpm run build` (which runs `build-all.mts`) and written to `assets/widgets.json`.

The manifest always contains a `schemaVersion` value so MCP servers can validate compatibility. The initial version is `1.0.0`, which is expected by the Rust server.

## Schema (`schemaVersion: "1.0.0"`)

```jsonc
{
  "schemaVersion": "1.0.0",
  "generatedAt": "2025-01-01T00:00:00.000Z",
  "widgets": [
    {
      "id": "pizza-map",
      "title": "Show Pizza Map",
      "templateUri": "ui://widget/pizza-map.html",
      "invoking": "Hand-tossing a map",
      "invoked": "Served a fresh map",
      "responseText": "Rendered a pizza map!",
      "html": "<!doctype html> ...",
      "assets": {
        "html": "assets/pizzaz-2d2b.html",
        "css": "assets/pizzaz-2d2b.css",
        "js": "assets/pizzaz-2d2b.js"
      }
    }
  ]
}
```

### Field Descriptions

- `schemaVersion`: Semantic version for the manifest. Servers must check this before consuming the data.
- `generatedAt`: ISO-8601 timestamp recorded when the manifest was produced.
- `widgets`: Ordered list (sorted by `id`) of widget entries.
- `widgets[].id`: Tool identifier exposed through MCP.
- `widgets[].title`: Human readable description.
- `widgets[].templateUri`: Resource URI bound to the widget markup.
- `widgets[].invoking`: Status text to display while the tool is running.
- `widgets[].invoked`: Status text after completion.
- `widgets[].responseText`: Plain text response returned to the client.
- `widgets[].html`: Fully qualified URL to the widget HTML bundle. Local builds default to `http://localhost:4444/<file>.html`; production manifests should reference the CDN location.
- `widgets[].assets`: Optional relative paths (or absolute URLs) pointing to the generated asset files. Local builds store paths relative to the `assets/` directory (e.g., `pizzaz-2d2b.html`). Production manifests should replace these with CDN URLs.

## Build Guarantees

`build-all.mts` uses an atomic write strategy (`widgets.json.tmp` → `widgets.json`) and validates that every referenced local asset is present and readable before committing the manifest. If validation fails, the build stops with a descriptive error.

When new widgets are added, update `widgetCatalog` in `build-all.mts` so metadata stays in sync across the manifest and MCP servers.

## MCP Server Refresh Workflow

The Rust MCP server consumes `assets/widgets.json` at startup and exposes two internal endpoints for operators:

- `POST /internal/widgets/refresh` &mdash; Reloads the manifest without restarting the server.
- `GET /internal/widgets/status` &mdash; Reports registry health (widget count, schema version, last successful load, manifest path).

Configure the refresh endpoint via environment variables:

- `WIDGETS_MANIFEST_PATH` (optional): Override the manifest location (defaults to `assets/widgets.json`).
- `WIDGETS_REFRESH_TOKEN`: Bearer token required to access the refresh endpoint. If unset, the endpoint returns `404`.
- `WIDGETS_REFRESH_RATE_LIMIT` (optional): Rate limit in the form `count/window`, e.g. `10/60s` (default). Supports seconds (`s`) or minutes (`m`).
- `WIDGETS_ASSET_BASE_URL` (optional): Base URL used to generate the `html` URLs in the manifest (defaults to `http://localhost:4444/` for local development).

After running `pnpm run build`, you can trigger a hot reload with:

```bash
WIDGETS_REFRESH_TOKEN=... pnpm run refresh:widgets
```

The script uses `scripts/refresh-widgets.mjs`, which also accepts optional overrides:

- `WIDGETS_REFRESH_URL` or `--url <http://host:port/internal/widgets/refresh>`
- `WIDGETS_REFRESH_TOKEN` or `--token <secret>`

For local workflows, you can place these values in a project-root `.env` file; the MCP server (via `dotenvy`) and the refresh script (via `dotenv`) load it automatically.

On refresh, the server validates schema compatibility and asset availability, swapping the registry atomically only after a successful load. Failures keep the previous registry in memory and return structured error responses to the caller.
