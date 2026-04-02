const BASE = "";

export interface ModelInfo {
  id: string;
  provider: string;
  label: string;
}

export interface SessionSummary {
  id: string;
  created_at: number;
  model: string;
  message_count: number;
}

export type ContentBlock =
  | { type: "text"; text: string }
  | { type: "tool_use"; id: string; name: string; input: string }
  | { type: "tool_result"; tool_use_id: string; tool_name: string; output: string; is_error: boolean };

export interface ConversationMessage {
  role: "user" | "assistant" | "system" | "tool";
  blocks: ContentBlock[];
  usage?: unknown;
}

export interface Session {
  messages: ConversationMessage[];
}

export interface SessionDetails {
  id: string;
  created_at: number;
  model: string;
  session: Session;
}

export interface CreateSessionResponse {
  session_id: string;
  model: string;
}

export async function fetchModels(): Promise<ModelInfo[]> {
  const res = await fetch(`${BASE}/api/models`);
  if (!res.ok) throw new Error(`Failed to fetch models: ${res.status}`);
  const data = await res.json();
  return data.models as ModelInfo[];
}

export async function fetchSessions(): Promise<SessionSummary[]> {
  const res = await fetch(`${BASE}/sessions`);
  if (!res.ok) throw new Error(`Failed to fetch sessions: ${res.status}`);
  const data = await res.json();
  return data.sessions as SessionSummary[];
}

export async function createSession(model: string, workspaceDir?: string): Promise<CreateSessionResponse> {
  const res = await fetch(`${BASE}/sessions`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ model, workspace_dir: workspaceDir ?? null }),
  });
  if (!res.ok) throw new Error(`Failed to create session: ${res.status}`);
  return res.json();
}

export async function fetchSession(id: string): Promise<SessionDetails> {
  const res = await fetch(`${BASE}/sessions/${id}`);
  if (!res.ok) throw new Error(`Failed to fetch session: ${res.status}`);
  return res.json();
}

export async function sendMessage(sessionId: string, message: string): Promise<void> {
  const res = await fetch(`${BASE}/sessions/${sessionId}/message`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ message }),
  });
  if (!res.ok) throw new Error(`Failed to send message: ${res.status}`);
}

export function openEventStream(sessionId: string): EventSource {
  return new EventSource(`${BASE}/sessions/${sessionId}/events`);
}
