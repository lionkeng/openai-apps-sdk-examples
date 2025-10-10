import React, { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useChannel } from "@storybook/manager-api";
import { IconButton } from "@storybook/components";
import { WandIcon } from "@storybook/icons";
import type { DisplayMode, DeviceType, Theme } from "../../src/types";
import type { HostSnapshot, HostMethodName } from "../../src/testing/openai-storybook";
import {
  OPENAI_EVENT_CALL_METHOD,
  OPENAI_EVENT_CLEAR_HISTORY,
  OPENAI_EVENT_REQUEST_SNAPSHOT,
  OPENAI_EVENT_RESET,
  OPENAI_EVENT_SNAPSHOT,
  OPENAI_EVENT_UPDATE_GLOBALS,
  type GlobalsPatch,
} from "../../src/testing/openai-addon-channel";

type EditableJsonField = "toolInput" | "toolOutput";

const themeOptions: Theme[] = ["light", "dark"];
const displayModeOptions: DisplayMode[] = ["pip", "inline", "fullscreen"];
const deviceTypeOptions: DeviceType[] = ["desktop", "mobile", "tablet"];

type JsonEditorState =
  | {
      field: EditableJsonField;
      text: string;
      error: string | null;
    }
  | null;

type ToolbarPopoverProps = {
  snapshot: HostSnapshot | null;
  onUpdateGlobals: (patch: GlobalsPatch) => void;
  onCallMethod: (name: HostMethodName, args: unknown[]) => void;
  onReset: () => void;
  onClearHistory: () => void;
  onEditJson: (field: EditableJsonField) => void;
  onRequestSnapshot: () => void;
};

const SectionLabel = ({ children }: { children: string }) => (
  <p
    style={{
      fontSize: 11,
      fontWeight: 600,
      letterSpacing: 0.3,
      textTransform: "uppercase",
      margin: "0 0 4px",
    }}
  >
    {children}
  </p>
);

const ControlSelect = <T extends string>({
  label,
  value,
  options,
  onChange,
  disabled,
}: {
  label: string;
  value: T | "";
  options: readonly T[];
  onChange: (next: T) => void;
  disabled?: boolean;
}) => (
  <label style={{ display: "block", fontSize: 12, marginBottom: 8 }}>
    <span style={{ display: "block", marginBottom: 2, fontWeight: 500 }}>
      {label}
    </span>
    <select
      disabled={disabled}
      value={value}
      onChange={(event) => onChange(event.target.value as T)}
      style={{
        width: "100%",
        fontSize: 12,
        padding: "4px 6px",
        borderRadius: 4,
        border: "1px solid rgba(0,0,0,0.1)",
      }}
    >
      {options.map((option) => (
        <option key={option} value={option}>
          {option}
        </option>
      ))}
    </select>
  </label>
);

const buttonRowStyle: React.CSSProperties = {
  display: "flex",
  gap: 6,
  flexWrap: "wrap",
  marginBottom: 8,
};

const buttonStyle: React.CSSProperties = {
  fontSize: 12,
  borderRadius: 4,
  border: "1px solid rgba(0,0,0,0.15)",
  background: "var(--openai-toolbar-button-bg, #f5f5f5)",
  padding: "4px 8px",
  cursor: "pointer",
};

const primaryButtonStyle: React.CSSProperties = {
  ...buttonStyle,
  background: "var(--openai-toolbar-button-primary-bg, #2563eb)",
  color: "#fff",
  borderColor: "rgba(37, 99, 235, 0.6)",
};

const secondaryButtonStyle: React.CSSProperties = {
  ...buttonStyle,
  background: "var(--openai-toolbar-button-secondary-bg, #fff)",
};

const dangerButtonStyle: React.CSSProperties = {
  ...buttonStyle,
  background: "#d1292f",
  color: "#fff",
  borderColor: "#b32228",
};

const clamp = (value: number, min: number, max: number) =>
  Math.min(Math.max(value, min), max);

