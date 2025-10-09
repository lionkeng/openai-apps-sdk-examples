import React, { useEffect, useState } from "react";
import { useChannel } from "@storybook/manager-api";
import { IconButton, WithTooltip } from "@storybook/components";
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
  const [tooltipVisible, setTooltipVisible] = useState(false);
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
    setTooltipVisible(false);
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

  return (
    <>
      <WithTooltip
        placement="top"
        trigger="click"
        closeOnOutsideClick
        tooltip={
          <ToolbarPopover
            snapshot={snapshot}
            onUpdateGlobals={handleUpdateGlobals}
            onCallMethod={handleCallMethod}
            onReset={handleReset}
            onClearHistory={handleClearHistory}
            onEditJson={handleEditJson}
            onRequestSnapshot={() => emit(OPENAI_EVENT_REQUEST_SNAPSHOT)}
          />
        }
        onVisibilityChange={setTooltipVisible}
      >
        <IconButton
          key="openai-toolbar"
          title="OpenAI host controls"
          active={tooltipVisible}
        >
          <WandIcon size={14} color="#F46C21" />
        </IconButton>
      </WithTooltip>

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
