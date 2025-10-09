import { useEffect, type ReactNode } from "react";
import {
  SET_GLOBALS_EVENT_TYPE,
  SetGlobalsEvent,
  type CallToolResponse,
  type OpenAiGlobals,
  type RequestDisplayMode,
} from "../types";

type OpenAiApi = {
  callTool: (
    name: string,
    args: Record<string, unknown>
  ) => Promise<CallToolResponse>;
  sendFollowUpMessage: (args: { prompt: string }) => Promise<void>;
  openExternal: (payload: { href: string }) => void;
  requestDisplayMode: RequestDisplayMode;
};

export type MockOpenAi = OpenAiGlobals &
  OpenAiApi & {
    /**
     * Attach custom listeners for assertions without replacing the core mock.
     */
    __handlers: {
      onSetWidgetState?: (
        state: OpenAiGlobals["widgetState"]
      ) => void | Promise<void>;
      onRequestDisplayMode?: (
        mode: Parameters<RequestDisplayMode>[0]["mode"],
        granted: Parameters<RequestDisplayMode>[0]["mode"]
      ) => void | Promise<void>;
    };
  };

let currentMock: MockOpenAi | null = null;

const dispatchGlobals = (globals: Partial<OpenAiGlobals>) => {
  if (typeof window === "undefined") return;
  const event = new SetGlobalsEvent(SET_GLOBALS_EVENT_TYPE, {
    detail: { globals },
  });
  window.dispatchEvent(event);
};

const createDefaultMock = (): MockOpenAi => {
  const mock: MockOpenAi = {
    theme: "light",
    userAgent: {
      device: { type: "desktop" },
      capabilities: { hover: true, touch: false },
    },
    locale: "en-US",
    maxHeight: 600,
    displayMode: "inline",
    safeArea: {
      insets: { top: 0, right: 0, bottom: 0, left: 0 },
    },
    toolInput: {},
    toolOutput: null,
    toolResponseMetadata: null,
    widgetState: null,
    async setWidgetState(state) {
      mock.widgetState = state;
      dispatchGlobals({ widgetState: state });
      await mock.__handlers.onSetWidgetState?.(state);
    },
    async callTool(name, args) {
      console.info("[mock openai] callTool", name, args);
      return { result: "" };
    },
    async sendFollowUpMessage({ prompt }) {
      console.info("[mock openai] sendFollowUpMessage", prompt);
    },
    openExternal({ href }) {
      if (typeof window !== "undefined") {
        window.open?.(href, "_blank", "noopener,noreferrer");
      }
    },
    async requestDisplayMode({ mode }) {
      mock.displayMode = mode;
      dispatchGlobals({ displayMode: mode });
      await mock.__handlers.onRequestDisplayMode?.(mode, mode);
      return { mode };
    },
    __handlers: {},
  };

  return mock;
};

const ensureMock = () => {
  if (typeof window === "undefined") {
    throw new Error("window is undefined; OpenAI mock requires a browser-like environment.");
  }
  if (currentMock) {
    return currentMock;
  }

  const mock = createDefaultMock();
  Object.defineProperty(window, "openai", {
    configurable: true,
    enumerable: false,
    writable: true,
    value: mock,
  });
  currentMock = mock;
  return mock;
};

export const installOpenAiMock = (
  overrides?: Partial<Omit<MockOpenAi, "__handlers">> & {
    onSetWidgetState?: MockOpenAi["__handlers"]["onSetWidgetState"];
    onRequestDisplayMode?: MockOpenAi["__handlers"]["onRequestDisplayMode"];
  }
): MockOpenAi => {
  const mock = ensureMock();

  if (overrides) {
    const { onSetWidgetState, onRequestDisplayMode, ...rest } = overrides;
    Object.assign(mock, rest);
    mock.__handlers.onSetWidgetState = onSetWidgetState;
    mock.__handlers.onRequestDisplayMode = onRequestDisplayMode;
  }

  return mock;
};

export const updateOpenAiGlobals = (partial: Partial<OpenAiGlobals>) => {
  if (!partial || Object.keys(partial).length === 0) {
    return;
  }
  const mock = ensureMock();
  Object.assign(mock, partial);
  dispatchGlobals(partial);
};

export const simulateToolResponse = (
  partial: Partial<Pick<OpenAiGlobals, "toolOutput" | "toolResponseMetadata">>
) => {
  updateOpenAiGlobals(partial);
};

export const uninstallOpenAiMock = () => {
  if (typeof window === "undefined") return;
  if (!currentMock) return;
  delete (window as typeof window & { openai?: MockOpenAi }).openai;
  currentMock = null;
};

export const getOpenAiMock = (): MockOpenAi | null => {
  return currentMock;
};

type MockOpenAiProviderProps = {
  globals?: Partial<OpenAiGlobals>;
  children: ReactNode;
  onSetWidgetState?: MockOpenAi["__handlers"]["onSetWidgetState"];
  onRequestDisplayMode?: MockOpenAi["__handlers"]["onRequestDisplayMode"];
};

export const MockOpenAiProvider = ({
  globals,
  children,
  onSetWidgetState,
  onRequestDisplayMode,
}: MockOpenAiProviderProps) => {
  useEffect(() => {
    installOpenAiMock({
      onSetWidgetState,
      onRequestDisplayMode,
    });
    return () => {
      uninstallOpenAiMock();
    };
  }, [onRequestDisplayMode, onSetWidgetState]);

  useEffect(() => {
    if (globals) {
      updateOpenAiGlobals(globals);
    }
  }, [globals]);

  return <>{children}</>;
};
