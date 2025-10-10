import type { Preview, StoryContext, StoryFn } from "@storybook/react";
import "./openai-addon/preview-hooks";
import { MockOpenAiProvider } from "../src/testing/openai-storybook";
import type { OpenAiGlobals } from "../src/types";

import "../src/index.css";

type GlobalsContext = StoryContext & {
  globals: StoryContext["globals"] & {
    openAiTheme?: OpenAiGlobals["theme"];
    openAiDisplayMode?: OpenAiGlobals["displayMode"];
    openAiLocale?: OpenAiGlobals["locale"];
  };
};

const withOpenAiProvider = (Story: StoryFn, context: GlobalsContext) => {
  const { openAiTheme, openAiDisplayMode, openAiLocale } = context.globals;

  const globals: Partial<OpenAiGlobals> = {};
  if (openAiTheme) globals.theme = openAiTheme;
  if (openAiDisplayMode) globals.displayMode = openAiDisplayMode;
  if (openAiLocale) globals.locale = openAiLocale;

  return (
    <MockOpenAiProvider globals={globals}>
      <Story />
    </MockOpenAiProvider>
  );
};

export const globalTypes = {
  openAiTheme: {
    name: "Theme",
    description: "OpenAI host theme value",
    defaultValue: "light",
    toolbar: {
      icon: "circlehollow",
      items: [
        { value: "light", title: "Light" },
        { value: "dark", title: "Dark" },
      ],
    },
  },
  openAiDisplayMode: {
    name: "Display Mode",
    description: "Widget display mode reported by the host",
    defaultValue: "inline",
    toolbar: {
      icon: "mirror",
      items: [
        { value: "pip", title: "PiP" },
        { value: "inline", title: "Inline" },
        { value: "fullscreen", title: "Fullscreen" },
      ],
    },
  },
  openAiLocale: {
    name: "Locale",
    description: "Host locale value exposed to the widget",
    defaultValue: "en-US",
    toolbar: {
      icon: "globe",
      items: [
        { value: "en-US", title: "English (US)" },
        { value: "en-GB", title: "English (UK)" },
        { value: "fr-FR", title: "French" },
        { value: "ja-JP", title: "Japanese" },
      ],
    },
  },
};

const preview: Preview = {
  decorators: [withOpenAiProvider],
  parameters: {
    actions: { argTypesRegex: "^on[A-Z].*" },
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/,
      },
    },
    layout: "fullscreen",
  },
};

export default preview;
