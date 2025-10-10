# AGENTS.md

This file provides guidance when working with code in this repository.

## Overview

This repository showcases example UI widgets (components) for the OpenAI Apps SDK, along with example MCP (Model Context Protocol) servers that expose these widgets as tools. The MCP servers can be integrated with ChatGPT to render rich UI components alongside assistant messages.

## OpenAI Apps SDK Documentation

When research is needed, reference the [documentation for Apps SDK](https://developers.openai.com/apps-sdk)

## Architecture

### Widget System

- **Widget source**: [src/](src/) contains React-based widget implementations (pizzaz, pizzaz-carousel, pizzaz-list, pizzaz-albums, solar-system, todo)
- **Build system**: [build-all.mts](build-all.mts) orchestrates Vite builds for all widget entrypoints
- **Outputs**: Built assets are placed in [assets/](assets/) as hashed `.html`, `.js`, and `.css` bundles
- **Widget communication**: Widgets use `window.openai` global API to interact with ChatGPT host ([src/types.ts](src/types.ts))

### MCP Servers

The repository includes three MCP server implementations that serve widgets:

1. **pizzaz_server_node/** - TypeScript MCP server using official MCP SDK
2. **pizzaz_server_python/** - Python MCP server using FastMCP
3. **solar-system_server_python/** - Python MCP server for 3D solar system widget

Each server returns:

- Plain text content
- Structured JSON data
- `_meta.openai/outputTemplate` metadata for widget hydration

## Common Commands

### Development

```bash
# Install dependencies (uses pnpm workspace)
pnpm install

# Build all widgets (produces versioned bundles in assets/)
pnpm run build

# Development server with hot reload (serves at http://localhost:4444)
pnpm run dev

# Serve static assets after build (http://localhost:4444 with CORS)
pnpm run serve

# Type checking
pnpm run tsc          # All tsconfig
pnpm run tsc:app      # App code only
pnpm run tsc:node     # Node config only
```

### Running MCP Servers

**Pizzaz Node server:**

```bash
cd pizzaz_server_node
pnpm start
```

**Pizzaz Python server:**

```bash
# Create venv if needed
python -m venv .venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate

# Install dependencies
pip install -r pizzaz_server_python/requirements.txt

# Run server
uvicorn pizzaz_server_python.main:app --port 8000
```

**Solar system Python server:**

```bash
# Reuse same venv or create new one
source .venv/bin/activate

# Install dependencies
pip install -r solar-system_server_python/requirements.txt

# Run server
uvicorn solar-system_server_python.main:app --port 8000
```

### Testing with ChatGPT

To test locally with ChatGPT:

1. Enable developer mode at https://platform.openai.com/docs/guides/developer-mode
2. Start your MCP server
3. Use ngrok to expose local server: `ngrok http 8000`
4. Add the ngrok URL to ChatGPT Settings > Connectors: `https://<endpoint>.ngrok-free.app/mcp`

## Widget Development

### Creating New Widgets

1. Create a new directory in [src/](src/) with an `index.tsx` or `index.jsx` file
2. The build system auto-discovers entries matching `src/**/index.{tsx,jsx}`
3. Run `pnpm run build` to generate the bundled assets
4. Update MCP server to reference the new widget's HTML/JS/CSS

### Widget Structure

Widgets must:

- Export a default component or named `App` export
- Use the `window.openai` global for communication with ChatGPT
- Access tool inputs via `window.openai.toolInput`
- Access tool outputs via `window.openai.toolOutput`
- Update widget state via `window.openai.setWidgetState()`

Key hooks available:

- `useOpenAiGlobal()` - Access OpenAI globals ([src/use-openai-global.ts](src/use-openai-global.ts))
- `useWidgetProps()` - Access widget props ([src/use-widget-props.ts](src/use-widget-props.ts))
- `useWidgetState()` - Manage widget state ([src/use-widget-state.ts](src/use-widget-state.ts))
- `useDisplayMode()` - Current display mode (pip/inline/fullscreen) ([src/use-display-mode.ts](src/use-display-mode.ts))

### Build Output

[build-all.mts](build-all.mts) produces:

- Per-widget JS bundles: `{name}-{hash}.js`
- Per-widget CSS bundles: `{name}-{hash}.css`
- Self-contained HTML files: `{name}-{hash}.html` (with inlined CSS/JS)

Hash is derived from package.json version for cache-busting.

## MCP Server Implementation

### Node Server ([pizzaz_server_node/src/server.ts](pizzaz_server_node/src/server.ts))

Uses `@modelcontextprotocol/sdk`:

- Implements list_tools, call_tool, list_resources, read_resource
- SSE transport on `/mcp` endpoint
- POST messages on `/mcp/messages?sessionId=...`
- Default port: 8000 (configurable via PORT env var)

### Python Servers

Use `mcp[fastapi]` package:

- FastMCP for simplified MCP implementation
- Uvicorn ASGI server
- SSE + HTTP endpoints similar to Node implementation

## Key Configuration Files

- [vite.config.mts](vite.config.mts) - Main Vite config with multi-entry dev endpoints
- [vite.host.config.mts](vite.host.config.mts) - Alternative host config
- [tailwind.config.ts](tailwind.config.ts) - Tailwind CSS configuration
- [tsconfig.json](tsconfig.json) - TypeScript root config
- [pnpm-workspace.yaml](pnpm-workspace.yaml) - PNPM workspace configuration

## Repository Structure

```
.
├── src/                    # Widget source code
│   ├── pizzaz/            # Main pizzaz map widget
│   ├── pizzaz-carousel/   # Carousel widget
│   ├── pizzaz-list/       # List widget
│   ├── pizzaz-albums/     # Albums widget
│   ├── solar-system/      # 3D solar system
│   ├── todo/              # Todo widget
│   └── types.ts           # TypeScript types for OpenAI globals
├── assets/                # Built widget bundles (generated)
├── pizzaz_server_node/    # TypeScript MCP server
├── pizzaz_server_python/  # Python MCP server
├── solar-system_server_python/ # Solar system Python server
├── build-all.mts          # Build orchestrator
└── dev-all.mts            # Dev server with proxy
```
