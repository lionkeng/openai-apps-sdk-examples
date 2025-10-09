import React from "react";
import { addons, types } from "@storybook/manager-api";
import { AddonPanel } from "@storybook/components";
import Toolbar from "./Toolbar";
import Panel from "./Panel";

const ADDON_ID = "openai/toolbar";
const TOOL_ID = `${ADDON_ID}/tool`;
const PANEL_ID = `${ADDON_ID}/panel`;

addons.register(ADDON_ID, () => {
  addons.add(TOOL_ID, {
    type: types.TOOL,
    title: "OpenAI",
    match: ({ viewMode }) => Boolean(viewMode),
    render: () => <Toolbar />,
  });

  addons.add(PANEL_ID, {
    type: types.PANEL,
    title: "OpenAI State",
    render: ({ active, key }) => (
      <AddonPanel active={active} key={key}>
        <Panel active={active} />
      </AddonPanel>
    ),
  });
});
