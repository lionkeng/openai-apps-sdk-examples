import { useEffect, useSyncExternalStore, type ReactNode } from "react";
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

type OpenAiGlobalsWithoutSetter = Omit<OpenAiGlobals, "setWidgetState">;

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
        requested: Parameters<RequestDisplayMode>[0]["mode"],
        granted: Parameters<RequestDisplayMode>[0]["mode"]
      ) => void | Promise<void>;
    };
  };

export type HostMethodName =
  | "setWidgetState"
  | "callTool"
  | "sendFollowUpMessage"
  | "openExternal"
  | "requestDisplayMode";

export type CallLogEntry = {
  id: number;
  method: HostMethodName;
  args: unknown;
  result?: unknown;
  error?: { message: string; stack?: string } | unknown;
  timestamp: number;
};

export type HostSnapshot = {
  globals: OpenAiGlobals;
  callHistory: CallLogEntry[];
};

type SnapshotListener = (snapshot: HostSnapshot) => void;

const CALL_HISTORY_LIMIT = 50;

let currentMock: MockOpenAi | null = null;
let handlerOverrides: Partial<{
  callTool: OpenAiApi["callTool"];
  sendFollowUpMessage: OpenAiApi["sendFollowUpMessage"];
  openExternal: OpenAiApi["openExternal"];
  requestDisplayMode: OpenAiApi["requestDisplayMode"];
  setWidgetState: OpenAiGlobals["setWidgetState"];
}> = {};

const callHistory: CallLogEntry[] = [];
const listeners = new Set<SnapshotListener>();
const storeListeners = new Set<() => void>();
let callCounter = 0;

const clone = <T,>(value: T): T => {
  if (typeof structuredClone === "function") {
    return structuredClone(value);
  }
  return JSON.parse(JSON.stringify(value));
};

const dispatchGlobals = (globals: Partial<OpenAiGlobals>) => {
  if (typeof window === "undefined") return;
  const event = new SetGlobalsEvent(SET_GLOBALS_EVENT_TYPE, {
    detail: { globals },
  });
  window.dispatchEvent(event);
};

const createDefaultGlobals = (): OpenAiGlobalsWithoutSetter => ({
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
});

const recordCallStart = (
  method: HostMethodName,
  args: unknown
): CallLogEntry => {
  const entry: CallLogEntry = {
    id: ++callCounter,
    method,
    args,
    timestamp: Date.now(),
  };
  callHistory.unshift(entry);
  if (callHistory.length > CALL_HISTORY_LIMIT) {
    callHistory.length = CALL_HISTORY_LIMIT;
  }
  notify();
  return entry;
};

const resolveCall = (entry: CallLogEntry, result: unknown) => {
  entry.result = result;
  notify();
};

const rejectCall = (entry: CallLogEntry, error: unknown) => {
  if (error instanceof Error) {
    entry.error = { message: error.message, stack: error.stack };
  } else {
    entry.error = error;
  }
  notify();
};

const buildSnapshot = (mock: MockOpenAi): HostSnapshot => {
  const globals: OpenAiGlobals = {
    theme: mock.theme,
    userAgent: clone(mock.userAgent),
    locale: mock.locale,
    maxHeight: mock.maxHeight,
    displayMode: mock.displayMode,
    safeArea: clone(mock.safeArea),
    toolInput: clone(mock.toolInput),
    toolOutput: clone(mock.toolOutput),
    toolResponseMetadata: clone(mock.toolResponseMetadata),
    widgetState: clone(mock.widgetState),
    setWidgetState: mock.setWidgetState.bind(mock),
  };

  return {
    globals,
    callHistory: callHistory.map((entry) => ({ ...entry })),
  };
};

const notify = () => {
  if (!currentMock) return;
  const snapshot = buildSnapshot(currentMock);
  listeners.forEach((listener) => {
    try {
      listener(snapshot);
    } catch (error) {
      console.error("[mock openai] listener error", error);
    }
  });
  storeListeners.forEach((listener) => {
    try {
      listener();
    } catch (error) {
      console.error("[mock openai] store listener error", error);
    }
  });
};

const ensureMock = () => {
  if (typeof window === "undefined") {
    throw new Error(
      "window is undefined; OpenAI mock requires a browser-like environment."
    );
  }
  if (currentMock) {
    return currentMock;
  }

  const globals = createDefaultGlobals();

  const mock: MockOpenAi = {
    ...globals,
    async setWidgetState(state) {
      const entry = recordCallStart("setWidgetState", [state]);

      const impl =
        handlerOverrides.setWidgetState ??
        (async (nextState: OpenAiGlobals["widgetState"]) => {
          mock.widgetState = nextState;
          dispatchGlobals({ widgetState: nextState });
        });

      try {
        await impl(state);
        resolveCall(entry, { widgetState: state });
        await mock.__handlers.onSetWidgetState?.(state);
      } catch (error) {
        rejectCall(entry, error);
        throw error;
      }
    },
    async callTool(name, args) {
      const entry = recordCallStart("callTool", { name, args });
      const impl =
        handlerOverrides.callTool ??
        (async (): Promise<CallToolResponse> => ({ result: "" }));
      try {
        const result = await impl(name, args);
        resolveCall(entry, result);
        return result;
      } catch (error) {
        rejectCall(entry, error);
        throw error;
      }
    },
    async sendFollowUpMessage(payload) {
      const entry = recordCallStart("sendFollowUpMessage", payload);
      const impl =
        handlerOverrides.sendFollowUpMessage ??
        (async () => {
          console.info("[mock openai] sendFollowUpMessage", payload.prompt);
        });

      try {
        await impl(payload);
        resolveCall(entry, { ok: true });
      } catch (error) {
        rejectCall(entry, error);
        throw error;
      }
    },
    openExternal(payload) {
      const entry = recordCallStart("openExternal", payload);
      const impl =
        handlerOverrides.openExternal ??
        ((args: { href: string }) => {
          if (typeof window !== "undefined") {
            window.open?.(args.href, "_blank", "noopener,noreferrer");
          }
        });

      try {
        const result = impl(payload);
        resolveCall(entry, result ?? { ok: true });
      } catch (error) {
        rejectCall(entry, error);
        throw error;
      }
    },
    async requestDisplayMode({ mode }) {
      const entry = recordCallStart("requestDisplayMode", { mode });
      const impl =
        handlerOverrides.requestDisplayMode ??
        (async (args: { mode: OpenAiGlobals["displayMode"] }) => args);

      try {
        const { mode: grantedMode } = await impl({ mode });
        mock.displayMode = grantedMode;
        dispatchGlobals({ displayMode: grantedMode });
        resolveCall(entry, { mode: grantedMode });
        await mock.__handlers.onRequestDisplayMode?.(mode, grantedMode);
        return { mode: grantedMode };
      } catch (error) {
        rejectCall(entry, error);
        throw error;
      }
    },
    __handlers: {},
  };

  Object.defineProperty(window, "openai", {
    configurable: true,
    enumerable: false,
    writable: true,
    value: mock,
  });

  currentMock = mock;
  notify();
  return mock;
};

