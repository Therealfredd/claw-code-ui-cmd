import { useEffect, useRef, useState } from "react";
import {
  ConversationMessage,
  ModelInfo,
  SessionSummary,
  createSession,
  fetchModels,
  fetchSession,
  fetchSessions,
  openEventStream,
  sendMessage,
} from "./api";
import { ChatWindow } from "./components/ChatWindow";
import { MessageInput } from "./components/MessageInput";
import { ModelSelector } from "./components/ModelSelector";
import { SessionList } from "./components/SessionList";

export default function App() {
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [selectedModel, setSelectedModel] = useState<string>("");
  const [workspaceDir, setWorkspaceDir] = useState<string>("");
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [messages, setMessages] = useState<ConversationMessage[]>([]);
  const [streaming, setStreaming] = useState(false);
  const [creating, setCreating] = useState(false);
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const eventSourceRef = useRef<EventSource | null>(null);

  // Load models on mount
  useEffect(() => {
    fetchModels()
      .then((ms) => {
        setModels(ms);
        if (ms.length > 0 && !selectedModel) {
          setSelectedModel(ms[0].id);
        }
      })
      .catch((e) => setError(String(e)));
  }, []);

  // Load sessions on mount
  useEffect(() => {
    refreshSessions();
  }, []);

  // Connect SSE when active session changes
  useEffect(() => {
    if (eventSourceRef.current) {
      eventSourceRef.current.close();
      eventSourceRef.current = null;
    }

    if (!activeSessionId) {
      setMessages([]);
      return;
    }

    // Immediately load existing messages
    fetchSession(activeSessionId)
      .then((details) => setMessages(details.session.messages))
      .catch((e) => setError(String(e)));

    const es = openEventStream(activeSessionId);
    eventSourceRef.current = es;

    es.addEventListener("snapshot", (e: MessageEvent) => {
      const data = JSON.parse(e.data);
      setMessages(data.session?.messages ?? []);
      setStreaming(false);
    });

    es.addEventListener("message", (e: MessageEvent) => {
      const data = JSON.parse(e.data);
      if (data.message) {
        const msg = data.message;
        setMessages((prev) => {
          // Skip exact duplicates (optimistic user messages already appended)
          const firstText = (msg.blocks?.[0] as { type: string; text?: string } | undefined)?.text ?? "";
          const exists = prev.some(
            (m) =>
              m.role === msg.role &&
              (m.blocks?.[0] as { type: string; text?: string } | undefined)?.text === firstText
          );
          return exists ? prev : [...prev, msg];
        });
        // Only stop the spinner once the assistant replies (not on the echoed user message)
        if (msg.role === "assistant") {
          setStreaming(false);
          setSending(false);
          refreshSessions();
        }
      }
    });

    es.onerror = () => {
      setStreaming(false);
      setSending(false);
    };

    return () => {
      es.close();
    };
  }, [activeSessionId]);

  async function refreshSessions() {
    try {
      const list = await fetchSessions();
      setSessions(list);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleNewSession() {
    if (!selectedModel) return;
    setCreating(true);
    setError(null);
    try {
      const { session_id } = await createSession(
        selectedModel,
        workspaceDir.trim() || undefined,
      );
      await refreshSessions();
      setActiveSessionId(session_id);
    } catch (e) {
      setError(String(e));
    } finally {
      setCreating(false);
    }
  }

  async function handleSend(text: string) {
    if (!activeSessionId || sending) return;
    setSending(true);
    setStreaming(true);
    setError(null);
    // Optimistically append user message
    setMessages((prev) => [...prev, { role: "user", blocks: [{ type: "text" as const, text }] }]);
    try {
      await sendMessage(activeSessionId, text);
    } catch (e) {
      setError(String(e));
      setStreaming(false);
      setSending(false);
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100dvh" }}>
      {/* Top bar */}
      <header
        style={{
          display: "flex",
          alignItems: "center",
          gap: 12,
          padding: "8px 16px",
          background: "#161b22",
          borderBottom: "1px solid #21262d",
          flexShrink: 0,
        }}
      >
        <span style={{ fontWeight: 700, fontSize: 16, letterSpacing: "-0.02em", color: "#e6edf3" }}>
          🦞 Claw
        </span>
        <div style={{ flex: 1 }} />
        <input
          type="text"
          value={workspaceDir}
          onChange={(e) => setWorkspaceDir(e.target.value)}
          placeholder="Workspace folder (optional)"
          title="Local directory the model can read and edit files in"
          style={{
            background: "#0d1117",
            border: "1px solid #30363d",
            borderRadius: 6,
            color: "#e6edf3",
            fontFamily: "inherit",
            fontSize: 12,
            padding: "4px 10px",
            width: 240,
            outline: "none",
          }}
        />
        <ModelSelector
          models={models}
          selected={selectedModel}
          onChange={setSelectedModel}
        />
      </header>

      {/* Main area */}
      <div style={{ display: "flex", flex: 1, overflow: "hidden" }}>
        <SessionList
          sessions={sessions}
          activeId={activeSessionId}
          onSelect={setActiveSessionId}
          onNew={handleNewSession}
          creating={creating}
        />

        <main style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
          {error && (
            <div
              style={{
                background: "#3d1c1c",
                color: "#f85149",
                padding: "8px 16px",
                fontSize: 13,
                borderBottom: "1px solid #5a1e1e",
                flexShrink: 0,
              }}
            >
              {error}{" "}
              <button
                onClick={() => setError(null)}
                style={{
                  background: "none",
                  border: "none",
                  color: "#f85149",
                  cursor: "pointer",
                  fontFamily: "inherit",
                  fontSize: 12,
                  textDecoration: "underline",
                  padding: 0,
                }}
              >
                dismiss
              </button>
            </div>
          )}

          {activeSessionId ? (
            <>
              <div
                style={{
                  padding: "6px 16px",
                  borderBottom: "1px solid #21262d",
                  fontSize: 12,
                  color: "#8b949e",
                  flexShrink: 0,
                }}
              >
                {activeSessionId} · {sessions.find((s) => s.id === activeSessionId)?.model ?? ""}
              </div>
              <ChatWindow messages={messages} streaming={streaming} />
              <MessageInput onSend={handleSend} disabled={sending} />
            </>
          ) : (
            <div
              style={{
                flex: 1,
                display: "flex",
                flexDirection: "column",
                alignItems: "center",
                justifyContent: "center",
                color: "#8b949e",
                gap: 12,
                fontSize: 14,
              }}
            >
              <div style={{ fontSize: 48 }}>🦞</div>
              <div>Select a model and click <strong>+ New Chat</strong> to begin</div>
              {models.filter((m) => m.provider === "ollama").length === 0 && (
                <div style={{ fontSize: 12, color: "#6e7681", maxWidth: 340, textAlign: "center" }}>
                  To use local models, install{" "}
                  <strong>Ollama</strong> and run{" "}
                  <code style={{ background: "#161b22", padding: "1px 4px", borderRadius: 4 }}>
                    ollama pull llama3
                  </code>
                  , then refresh.
                </div>
              )}
            </div>
          )}
        </main>
      </div>
    </div>
  );
}
