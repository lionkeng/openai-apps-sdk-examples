import type { HostSnapshot, HostMethodName } from "./openai-storybook";
import type { OpenAiGlobals } from "../types";

export const OPENAI_EVENT_UPDATE_GLOBALS = "openai:updateGlobals";
export const OPENAI_EVENT_CALL_METHOD = "openai:callMethod";
export const OPENAI_EVENT_RESET = "openai:reset";
export const OPENAI_EVENT_SNAPSHOT = "openai:snapshot";
export const OPENAI_EVENT_REQUEST_SNAPSHOT = "openai:requestSnapshot";
export const OPENAI_EVENT_CLEAR_HISTORY = "openai:clearHistory";

export type GlobalsPatch = Partial<Omit<OpenAiGlobals, "setWidgetState">>;

export type UpdateGlobalsPayload = {
  patch: GlobalsPatch;
};

export type CallMethodPayload = {
  name: HostMethodName;
  args: unknown[];
};

export type SnapshotPayload = HostSnapshot;
