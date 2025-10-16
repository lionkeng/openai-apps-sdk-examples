import { build, type InlineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";
import fg from "fast-glob";
import path from "path";
import fs from "fs";
import crypto from "crypto";
import pkg from "./package.json" with { type: "json" };
import tailwindcss from "@tailwindcss/vite";

type WidgetCatalogEntry = {
  id: string;
  bundleName?: string;
  title: string;
  templateUri: string;
  invoking: string;
  invoked: string;
  responseText: string;
  htmlOverride?: string;
  assets?: {
    html?: string;
    css?: string;
    js?: string;
  };
};

const entries = fg.sync("src/**/index.{tsx,jsx}", {
  ignore: ["**/*.stories.*"],
});
const outDir = "assets";
const outDirAbs = path.resolve(outDir);
const assetBaseUrl = (() => {
  const raw = process.env.WIDGETS_ASSET_BASE_URL ?? "http://localhost:4444/";
  return raw.endsWith("/") ? raw : `${raw}/`;
})();

const PER_ENTRY_CSS_GLOB = "**/*.{css,pcss,scss,sass}";
const PER_ENTRY_CSS_IGNORE = "**/*.module.*".split(",").map((s) => s.trim());
const GLOBAL_CSS_LIST = [path.resolve("src/index.css")];

const toPosixPath = (value: string) => value.split(path.sep).join("/");

function normalizeAssetReference(value: string | undefined): string | undefined {
  if (!value) {
    return value;
  }
  const trimmed = value.trim();
  if (isRemotePath(trimmed)) {
    return trimmed;
  }

  const withoutPrefix = trimmed.replace(new RegExp(`^${outDir}[\\\\/]?`), "");
  const withoutLeading = withoutPrefix.replace(/^\.?[\\/]?/, "");
  return toPosixPath(withoutLeading);
}

function toAbsoluteAssetPath(value: string) {
  if (path.isAbsolute(value)) {
    return value;
  }

  const normalized = normalizeAssetReference(value) ?? value;
  return path.resolve(outDir, normalized);
}

function toAssetUrl(value: string) {
  if (isRemotePath(value)) {
    return value;
  }
  const normalized = normalizeAssetReference(value) ?? value;
  return new URL(normalized, assetBaseUrl).toString();
}

const widgetCatalog: WidgetCatalogEntry[] = [
  {
    id: "pizza-map",
    bundleName: "pizzaz",
    title: "Show Pizza Map",
    templateUri: "ui://widget/pizza-map.html",
    invoking: "Hand-tossing a map",
    invoked: "Served a fresh map",
    responseText: "Rendered a pizza map!",
  },
  {
    id: "pizza-carousel",
    bundleName: "pizzaz-carousel",
    title: "Show Pizza Carousel",
    templateUri: "ui://widget/pizza-carousel-2d2b.html",
    invoking: "Carousel some spots",
    invoked: "Served a fresh carousel",
    responseText: "Rendered a pizza carousel!",
  },
  {
    id: "pizza-albums",
    bundleName: "pizzaz-albums",
    title: "Show Pizza Album",
    templateUri: "ui://widget/pizza-albums-2d2b.html",
    invoking: "Hand-tossing an album",
    invoked: "Served a fresh album",
    responseText: "Rendered a pizza album!",
  },
  {
    id: "pizza-list",
    bundleName: "pizzaz-list",
    title: "Show Pizza List",
    templateUri: "ui://widget/pizza-list-2d2b.html",
    invoking: "Hand-tossing a list",
    invoked: "Served a fresh list",
    responseText: "Rendered a pizza list!",
  },
  {
    id: "pizza-video",
    title: "Show Pizza Video",
    templateUri: "ui://widget/pizza-video.html",
    invoking: "Hand-tossing a video",
    invoked: "Served a fresh video",
    responseText: "Rendered a pizza video!",
    htmlOverride: `
<div id="pizzaz-video-root"></div>
<link rel="stylesheet" href="https://persistent.oaistatic.com/ecosystem-built-assets/pizzaz-video-0038.css">
<script type="module" src="https://persistent.oaistatic.com/ecosystem-built-assets/pizzaz-video-0038.js"></script>
    `.trim(),
    assets: {
      css: "https://persistent.oaistatic.com/ecosystem-built-assets/pizzaz-video-0038.css",
      js: "https://persistent.oaistatic.com/ecosystem-built-assets/pizzaz-video-0038.js",
    },
  },
];

const additionalTargets = ["todo", "solar-system"];

const targets: string[] = [
  ...new Set([
    ...widgetCatalog
      .map((widget) => widget.bundleName)
      .filter((name): name is string => !!name),
    ...additionalTargets,
  ]),
];
const builtNames: string[] = [];

function wrapEntryPlugin(
  virtualId: string,
  entryFile: string,
  cssPaths: string[]
): Plugin {
  return {
    name: `virtual-entry-wrapper:${entryFile}`,
    resolveId(id) {
      if (id === virtualId) return id;
    },
    load(id) {
      if (id !== virtualId) {
        return null;
      }

      const cssImports = cssPaths
        .map((css) => `import ${JSON.stringify(css)};`)
        .join("\n");

      return `
    ${cssImports}
    export * from ${JSON.stringify(entryFile)};

    import * as __entry from ${JSON.stringify(entryFile)};
    export default (__entry.default ?? __entry.App);

    import ${JSON.stringify(entryFile)};
  `;
    },
  };
}

fs.rmSync(outDir, { recursive: true, force: true });

for (const file of entries) {
  const name = path.basename(path.dirname(file));
  if (targets.length && !targets.includes(name)) {
    continue;
  }

  const entryAbs = path.resolve(file);
  const entryDir = path.dirname(entryAbs);

  // Collect CSS for this entry using the glob(s) rooted at its directory
  const perEntryCss = fg.sync(PER_ENTRY_CSS_GLOB, {
    cwd: entryDir,
    absolute: true,
    dot: false,
    ignore: PER_ENTRY_CSS_IGNORE,
  });

  // Global CSS (Tailwind, etc.), only include those that exist
  const globalCss = GLOBAL_CSS_LIST.filter((p) => fs.existsSync(p));

  // Final CSS list (global first for predictable cascade)
  const cssToInclude = [...globalCss, ...perEntryCss].filter((p) =>
    fs.existsSync(p)
  );

  const virtualId = `\0virtual-entry:${entryAbs}`;

  const createConfig = (): InlineConfig => ({
    plugins: [
      wrapEntryPlugin(virtualId, entryAbs, cssToInclude),
      tailwindcss(),
      react(),
      {
        name: "remove-manual-chunks",
        outputOptions(options) {
          if ("manualChunks" in options) {
            delete (options as any).manualChunks;
          }
          return options;
        },
      },
    ],
    esbuild: {
      jsx: "automatic",
      jsxImportSource: "react",
      target: "es2022",
    },
    build: {
      target: "es2022",
      outDir,
      emptyOutDir: false,
      chunkSizeWarningLimit: 2000,
      minify: "esbuild",
      cssCodeSplit: false,
      rollupOptions: {
        input: virtualId,
        output: {
          format: "es",
          entryFileNames: `${name}.js`,
          inlineDynamicImports: true,
          assetFileNames: (info) =>
            (info.name || "").endsWith(".css")
              ? `${name}.css`
              : `[name]-[hash][extname]`,
        },
        preserveEntrySignatures: "allow-extension",
        treeshake: true,
      },
    },
  });

  console.group(`Building ${name} (react)`);
  await build(createConfig());
  console.groupEnd();
  builtNames.push(name);
  console.log(`Built ${name}`);
}

const outputs = fs
  .readdirSync(outDirAbs)
  .filter((f) => f.endsWith(".js") || f.endsWith(".css"))
  .map((f) => path.join(outDirAbs, f))
  .filter((p) => fs.existsSync(p));

const renamed = [];
const bundleArtifacts = new Map<
  string,
  {
    htmlPath: string;
    htmlRelative: string;
    cssPath?: string;
    cssRelative?: string;
    jsPath?: string;
    jsRelative?: string;
    htmlContent: string;
  }
>();

const h = crypto
  .createHash("sha256")
  .update(pkg.version, "utf8")
  .digest("hex")
  .slice(0, 4);

console.group("Hashing outputs");
for (const out of outputs) {
  const dir = path.dirname(out);
  const ext = path.extname(out);
  const base = path.basename(out, ext);
  const newName = path.join(dir, `${base}-${h}${ext}`);

  fs.renameSync(out, newName);
  renamed.push({ old: out, neu: newName });
  console.log(`${out} -> ${newName}`);
}
console.groupEnd();

console.log("new hash: ", h);

for (const name of builtNames) {
  const htmlPath = path.join(outDirAbs, `${name}-${h}.html`);
  const cssPath = path.join(outDirAbs, `${name}-${h}.css`);
  const jsPath = path.join(outDirAbs, `${name}-${h}.js`);

  const css = fs.existsSync(cssPath)
    ? fs.readFileSync(cssPath, { encoding: "utf8" })
    : "";
  const js = fs.existsSync(jsPath)
    ? fs.readFileSync(jsPath, { encoding: "utf8" })
    : "";

  const cssBlock = css ? `\n  <style>\n${css}\n  </style>\n` : "";
  const jsBlock = js ? `\n  <script type="module">\n${js}\n  </script>` : "";

  const html = [
    "<!doctype html>",
    "<html>",
    `<head>${cssBlock}</head>`,
    "<body>",
    `  <div id="${name}-root"></div>${jsBlock}`,
    "</body>",
    "</html>",
  ].join("\n");

  fs.writeFileSync(htmlPath, html, { encoding: "utf8" });
  const cssExists = fs.existsSync(cssPath);
  const jsExists = fs.existsSync(jsPath);

  bundleArtifacts.set(name, {
    htmlPath,
    htmlRelative: normalizeAssetReference(`${name}-${h}.html`) ?? `${name}-${h}.html`,
    cssPath: cssExists ? cssPath : undefined,
    cssRelative: cssExists
      ? normalizeAssetReference(`${name}-${h}.css`) ?? `${name}-${h}.css`
      : undefined,
    jsPath: jsExists ? jsPath : undefined,
    jsRelative: jsExists
      ? normalizeAssetReference(`${name}-${h}.js`) ?? `${name}-${h}.js`
      : undefined,
    htmlContent: html,
  });
  console.log(`${htmlPath} (generated)`);
}

function isRemotePath(candidate: string | undefined): candidate is string {
  return (
    typeof candidate === "string" &&
    /^(https?:)?\/\//i.test(candidate.trim())
  );
}

function ensureReadable(assetPath: string) {
  try {
    fs.accessSync(assetPath, fs.constants.R_OK);
  } catch (error) {
    throw new Error(
      `Asset ${assetPath} is not readable or does not exist: ${String(error)}`
    );
  }
}

const manifestWidgets = widgetCatalog
  .map((widget) => {
    let artifacts = widget.bundleName
      ? bundleArtifacts.get(widget.bundleName)
      : undefined;

    if (widget.bundleName && !artifacts) {
      throw new Error(
        `Expected artifacts for bundle "${widget.bundleName}" but none were generated`
      );
    }

    if (!artifacts && widget.htmlOverride) {
      const fileName = `${widget.id}-${h}.html`;
      const htmlPath = path.join(outDirAbs, fileName);
      const htmlDocument = [
        "<!doctype html>",
        "<html>",
        "<head></head>",
        "<body>",
        widget.htmlOverride,
        "</body>",
        "</html>",
      ].join("\n");

      fs.writeFileSync(htmlPath, htmlDocument, { encoding: "utf8" });
      artifacts = {
        htmlPath,
        htmlRelative: normalizeAssetReference(fileName) ?? fileName,
        htmlContent: htmlDocument,
      };
      console.log(`${htmlPath} (generated from override)`);
    }

    const htmlAssetRef =
      artifacts?.htmlRelative ?? normalizeAssetReference(widget.assets?.html);
    const cssAssetRef =
      artifacts?.cssRelative ?? normalizeAssetReference(widget.assets?.css);
    const jsAssetRef =
      artifacts?.jsRelative ?? normalizeAssetReference(widget.assets?.js);

    if (!htmlAssetRef) {
      throw new Error(
        `Widget "${widget.id}" is missing an HTML asset or override`
      );
    }

    for (const candidate of [htmlAssetRef, cssAssetRef, jsAssetRef]) {
      if (!candidate || isRemotePath(candidate)) {
        continue;
      }
      ensureReadable(toAbsoluteAssetPath(candidate));
    }

    const htmlUrl = toAssetUrl(htmlAssetRef);

    return {
      id: widget.id,
      title: widget.title,
      templateUri: widget.templateUri,
      invoking: widget.invoking,
      invoked: widget.invoked,
      html: htmlUrl,
      responseText: widget.responseText,
      assets: {
        html: htmlAssetRef,
        css: cssAssetRef,
        js: jsAssetRef,
      },
    };
  })
  .sort((a, b) => a.id.localeCompare(b.id));

const manifest = {
  schemaVersion: "1.0.0",
  generatedAt: new Date().toISOString(),
  widgets: manifestWidgets,
};

const manifestPath = path.join(outDir, "widgets.json");
const manifestTempPath = `${manifestPath}.tmp`;

try {
  fs.writeFileSync(manifestTempPath, JSON.stringify(manifest, null, 2), {
    encoding: "utf8",
  });
  fs.renameSync(manifestTempPath, manifestPath);
  console.log(`${manifestPath} (generated)`);
} catch (error) {
  fs.rmSync(manifestTempPath, { force: true });
  throw error;
}