const ToolbarPopover = ({
  snapshot,
  onUpdateGlobals,
  onCallMethod,
  onReset,
  onClearHistory,
  onEditJson,
  onRequestSnapshot,
}: ToolbarPopoverProps) => {
  const globals = snapshot?.globals;

  return (
    <div
      style={{
        padding: 12,
        minWidth: 260,
        maxWidth: 320,
        fontSize: 12,
      }}
    >
      <SectionLabel>Globals</SectionLabel>
      <ControlSelect
        label="Theme"
        disabled={!globals}
        value={globals?.theme ?? ""}
        options={themeOptions}
        onChange={(theme) => onUpdateGlobals({ theme })}
      />
      <ControlSelect
        label="Display Mode"
        disabled={!globals}
        value={globals?.displayMode ?? ""}
        options={displayModeOptions}
        onChange={(displayMode) =>
          onUpdateGlobals({ displayMode })
        }
      />
      <ControlSelect
        label="Device Type"
        disabled={!globals}
        value={globals?.userAgent.device.type ?? ""}
        options={deviceTypeOptions}
        onChange={(deviceType) => {
          if (!globals) return;
          const capabilities =
            deviceType === "desktop"
              ? { hover: true, touch: false }
              : { hover: false, touch: true };
          onUpdateGlobals({
            userAgent: {
              ...globals.userAgent,
              device: { type: deviceType },
              capabilities,
            },
          });
        }}
      />

      <SectionLabel>JSON</SectionLabel>
      <div style={buttonRowStyle}>
        <button
          type="button"
          style={secondaryButtonStyle}
          disabled={!globals}
          onClick={() => globals && onEditJson("toolInput")}
        >
          Edit toolInput
        </button>
        <button
          type="button"
          style={primaryButtonStyle}
          disabled={!globals}
          onClick={() => globals && onEditJson("toolOutput")}
        >
          Edit toolOutput
        </button>
      </div>

      <hr
        style={{
          margin: "8px 0",
          border: "none",
          borderTop: "1px solid rgba(0,0,0,0.06)",
        }}
      />

      <SectionLabel>Quick Actions</SectionLabel>
      <div style={buttonRowStyle}>
        <button
          type="button"
          style={buttonStyle}
          onClick={() => onCallMethod("setWidgetState", [{}])}
        >
          Clear widget state
        </button>
      </div>

      <div style={{ ...buttonRowStyle, marginBottom: 0 }}>
        <button type="button" style={buttonStyle} onClick={onRequestSnapshot}>
          Refresh snapshot
        </button>
        <button type="button" style={secondaryButtonStyle} onClick={onClearHistory}>
          Clear history
        </button>
        <button type="button" style={dangerButtonStyle} onClick={onReset}>
          Reset host
        </button>
      </div>
    </div>
  );
};

