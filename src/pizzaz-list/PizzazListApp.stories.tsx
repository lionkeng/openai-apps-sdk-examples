import type { Meta, StoryObj } from "@storybook/react";
import { userEvent, within } from "@storybook/test";
import { MockOpenAiProvider } from "../testing/openai-storybook";
import { PizzazListApp } from "./PizzazListApp";

const meta: Meta<typeof PizzazListApp> = {
  title: "Widgets/Pizzaz List",
  component: PizzazListApp,
  parameters: {
    layout: "fullscreen",
  },
};

export default meta;

type Story = StoryObj<typeof PizzazListApp>;

export const Default: Story = {
  name: "Top Seven",
  play: async ({ canvasElement, step }) => {
    const canvas = within(canvasElement);
    await step("Click Save button", async () => {
      await userEvent.click(canvas.getAllByText("Save List")[0]);
    });
  },
};

export const Empty: Story = {
  name: "Empty State",
  decorators: [
    (StoryComponent) => (
      <MockOpenAiProvider globals={{ toolOutput: { city: "__no_match__" } }}>
        <StoryComponent />
      </MockOpenAiProvider>
    ),
  ],
};

export const CityNorthBeach: Story = {
  name: "City: North Beach",
  decorators: [
    (StoryComponent) => (
      <MockOpenAiProvider globals={{ toolOutput: { city: "North Beach" } }}>
        <StoryComponent />
      </MockOpenAiProvider>
    ),
  ],
};
