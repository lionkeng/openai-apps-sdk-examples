#!/usr/bin/env node

import "dotenv/config";

/**
 * Triggers the Rust MCP server's widget refresh endpoint so it reloads
 * the latest manifest generated in assets/widgets.json.
 *
 * Usage:
 *   WIDGETS_REFRESH_TOKEN=... pnpm run refresh:widgets
 *   node scripts/refresh-widgets.mjs --url http://localhost:8000/internal/widgets/refresh --token ...
 *
 * The script exits with code 0 on success and logs the response payload.
 */

import process from "node:process";

function parseArg(flag) {
  const index = process.argv.indexOf(flag);
  if (index === -1 || index + 1 >= process.argv.length) {
    return undefined;
  }
  return process.argv[index + 1];
}

const endpoint =
  parseArg("--url") ??
  process.env.WIDGETS_REFRESH_URL ??
  "http://localhost:8000/internal/widgets/refresh";

const token = parseArg("--token") ?? process.env.WIDGETS_REFRESH_TOKEN;

if (!token) {
  console.error(
    "Missing refresh token. Set WIDGETS_REFRESH_TOKEN or pass --token <value>."
  );
  process.exitCode = 1;
  process.exit();
}

async function triggerRefresh() {
  try {
    const response = await fetch(endpoint, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${token}`,
      },
    });

    const contentType = response.headers.get("content-type") ?? "";
    let bodyText = await response.text();
    let parsed;
    if (contentType.includes("application/json") && bodyText) {
      try {
        parsed = JSON.parse(bodyText);
      } catch (error) {
        console.warn("Failed to parse JSON response:", error);
      }
    }

    if (response.ok) {
      console.log("Widgets refresh succeeded.");
      if (parsed) {
        console.log(JSON.stringify(parsed, null, 2));
      } else if (bodyText) {
        console.log(bodyText);
      }
      return;
    }

    console.error(
      `Widgets refresh failed with status ${response.status} ${response.statusText}`
    );
    if (parsed) {
      console.error(JSON.stringify(parsed, null, 2));
    } else if (bodyText) {
      console.error(bodyText);
    }
    process.exitCode = 1;
  } catch (error) {
    console.error("Failed to contact widgets refresh endpoint:", error);
    process.exitCode = 1;
  }
}

triggerRefresh();
