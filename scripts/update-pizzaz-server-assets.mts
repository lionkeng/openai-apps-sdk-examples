#!/usr/bin/env node

import fs from "node:fs/promises";
import path from "node:path";
import fg from "fast-glob";
import { fileURLToPath } from "node:url";

const assetHost = process.env.ASSET_HOST ?? "http://localhost:4444";
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const repoRoot = path.resolve(__dirname, "..");
const assetsDir = path.join(repoRoot, "assets");
const serverPath = path.join(repoRoot, "pizzaz_server_node", "src", "server.ts");

async function main() {
  const [serverSource, assetsExists] = await Promise.all([
    fs.readFile(serverPath, "utf8"),
    fs
      .access(assetsDir)
      .then(() => true)
      .catch(() => false),
  ]);

  if (!assetsExists) {
    throw new Error(
      `Expected assets directory at ${assetsDir}. Run "pnpm run build" before updating the server assets.`,
    );
  }

  const cssPattern =
    /<link\s+rel=["']stylesheet["'][^>]*href=["']([^"']+)["']/gi;
  const jsPattern =
    /<script\s+type=["']module["'][^>]*src=["']([^"']+)["']/gi;

  let updatedSource = serverSource;
  const assetCache = new Map<string, string | null>();

  const findAsset = (base: string, ext: "css" | "js") => {
    const key = `${base}.${ext}`;
    if (assetCache.has(key)) {
      return assetCache.get(key)!;
    }

    const pattern = `${base}-[0-9a-f][0-9a-f][0-9a-f][0-9a-f].${ext}`;
    const matches = fg.sync(pattern, {
      cwd: assetsDir,
      onlyFiles: true,
      unique: true,
      dot: false,
      caseSensitiveMatch: true,
    });

    if (matches.length === 0) {
      assetCache.set(key, null);
      return null;
    }

    if (matches.length > 1) {
      throw new Error(
        `Expected exactly one ${ext.toUpperCase()} asset for "${base}", found ${matches.length}.` +
          ` Looked for pattern ${path.join("assets", pattern)}.`,
      );
    }

    const filename = matches[0];
    assetCache.set(key, filename);
    return filename;
  };

  const getBaseName = (url: string, ext: "css" | "js") => {
    let pathname = url;
    try {
      pathname = new URL(url).pathname;
    } catch {
      // Not an absolute URL; treat as path relative to assets
    }

    const filename = path.posix.basename(pathname);
    const match = /^([a-z0-9-]+)-[0-9a-f]{4}\.(css|js)$/i.exec(filename);
    if (!match) {
      return null;
    }
    const [, base, foundExt] = match;
    if (foundExt.toLowerCase() !== ext) {
      return null;
    }
    return base;
  };

  const replaceHref = (match: string, href: string, ext: "css" | "js") => {
    const base = getBaseName(href, ext);
    if (!base) {
      return match;
    }
    const asset = findAsset(base, ext);
    if (!asset) {
      return match;
    }
    const assetUrl = `${assetHost}/${asset.replace(/\\/g, "/")}`;
    return match.replace(href, assetUrl);
  };

  updatedSource = updatedSource.replace(cssPattern, (match, href) =>
    replaceHref(match, href, "css"),
  );
  updatedSource = updatedSource.replace(jsPattern, (match, src) =>
    replaceHref(match, src, "js"),
  );

  if (updatedSource === serverSource) {
    console.log("Server asset references already up to date.");
    return;
  }

  await fs.writeFile(serverPath, updatedSource, "utf8");
  console.log(
    `Updated localhost asset references in ${path.relative(repoRoot, serverPath)}.`,
  );
}

main().catch((err) => {
  console.error(err instanceof Error ? err.message : err);
  process.exitCode = 1;
});
