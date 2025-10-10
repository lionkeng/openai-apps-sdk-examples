import type { TestRunnerConfig } from "@storybook/test-runner";

const config: TestRunnerConfig = {
  async setup() {
    // Polyfills or global test hooks for Storybook interaction tests can live here.
  },
};

export default config;