const applyGlobalOverrides = (
  mock: MockOpenAi,
  globals?: Partial<OpenAiGlobalsWithoutSetter>
) => {
  if (!globals) return;
  Object.assign(mock, globals);
  dispatchGlobals(globals);
  notify();
};

const applyHandlers = (
  overrides?: Partial<{
    callTool: OpenAiApi["callTool"];
    sendFollowUpMessage: OpenAiApi["sendFollowUpMessage"];
    openExternal: OpenAiApi["openExternal"];
    requestDisplayMode: OpenAiApi["requestDisplayMode"];
    setWidgetState: OpenAiGlobals["setWidgetState"];
  }>
) => {
  if (!overrides) return;
  handlerOverrides = {
    ...handlerOverrides,
    ...overrides,
  };
};

export const installOpenAiMock = (
  overrides?: Partial<Omit<MockOpenAi, "__handlers">> & {
    onSetWidgetState?: MockOpenAi["__handlers"]["onSetWidgetState"];
    onRequestDisplayMode?: MockOpenAi["__handlers"]["onRequestDisplayMode"];
  }
): MockOpenAi => {
  const mock = ensureMock();

  const {
    onSetWidgetState,
    onRequestDisplayMode,
    callTool,
    sendFollowUpMessage,
    openExternal,
    requestDisplayMode,
    setWidgetState,
    ...globals
  } = overrides ?? {};

  applyHandlers({
    callTool,
    sendFollowUpMessage,
    openExternal,
    requestDisplayMode,
    setWidgetState,
  });

  mock.__handlers.onSetWidgetState = onSetWidgetState;
  mock.__handlers.onRequestDisplayMode = onRequestDisplayMode;

  applyGlobalOverrides(mock, globals);

  return mock;
};

export const updateOpenAiGlobals = (partial: Partial<OpenAiGlobals>) => {
  if (!partial || Object.keys(partial).length === 0) {
    return;
  }
  const mock = ensureMock();
  Object.assign(mock, partial);
  dispatchGlobals(partial);
  notify();
};

export const simulateToolResponse = (
  partial: Partial<Pick<OpenAiGlobals, "toolOutput" | "toolResponseMetadata">>
) => {
  updateOpenAiGlobals(partial);
};

export const resetHostState = () => {
  if (!currentMock) return;
  handlerOverrides = {};
  callHistory.length = 0;
  callCounter = 0;

  const defaults = createDefaultGlobals();
  Object.assign(currentMock, defaults);
  dispatchGlobals(defaults);
  notify();
};

export const clearCallHistory = () => {
  if (!currentMock) return;
  callHistory.length = 0;
  callCounter = 0;
  notify();
};

export const uninstallOpenAiMock = () => {
  if (typeof window === "undefined") return;
  if (!currentMock) return;
  Reflect.deleteProperty(window as typeof window & { openai?: MockOpenAi }, "openai");
  handlerOverrides = {};
  callHistory.length = 0;
  callCounter = 0;
  currentMock = null;
  notify();
};

export const getOpenAiMock = (): MockOpenAi | null => {
  return currentMock;
};

export const getHostSnapshot = (): HostSnapshot => {
  const mock = ensureMock();
  return buildSnapshot(mock);
};

export const subscribe = (listener: SnapshotListener) => {
  listeners.add(listener);
  try {
    listener(getHostSnapshot());
  } catch (error) {
    console.error("[mock openai] listener error during subscribe", error);
  }
  return () => {
    listeners.delete(listener);
  };
};

const subscribeStore = (listener: () => void) => {
  storeListeners.add(listener);
  return () => {
    storeListeners.delete(listener);
  };
};

export const useOpenAiSnapshot = () =>
  useSyncExternalStore(subscribeStore, getHostSnapshot, getHostSnapshot);

export const invokeHostMethod = async <
  Name extends keyof Pick<
    MockOpenAi,
    "setWidgetState" | "callTool" | "sendFollowUpMessage" | "openExternal" | "requestDisplayMode"
  >
>(
  name: Name,
  ...args: Parameters<MockOpenAi[Name]>
): Promise<Awaited<ReturnType<MockOpenAi[Name]>>> => {
  const mock = ensureMock();
  const method =
    (mock[name] as unknown) as (
      ...fnArgs: Parameters<MockOpenAi[Name]>
    ) => ReturnType<MockOpenAi[Name]>;
  const result = method(...args);
  return await result;
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
