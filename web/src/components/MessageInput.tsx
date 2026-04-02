import { KeyboardEvent, useRef, useState } from "react";

interface Props {
  onSend: (text: string) => void;
  disabled: boolean;
}

export function MessageInput({ onSend, disabled }: Props) {
  const [value, setValue] = useState("");
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  function submit() {
    const text = value.trim();
    if (!text || disabled) return;
    onSend(text);
    setValue("");
    if (textareaRef.current) {
      textareaRef.current.style.height = "auto";
    }
  }

  function onKeyDown(e: KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
  }

  function onInput() {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${Math.min(el.scrollHeight, 200)}px`;
  }

  return (
    <div
      style={{
        padding: "12px 16px",
        borderTop: "1px solid #21262d",
        display: "flex",
        gap: 8,
        alignItems: "flex-end",
        background: "#0d1117",
      }}
    >
      <textarea
        ref={textareaRef}
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={onKeyDown}
        onInput={onInput}
        placeholder={disabled ? "Select a session to chat…" : "Message… (Enter to send, Shift+Enter for newline)"}
        disabled={disabled}
        rows={1}
        style={{
          flex: 1,
          resize: "none",
          background: "#161b22",
          color: "#e6edf3",
          border: "1px solid #30363d",
          borderRadius: 8,
          padding: "8px 12px",
          fontSize: 14,
          lineHeight: 1.5,
          fontFamily: "inherit",
          outline: "none",
          overflow: "hidden",
        }}
      />
      <button
        onClick={submit}
        disabled={disabled || !value.trim()}
        style={{
          background: disabled || !value.trim() ? "#21262d" : "#1f6feb",
          color: disabled || !value.trim() ? "#6e7681" : "#fff",
          border: "none",
          borderRadius: 8,
          padding: "8px 16px",
          fontSize: 14,
          cursor: disabled || !value.trim() ? "default" : "pointer",
          fontFamily: "inherit",
          whiteSpace: "nowrap",
          transition: "background 0.15s",
        }}
      >
        Send
      </button>
    </div>
  );
}
