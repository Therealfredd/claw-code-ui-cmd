import { useEffect, useRef, useState } from "react";
import { ContentBlock, ConversationMessage } from "../api";

interface Props {
  messages: ConversationMessage[];
  streaming: boolean;
}

const ROLE_COLORS: Record<string, string> = {
  user: "#1f6feb",
  assistant: "#238636",
  tool: "#e3a343",
};

export function ChatWindow({ messages, streaming }: Props) {
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, streaming]);

  if (messages.length === 0 && !streaming) {
    return (
      <div
        style={{
          flex: 1,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          color: "#8b949e",
          fontSize: 14,
        }}
      >
        Send a message to start the conversation
      </div>
    );
  }

  return (
    <div
      style={{
        flex: 1,
        overflowY: "auto",
        padding: "16px 20px",
        display: "flex",
        flexDirection: "column",
        gap: 12,
      }}
    >
      {messages.map((msg, i) => (
        <MessageBubble key={i} message={msg} />
      ))}
      {streaming && (
        <div style={{ color: "#8b949e", fontSize: 13 }}>
          ⠋ Thinking…
        </div>
      )}
      <div ref={bottomRef} />
    </div>
  );
}

const MAX_OUTPUT_PREVIEW = 400;

function ToolUseBlock({ block, index }: { block: Extract<ContentBlock, { type: "tool_use" }>; index: number }) {
  const [expanded, setExpanded] = useState(false);
  let parsed: unknown;
  try { parsed = JSON.parse(block.input); } catch { parsed = block.input; }
  const pretty = typeof parsed === "string" ? parsed : JSON.stringify(parsed, null, 2);

  return (
    <div
      key={index}
      style={{
        fontSize: 12,
        fontFamily: "monospace",
        background: "#1a1a2e",
        border: "1px solid #3a3a6e",
        borderRadius: 6,
        marginTop: 6,
        overflow: "hidden",
      }}
    >
      <button
        onClick={() => setExpanded((x) => !x)}
        style={{
          width: "100%",
          textAlign: "left",
          background: "none",
          border: "none",
          color: "#c9a8ff",
          cursor: "pointer",
          padding: "4px 8px",
          fontFamily: "monospace",
          fontSize: 12,
          display: "flex",
          alignItems: "center",
          gap: 6,
        }}
      >
        <span>🔧</span>
        <strong>{block.name}</strong>
        <span style={{ color: "#8b949e", marginLeft: "auto" }}>{expanded ? "▲" : "▼"}</span>
      </button>
      {expanded && (
        <pre
          style={{
            margin: 0,
            padding: "4px 8px 6px",
            color: "#e6edf3",
            whiteSpace: "pre-wrap",
            wordBreak: "break-all",
            borderTop: "1px solid #3a3a6e",
            fontSize: 11,
          }}
        >
          {pretty}
        </pre>
      )}
    </div>
  );
}

function ToolResultBlock({ block, index }: { block: Extract<ContentBlock, { type: "tool_result" }>; index: number }) {
  const [expanded, setExpanded] = useState(false);
  const isLong = block.output.length > MAX_OUTPUT_PREVIEW;
  const displayText = expanded || !isLong ? block.output : block.output.slice(0, MAX_OUTPUT_PREVIEW) + "…";

  return (
    <div
      key={index}
      style={{
        fontSize: 12,
        fontFamily: "monospace",
        background: block.is_error ? "#2d0f0f" : "#0f1f0f",
        border: `1px solid ${block.is_error ? "#6e1e1e" : "#1e4e1e"}`,
        borderRadius: 6,
        marginTop: 4,
        overflow: "hidden",
      }}
    >
      <div
        style={{
          padding: "3px 8px",
          color: block.is_error ? "#f85149" : "#6ee89a",
          display: "flex",
          alignItems: "center",
          gap: 6,
        }}
      >
        <span>{block.is_error ? "✗" : "✓"}</span>
        <strong>{block.tool_name}</strong>
      </div>
      <pre
        style={{
          margin: 0,
          padding: "2px 8px 6px",
          color: "#c9d1d9",
          whiteSpace: "pre-wrap",
          wordBreak: "break-all",
          borderTop: `1px solid ${block.is_error ? "#6e1e1e" : "#1e4e1e"}`,
          fontSize: 11,
          maxHeight: expanded ? "none" : undefined,
        }}
      >
        {displayText}
      </pre>
      {isLong && (
        <button
          onClick={() => setExpanded((x) => !x)}
          style={{
            background: "none",
            border: "none",
            color: "#8b949e",
            cursor: "pointer",
            padding: "2px 8px 4px",
            fontSize: 11,
            fontFamily: "monospace",
          }}
        >
          {expanded ? "▲ Show less" : "▼ Show more"}
        </button>
      )}
    </div>
  );
}

function renderBlock(block: ContentBlock, i: number) {
  if (block.type === "text") {
    return (
      <span key={i} style={{ whiteSpace: "pre-wrap", wordBreak: "break-word" }}>
        {block.text}
      </span>
    );
  }
  if (block.type === "tool_use") {
    return <ToolUseBlock key={i} block={block} index={i} />;
  }
  if (block.type === "tool_result") {
    return <ToolResultBlock key={i} block={block} index={i} />;
  }
  return null;
}

function MessageBubble({ message }: { message: ConversationMessage }) {
  const isUser = message.role === "user";
  const color = ROLE_COLORS[message.role] ?? "#6e7681";

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        alignItems: isUser ? "flex-end" : "flex-start",
        gap: 4,
      }}
    >
      <div
        style={{
          fontSize: 11,
          color,
          textTransform: "uppercase",
          letterSpacing: "0.05em",
          fontWeight: 600,
        }}
      >
        {message.role}
      </div>
      <div
        style={{
          maxWidth: "80%",
          background: isUser ? "#0c2d6b" : "#161b22",
          border: `1px solid ${isUser ? "#1f6feb" : "#30363d"}`,
          borderRadius: isUser ? "12px 12px 4px 12px" : "12px 12px 12px 4px",
          padding: "8px 12px",
          fontSize: 14,
          lineHeight: 1.6,
        }}
      >
        {message.blocks.map((block, i) => renderBlock(block, i))}
      </div>
    </div>
  );
}
