import { ModelInfo } from "../api";

interface Props {
  models: ModelInfo[];
  selected: string;
  onChange: (id: string) => void;
}

const PROVIDER_BADGE: Record<string, { bg: string; text: string; label: string }> = {
  anthropic: { bg: "#1e3a5f", text: "#7cc4fa", label: "Anthropic" },
  xai:        { bg: "#2d1e4a", text: "#b09fe8", label: "xAI" },
  ollama:     { bg: "#1e3a2a", text: "#6ee89a", label: "Ollama" },
};

function badge(provider: string) {
  return PROVIDER_BADGE[provider] ?? { bg: "#1e2a3a", text: "#aaa", label: provider };
}

export function ModelSelector({ models, selected, onChange }: Props) {
  if (models.length === 0) {
    return <span style={{ color: "#8b949e", fontSize: 13 }}>Loading models…</span>;
  }
  return (
    <select
      value={selected}
      onChange={(e) => onChange(e.target.value)}
      style={{
        background: "#161b22",
        color: "#e6edf3",
        border: "1px solid #30363d",
        borderRadius: 6,
        padding: "4px 8px",
        fontSize: 13,
        cursor: "pointer",
      }}
    >
      {models.map((m) => {
        const b = badge(m.provider);
        return (
          <option key={m.id} value={m.id}>
            [{b.label}] {m.label}
          </option>
        );
      })}
    </select>
  );
}
