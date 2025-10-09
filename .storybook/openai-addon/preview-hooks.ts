import { addons } from "@storybook/preview-api";
import {
  installOpenAiMock,
  updateOpenAiGlobals,
  invokeHostMethod,
  resetHostState,
  subscribe,
  getHostSnapshot,
  clearCallHistory,
} from "../../src/testing/openai-storybook";
import {
  OPENAI_EVENT_CALL_METHOD,
  OPENAI_EVENT_CLEAR_HISTORY,
  OPENAI_EVENT_REQUEST_SNAPSHOT,
  OPENAI_EVENT_RESET,
  OPENAI_EVENT_SNAPSHOT,
  OPENAI_EVENT_UPDATE_GLOBALS,
  type CallMethodPayload,
  type UpdateGlobalsPayload,
} from "../../src/testing/openai-addon-channel";

declare global {
  interface Window {
    __openAiToolbarInitialized__?: boolean;
  }
}

if (typeof window !== "undefined") {
  const w = window as Window;
  if (!w.__openAiToolbarInitialized__) {
    w.__openAiToolbarInitialized__ = true;

    installOpenAiMock();
    const channel = addons.getChannel();

    const emitSnapshot = () => {
      channel.emit(OPENAI_EVENT_SNAPSHOT, getHostSnapshot());
    };

    channel.on(
      OPENAI_EVENT_UPDATE_GLOBALS,
      (payload: UpdateGlobalsPayload) => {
        updateOpenAiGlobals(payload.patch);
      }
    );

    channel.on(OPENAI_EVENT_CALL_METHOD, async (payload: CallMethodPayload) => {
      try {
        await invokeHostMethod(
          payload.name as CallMethodPayload["name"],
          ...(payload.args ?? [])
        );
      } catch (error) {
        console.error("[openai-toolbar] host method failed", error);
      }
    });

    channel.on(OPENAI_EVENT_RESET, () => {
      resetHostState();
    });

    channel.on(OPENAI_EVENT_CLEAR_HISTORY, () => {
      clearCallHistory();
    });

    channel.on(OPENAI_EVENT_REQUEST_SNAPSHOT, () => {
      emitSnapshot();
    });

    subscribe(() => {
      emitSnapshot();
    });

    emitSnapshot();
  }
}
