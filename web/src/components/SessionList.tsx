import { SessionSummary } from "../api";

interface Props {
  sessions: SessionSummary[];
  activeId: string | null;
  onSelect: (id: string) => void;
  onNew: () => void;
  creating: boolean;
}

export function SessionList({ sessions, activeId, onSelect, onNew, creating }: Props) {
  return (
    <aside
      style={{
        width: 220,
        flexShrink: 0,
        background: "#0d1117",
        borderRight: "1px solid #21262d",
        display: "flex",
        flexDirection: "column",
        overflow: "hidden",
      }}
    >
      <div style={{ padding: "12px 12px 8px", borderBottom: "1px solid #21262d" }}>
        <button
          onClick={onNew}
          disabled={creating}
          style={{
            width: "100%",
            background: creating ? "#21262d" : "#238636",
            color: "#fff",
            border: "none",
            borderRadius: 6,
            padding: "6px 12px",
            fontSize: 13,
            cursor: creating ? "default" : "pointer",
            fontFamily: "inherit",
          }}
        >
          {creating ? "Creating…" : "+ New Chat"}
        </button>
      </div>
      <ul style={{ flex: 1, overflowY: "auto", listStyle: "none", margin: 0, padding: 0 }}>
        {sessions.length === 0 && (
          <li style={{ padding: "12px", color: "#8b949e", fontSize: 13 }}>No sessions yet</li>
        )}
        {sessions.map((s) => (
          <li key={s.id}>
            <button
              onClick={() => onSelect(s.id)}
              style={{
                width: "100%",
                textAlign: "left",
                background: s.id === activeId ? "#161b22" : "transparent",
                border: "none",
                borderLeft: s.id === activeId ? "2px solid #1f6feb" : "2px solid transparent",
                color: s.id === activeId ? "#e6edf3" : "#8b949e",
                padding: "8px 12px",
                fontSize: 13,
                cursor: "pointer",
                fontFamily: "inherit",
                lineHeight: 1.4,
              }}
            >
              <div style={{ fontWeight: 600, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                {s.id}
              </div>
              <div style={{ fontSize: 11, color: "#6e7681", marginTop: 2 }}>
                {s.model.replace(/^ollama:/, "")} · {s.message_count} msg{s.message_count !== 1 ? "s" : ""}
              </div>
            </button>
          </li>
        ))}
      </ul>
    </aside>
  );
}