export const Toolbar = () => {
  const [snapshot, setSnapshot] = useState<HostSnapshot | null>(null);
  const [popoverOpen, setPopoverOpen] = useState(false);
  const [position, setPosition] = useState(() => {
    if (typeof window === "undefined") {
      return { x: 24, y: 96 };
    }
    const widthEstimate = 300;
    return {
      x: Math.max(window.innerWidth - widthEstimate - 24, 24),
      y: 96,
    };
  });
  const [dragOffset, setDragOffset] = useState<{ x: number; y: number } | null>(
    null
  );
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [editorState, setEditorState] = useState<JsonEditorState>(null);

  const emit = useChannel(
    {
      [OPENAI_EVENT_SNAPSHOT]: (incoming: HostSnapshot) => {
        setSnapshot(incoming);
      },
    },
    []
  );

  useEffect(() => {
    emit(OPENAI_EVENT_REQUEST_SNAPSHOT);
  }, [emit]);

  useEffect(() => {
    if (!popoverOpen) return;
    emit(OPENAI_EVENT_REQUEST_SNAPSHOT);
  }, [popoverOpen, emit]);

  useEffect(() => {
    if (!dragOffset || typeof window === "undefined") return;

    const offset = dragOffset;
    const previousUserSelect = document.body.style.userSelect;
    document.body.style.userSelect = "none";

    const handleMouseMove = (event: MouseEvent) => {
      setPosition((prevPosition) => {
        const width = containerRef.current?.offsetWidth ?? 320;
        const height = containerRef.current?.offsetHeight ?? 360;
        const maxX = Math.max(window.innerWidth - width - 16, 16);
        const maxY = Math.max(window.innerHeight - height - 16, 16);
        return {
          x: clamp(event.clientX - offset.x, 16, maxX),
          y: clamp(event.clientY - offset.y, 16, maxY),
        };
      });
    };

    const handleMouseUp = () => {
      setDragOffset(null);
    };

    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);

    return () => {
      document.body.style.userSelect = previousUserSelect;
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [dragOffset]);

  useEffect(() => {
    if (!popoverOpen || typeof window === "undefined") return;

    const clampToViewport = () => {
      setPosition((prevPosition) => {
        const width = containerRef.current?.offsetWidth ?? 320;
        const height = containerRef.current?.offsetHeight ?? 360;
        const maxX = Math.max(window.innerWidth - width - 16, 16);
        const maxY = Math.max(window.innerHeight - height - 16, 16);
        return {
          x: clamp(prevPosition.x, 16, maxX),
          y: clamp(prevPosition.y, 16, maxY),
        };
      });
    };

    clampToViewport();
    window.addEventListener("resize", clampToViewport);

    return () => {
      window.removeEventListener("resize", clampToViewport);
    };
  }, [popoverOpen]);

  useEffect(() => {
    if (!popoverOpen) return;

    const handlePointerDown = (event: MouseEvent) => {
      if (!containerRef.current) return;
      if (containerRef.current.contains(event.target as Node)) return;
      setPopoverOpen(false);
    };

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setPopoverOpen(false);
      }
    };

    window.addEventListener("mousedown", handlePointerDown);
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      window.removeEventListener("mousedown", handlePointerDown);
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [popoverOpen]);

  const handleUpdateGlobals = (patch: GlobalsPatch) => {
    emit(OPENAI_EVENT_UPDATE_GLOBALS, { patch });
  };

  const handleCallMethod = (name: HostMethodName, args: unknown[]) => {
    emit(OPENAI_EVENT_CALL_METHOD, { name, args });
  };

  const handleReset = () => {
    emit(OPENAI_EVENT_RESET);
  };

  const handleClearHistory = () => {
    emit(OPENAI_EVENT_CLEAR_HISTORY);
  };

  const handleEditJson = (field: EditableJsonField) => {
    if (!snapshot) return;
    const current =
      field === "toolOutput"
        ? snapshot.globals.toolOutput
        : snapshot.globals.toolInput;
    setEditorState({
      field,
      text: JSON.stringify(current ?? (field === "toolInput" ? {} : null), null, 2),
      error: null,
    });
    setPopoverOpen(false);
  };

  const closeEditor = () => setEditorState(null);

  const saveEditor = () => {
    if (!editorState) return;
    try {
      const parsed = editorState.text.trim()
        ? JSON.parse(editorState.text)
        : editorState.field === "toolInput"
          ? {}
          : null;
      if (editorState.field === "toolOutput") {
        handleUpdateGlobals({ toolOutput: parsed });
      } else {
        handleUpdateGlobals({ toolInput: parsed });
      }
      setEditorState(null);
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Invalid JSON payload.";
      setEditorState((prev) =>
        prev ? { ...prev, error: message } : prev
      );
    }
  };

  const handleTogglePopover = () => {
    setPopoverOpen((prev) => !prev);
  };

  const handleClosePopover = () => {
    setPopoverOpen(false);
  };

  const handleDragStart = (event: React.MouseEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;
    event.preventDefault();
    setDragOffset({
      x: event.clientX - position.x,
      y: event.clientY - position.y,
    });
  };

  const portalTarget =
    typeof document === "undefined" ? null : document.body;

  const popover =
    popoverOpen && portalTarget
      ? createPortal(
          <div
            ref={containerRef}
            role="dialog"
            aria-label="OpenAI host controls"
            style={{
              position: "fixed",
              top: `${position.y}px`,
              left: `${position.x}px`,
              zIndex: 9998,
              background: "var(--openai-toolbar-surface, #fff)",
              color: "var(--openai-toolbar-foreground, inherit)",
              borderRadius: 8,
              minWidth: 260,
              maxWidth: 320,
              boxShadow:
                "0 20px 30px rgba(15,23,42,0.18), 0 8px 12px rgba(15,23,42,0.12)",
              border: "1px solid rgba(15,23,42,0.12)",
            }}
          >
            <div
              onMouseDown={handleDragStart}
              style={{
                cursor: dragOffset ? "grabbing" : "grab",
                padding: "8px 12px",
                fontSize: 12,
                fontWeight: 600,
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                background: "var(--openai-toolbar-header-bg, #0f172a)",
                color: "var(--openai-toolbar-header-fg, #fff)",
                borderTopLeftRadius: 8,
                borderTopRightRadius: 8,
                borderBottom: "1px solid rgba(255,255,255,0.08)",
                textTransform: "uppercase",
                letterSpacing: 0.4,
              }}
            >
              <span>OpenAI Host Controls</span>
              <button
                type="button"
                onClick={handleClosePopover}
                style={{
                  appearance: "none",
                  background: "transparent",
                  border: "none",
                  color: "inherit",
                  cursor: "pointer",
                  fontSize: 16,
                  lineHeight: 1,
                  padding: 4,
                  margin: 0,
                }}
                aria-label="Close OpenAI host controls"
              >
                ×
              </button>
            </div>
            <ToolbarPopover
              snapshot={snapshot}
              onUpdateGlobals={handleUpdateGlobals}
              onCallMethod={handleCallMethod}
              onReset={handleReset}
              onClearHistory={handleClearHistory}
              onEditJson={handleEditJson}
              onRequestSnapshot={() => emit(OPENAI_EVENT_REQUEST_SNAPSHOT)}
            />
          </div>,
          portalTarget
        )
      : null;

  return (
    <>
      <IconButton
        key="openai-toolbar"
        title="OpenAI host controls"
        active={popoverOpen}
        onClick={handleTogglePopover}
        aria-pressed={popoverOpen}
      >
        <WandIcon size={14} color="#F46C21" />
      </IconButton>

      {popover}

      {editorState && (
        <div
          style={{
            position: "fixed",
            inset: 0,
            background: "rgba(0,0,0,0.48)",
            zIndex: 9999,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
          }}
        >
          <div
            role="dialog"
            aria-modal="true"
            style={{
              width: "min(520px, 90vw)",
              maxHeight: "80vh",
              background: "var(--color-background, #fff)",
              borderRadius: 8,
              boxShadow:
                "0 18px 24px rgba(0,0,0,0.12), 0 8px 12px rgba(0,0,0,0.08)",
              padding: 16,
              display: "flex",
              flexDirection: "column",
              gap: 12,
            }}
          >
            <div>
              <h3 style={{ margin: "0 0 8px" }}>
                Edit {editorState.field}
              </h3>
              <p style={{ margin: 0, fontSize: 12, color: "#666" }}>
                Provide valid JSON. Leave empty to reset to{" "}
                {editorState.field === "toolInput" ? "{}" : "null"}.
              </p>
            </div>
            <textarea
              value={editorState.text}
              onChange={(event) =>
                setEditorState((prev) =>
                  prev ? { ...prev, text: event.target.value, error: null } : prev
                )
              }
              style={{
                flex: 1,
                minHeight: 200,
                fontFamily: "var(--font-family-monospace, monospace)",
                fontSize: 12,
                borderRadius: 4,
                border: "1px solid rgba(0,0,0,0.15)",
                padding: 8,
                resize: "vertical",
              }}
            />
            {editorState.error && (
              <p style={{ color: "#d1292f", fontSize: 12, margin: 0 }}>
                {editorState.error}
              </p>
            )}
            <div
              style={{
                display: "flex",
                gap: 8,
                justifyContent: "flex-end",
              }}
            >
              <button
                type="button"
                style={secondaryButtonStyle}
                onClick={closeEditor}
              >
                Cancel
              </button>
              <button
                type="button"
                style={primaryButtonStyle}
                onClick={saveEditor}
              >
                Save
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
};

export default Toolbar;
