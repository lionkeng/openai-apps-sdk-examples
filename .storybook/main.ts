import type { StorybookConfig } from "@storybook/react-vite";
import { mergeConfig } from "vite";

const config: StorybookConfig = {
  stories: ["../src/**/*.stories.@(tsx|mdx)"],
  addons: [
    "@storybook/addon-essentials",
    "@storybook/addon-interactions",
    "@storybook/addon-a11y",
    "@storybook/addon-viewport",
  ],
  framework: {
    name: "@storybook/react-vite",
    options: {},
  },
  features: {
    experimentalRSC: false,
  },
  docs: {
    autodocs: "tag",
  },
  async viteFinal(config, { configType }) {
    const { default: tailwindcss } = await import("@tailwindcss/vite");
    return mergeConfig(config, {
      plugins: [tailwindcss()],
      define: {
        "process.env.NODE_ENV": JSON.stringify(
          process.env.NODE_ENV ??
            (configType === "PRODUCTION" ? "production" : "development")
        ),
      },
      esbuild: {
        jsx: "automatic",
        jsxImportSource: "react",
        target: "es2022",
      },
      build: {
        target: "es2022",
      },
    });
  },
};

export default config;
