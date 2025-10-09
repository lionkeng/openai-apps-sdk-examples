import React, { useEffect, useState, type CSSProperties } from "react";
import { useChannel } from "@storybook/manager-api";
import type { HostSnapshot } from "../../src/testing/openai-storybook";
import {
  OPENAI_EVENT_CLEAR_HISTORY,
  OPENAI_EVENT_REQUEST_SNAPSHOT,
  OPENAI_EVENT_RESET,
  OPENAI_EVENT_SNAPSHOT,
} from "../../src/testing/openai-addon-channel";

const sectionStyle: CSSProperties = {
  marginBottom: 16,
};

const headingStyle: CSSProperties = {
  margin: "0 0 8px",
  fontSize: 13,
  fontWeight: 600,
};

const preStyle: CSSProperties = {
  fontSize: 12,
  lineHeight: 1.4,
  background: "var(--panel-json-bg, #f5f5f5)",
  borderRadius: 6,
  border: "1px solid rgba(0,0,0,0.1)",
  padding: 12,
  overflowX: "auto",
};

const smallPreStyle: CSSProperties = {
  ...preStyle,
  background: "transparent",
  border: "none",
  padding: 0,
};

const buttonRowStyle: CSSProperties = {
  display: "flex",
  gap: 8,
  flexWrap: "wrap",
  marginBottom: 16,
};

const buttonStyle: CSSProperties = {
  fontSize: 12,
  borderRadius: 4,
  border: "1px solid rgba(0,0,0,0.15)",
  background: "var(--panel-button-bg, #fff)",
  padding: "4px 10px",
  cursor: "pointer",
};

const dangerButtonStyle: CSSProperties = {
  ...buttonStyle,
  background: "#d1292f",
  borderColor: "#b32228",
  color: "#fff",
};

const tableStyle: CSSProperties = {
  width: "100%",
  borderCollapse: "collapse",
  fontSize: 12,
};

const thStyle: CSSProperties = {
  textAlign: "left",
  borderBottom: "1px solid rgba(0,0,0,0.1)",
  padding: "6px 8px",
};

const tdStyle: CSSProperties = {
  borderBottom: "1px solid rgba(0,0,0,0.05)",
  verticalAlign: "top",
  padding: "6px 8px",
};

const formatJson = (value: unknown) =>
  value === undefined ? "undefined" : JSON.stringify(value, null, 2);

const formatTimestamp = (timestamp: number) =>
  new Date(timestamp).toLocaleTimeString();

type PanelProps = {
  active: boolean;
};

export const Panel = ({ active }: PanelProps) => {
  const [snapshot, setSnapshot] = useState<HostSnapshot | null>(null);

  const emit = useChannel(
    {
      [OPENAI_EVENT_SNAPSHOT]: (incoming: HostSnapshot) => {
        setSnapshot(incoming);
      },
    },
    []
  );

  useEffect(() => {
    if (active) {
      emit(OPENAI_EVENT_REQUEST_SNAPSHOT);
    }
  }, [active, emit]);

  const globals = snapshot?.globals;
  const callHistory = snapshot?.callHistory ?? [];

  return (
    <div style={{ padding: 16, overflow: "auto", height: "100%" }}>
      <div style={buttonRowStyle}>
        <button
          type="button"
          style={buttonStyle}
          onClick={() => emit(OPENAI_EVENT_REQUEST_SNAPSHOT)}
        >
          Refresh snapshot
        </button>
        <button
          type="button"
          style={buttonStyle}
          onClick={() => emit(OPENAI_EVENT_CLEAR_HISTORY)}
        >
          Clear history
        </button>
        <button
          type="button"
          style={dangerButtonStyle}
          onClick={() => emit(OPENAI_EVENT_RESET)}
        >
          Reset host
        </button>
      </div>

      <section style={sectionStyle}>
        <h3 style={headingStyle}>Globals</h3>
        {globals ? (
          <pre style={preStyle}>{formatJson(globals)}</pre>
        ) : (
          <p style={{ fontSize: 12, color: "#666" }}>
            Awaiting snapshot from preview iframe…
          </p>
        )}
      </section>

      <section style={sectionStyle}>
        <h3 style={headingStyle}>Call History</h3>
        {callHistory.length === 0 ? (
          <p style={{ fontSize: 12, color: "#666" }}>No host calls recorded.</p>
        ) : (
          <div style={{ maxHeight: 240, overflowY: "auto" }}>
            <table style={tableStyle}>
              <thead>
                <tr>
                  <th style={thStyle}>Time</th>
                  <th style={thStyle}>Method</th>
                  <th style={thStyle}>Args</th>
                  <th style={thStyle}>Result</th>
                  <th style={thStyle}>Error</th>
                </tr>
              </thead>
              <tbody>
                {callHistory.map((entry) => (
                  <tr key={entry.id}>
                    <td style={tdStyle}>{formatTimestamp(entry.timestamp)}</td>
                    <td style={tdStyle}>{entry.method}</td>
                    <td style={tdStyle}>
                      <pre style={{ ...smallPreStyle, margin: 0 }}>
                        {formatJson(entry.args)}
                      </pre>
                    </td>
                    <td style={tdStyle}>
                      {entry.result !== undefined ? (
                        <pre style={{ ...smallPreStyle, margin: 0 }}>
                          {formatJson(entry.result)}
                        </pre>
                      ) : (
                        <span style={{ color: "#888" }}>—</span>
                      )}
                    </td>
                    <td style={tdStyle}>
                      {entry.error ? (
                        <pre
                          style={{
                            ...smallPreStyle,
                            margin: 0,
                            color: "#d1292f",
                          }}
                        >
                          {formatJson(entry.error)}
                        </pre>
                      ) : (
                        <span style={{ color: "#888" }}>—</span>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>
    </div>
  );
};

export default Panel;
