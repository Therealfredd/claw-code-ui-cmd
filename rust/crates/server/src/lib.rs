mod workspace_tools;

use std::collections::HashMap;
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use api::{
    discover_ollama_models, max_tokens_for_model, InputContentBlock, InputMessage, MessageRequest,
    OutputContentBlock, ProviderClient, ToolDefinition, ToolResultContentBlock,
};
use async_stream::stream;
use axum::extract::{Path, State};
use axum::http::{Method, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use runtime::{ContentBlock, ConversationMessage, MessageRole, Session as RuntimeSession};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

pub type SessionId = String;
pub type SessionStore = Arc<RwLock<HashMap<SessionId, Session>>>;

const BROADCAST_CAPACITY: usize = 64;

/// Built-in cloud model names shown in the UI even without API keys configured.
const BUILTIN_ANTHROPIC_MODELS: &[&str] = &[
    "claude-opus-4-6",
    "claude-sonnet-4-6",
    "claude-haiku-4-5-20251213",
];
const BUILTIN_XAI_MODELS: &[&str] = &["grok-3", "grok-3-mini", "grok-2"];

#[derive(Clone)]
pub struct AppState {
    sessions: SessionStore,
    next_session_id: Arc<AtomicU64>,
    /// Directory to serve static UI files from (e.g. `web/dist`).
    pub static_dir: Option<PathBuf>,
    /// Default model used when creating a session without specifying one.
    pub default_model: String,
}

impl AppState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            next_session_id: Arc::new(AtomicU64::new(1)),
            static_dir: None,
            default_model: "claude-opus-4-6".to_string(),
        }
    }

    #[must_use]
    pub fn with_static_dir(mut self, dir: PathBuf) -> Self {
        self.static_dir = Some(dir);
        self
    }

    #[must_use]
    pub fn with_default_model(mut self, model: String) -> Self {
        self.default_model = model;
        self
    }

    fn allocate_session_id(&self) -> SessionId {
        let id = self.next_session_id.fetch_add(1, Ordering::Relaxed);
        format!("session-{id}")
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct Session {
    pub id: SessionId,
    pub created_at: u64,
    pub model: String,
    /// Absolute path to the workspace directory for this session.
    /// When set, file tool paths are resolved and constrained to this directory.
    pub workspace_dir: Option<PathBuf>,
    pub conversation: RuntimeSession,
    events: broadcast::Sender<SessionEvent>,
}

impl Session {
    fn new(id: SessionId, model: String, workspace_dir: Option<PathBuf>) -> Self {
        let (events, _) = broadcast::channel(BROADCAST_CAPACITY);
        Self {
            id,
            created_at: unix_timestamp_millis(),
            model,
            workspace_dir,
            conversation: RuntimeSession::new(),
            events,
        }
    }

    fn subscribe(&self) -> broadcast::Receiver<SessionEvent> {
        self.events.subscribe()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
enum SessionEvent {
    Snapshot {
        session_id: SessionId,
        session: RuntimeSession,
    },
    Message {
        session_id: SessionId,
        message: ConversationMessage,
    },
}

impl SessionEvent {
    fn event_name(&self) -> &'static str {
        match self {
            Self::Snapshot { .. } => "snapshot",
            Self::Message { .. } => "message",
        }
    }

    fn to_sse_event(&self) -> Result<Event, serde_json::Error> {
        Ok(Event::default()
            .event(self.event_name())
            .data(serde_json::to_string(self)?))
    }
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

type ApiError = (StatusCode, Json<ErrorResponse>);
type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateSessionRequest {
    #[serde(default)]
    pub model: Option<String>,
    /// Optional path to a local directory the model can read/edit files in.
    #[serde(default)]
    pub workspace_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateSessionResponse {
    pub session_id: SessionId,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionSummary {
    pub id: SessionId,
    pub created_at: u64,
    pub model: String,
    pub message_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListSessionsResponse {
    pub sessions: Vec<SessionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionDetailsResponse {
    pub id: SessionId,
    pub created_at: u64,
    pub model: String,
    pub session: RuntimeSession,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SendMessageRequest {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub provider: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsResponse {
    pub models: Vec<ModelInfo>,
}

/// Convert stored runtime messages into the format the API layer expects.
fn to_api_messages(messages: &[ConversationMessage]) -> Vec<InputMessage> {
    messages
        .iter()
        .filter_map(|msg| {
            let role = match msg.role {
                MessageRole::User => "user".to_string(),
                MessageRole::Assistant => "assistant".to_string(),
                MessageRole::System | MessageRole::Tool => return None,
            };
            let content: Vec<InputContentBlock> = msg
                .blocks
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::Text { text } => {
                        Some(InputContentBlock::Text { text: text.clone() })
                    }
                    ContentBlock::ToolUse { id, name, input } => {
                        Some(InputContentBlock::ToolUse {
                            id: id.clone(),
                            name: name.clone(),
                            input: serde_json::from_str(input).unwrap_or(serde_json::json!({})),
                        })
                    }
                    ContentBlock::ToolResult {
                        tool_use_id,
                        output,
                        is_error,
                        ..
                    } => Some(InputContentBlock::ToolResult {
                        tool_use_id: tool_use_id.clone(),
                        content: vec![ToolResultContentBlock::Text {
                            text: output.clone(),
                        }],
                        is_error: *is_error,
                    }),
                })
                .collect();
            if content.is_empty() {
                None
            } else {
                Some(InputMessage { role, content })
            }
        })
        .collect()
}

/// Convert an API response into a runtime `ConversationMessage`.
fn from_api_response(response: api::MessageResponse) -> ConversationMessage {
    let blocks: Vec<ContentBlock> = response
        .content
        .into_iter()
        .filter_map(|block| match block {
            OutputContentBlock::Text { text } => Some(ContentBlock::Text { text }),
            OutputContentBlock::ToolUse { id, name, input } => Some(ContentBlock::ToolUse {
                id,
                name,
                input: input.to_string(),
            }),
            _ => None,
        })
        .collect();
    ConversationMessage {
        role: MessageRole::Assistant,
        blocks,
        usage: None,
    }
}

#[must_use]
pub fn app(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);

    let api_routes = Router::new()
        .route("/sessions", post(create_session).get(list_sessions))
        .route("/sessions/{id}", get(get_session))
        .route("/sessions/{id}/events", get(stream_session_events))
        .route("/sessions/{id}/message", post(send_message))
        .route("/api/models", get(list_models));

    let router = api_routes.layer(cors).with_state(state.clone());

    if let Some(dir) = state.static_dir.clone() {
        router.fallback_service(ServeDir::new(dir).append_index_html_on_directories(true))
    } else {
        router
    }
}

async fn create_session(
    State(state): State<AppState>,
    body: Option<Json<CreateSessionRequest>>,
) -> (StatusCode, Json<CreateSessionResponse>) {
    let (model, workspace_dir) = match body {
        Some(Json(req)) => {
            let model = req.model.unwrap_or_else(|| state.default_model.clone());
            // Accept the workspace path only if it points to an existing directory.
            let workspace_dir = req
                .workspace_dir
                .as_deref()
                .map(PathBuf::from)
                .filter(|p| p.is_dir());
            (model, workspace_dir)
        }
        None => (state.default_model.clone(), None),
    };

    let session_id = state.allocate_session_id();
    let session = Session::new(session_id.clone(), model.clone(), workspace_dir);

    state
        .sessions
        .write()
        .await
        .insert(session_id.clone(), session);

    (
        StatusCode::CREATED,
        Json(CreateSessionResponse { session_id, model }),
    )
}

async fn list_sessions(State(state): State<AppState>) -> Json<ListSessionsResponse> {
    let sessions = state.sessions.read().await;
    let mut summaries = sessions
        .values()
        .map(|session| SessionSummary {
            id: session.id.clone(),
            created_at: session.created_at,
            model: session.model.clone(),
            message_count: session.conversation.messages.len(),
        })
        .collect::<Vec<_>>();
    summaries.sort_by(|left, right| left.id.cmp(&right.id));

    Json(ListSessionsResponse {
        sessions: summaries,
    })
}

async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<SessionId>,
) -> ApiResult<Json<SessionDetailsResponse>> {
    let sessions = state.sessions.read().await;
    let session = sessions
        .get(&id)
        .ok_or_else(|| not_found(format!("session `{id}` not found")))?;

    Ok(Json(SessionDetailsResponse {
        id: session.id.clone(),
        created_at: session.created_at,
        model: session.model.clone(),
        session: session.conversation.clone(),
    }))
}

async fn send_message(
    State(state): State<AppState>,
    Path(id): Path<SessionId>,
    Json(payload): Json<SendMessageRequest>,
) -> ApiResult<StatusCode> {
    let user_message = ConversationMessage::user_text(payload.message);

    // Store user message and collect what we need for the model call.
    let (model, mut current_messages, broadcaster, workspace_dir) = {
        let mut sessions = state.sessions.write().await;
        let session = sessions
            .get_mut(&id)
            .ok_or_else(|| not_found(format!("session `{id}` not found")))?;
        session.conversation.messages.push(user_message.clone());
        (
            session.model.clone(),
            session.conversation.messages.clone(),
            session.events.clone(),
            session.workspace_dir.clone(),
        )
    };

    // Broadcast user message immediately so the UI shows it right away.
    let _ = broadcaster.send(SessionEvent::Message {
        session_id: id.clone(),
        message: user_message,
    });

    // Run the agentic loop in a background task so we return 204 immediately.
    let state_bg = state.clone();
    tokio::spawn(async move {
        let tool_defs: Vec<ToolDefinition> = workspace_tools::code_editing_tool_definitions();
        let system = build_system_prompt(workspace_dir.as_deref());

        loop {
            let api_messages = to_api_messages(&current_messages);
            let request = MessageRequest {
                model: model.clone(),
                max_tokens: max_tokens_for_model(&model),
                messages: api_messages,
                system: Some(system.clone()),
                tools: Some(tool_defs.clone()),
                tool_choice: None,
                stream: false,
            };

            let result = match ProviderClient::from_model(&model) {
                Ok(client) => client.send_message(&request).await,
                Err(e) => {
                    eprintln!("[claw-server] provider init failed for {model}: {e}");
                    Err(e)
                }
            };

            let assistant_msg = match result {
                Ok(response) => from_api_response(response),
                Err(e) => {
                    eprintln!("[claw-server] model call failed: {e}");
                    let err_msg = ConversationMessage {
                        role: MessageRole::Assistant,
                        blocks: vec![ContentBlock::Text {
                            text: format!("⚠ Model error: {e}"),
                        }],
                        usage: None,
                    };
                    push_and_broadcast(&state_bg, &id, &broadcaster, err_msg).await;
                    break;
                }
            };

            // Persist the assistant turn and tell the UI about it immediately.
            push_and_broadcast(&state_bg, &id, &broadcaster, assistant_msg.clone()).await;
            current_messages.push(assistant_msg.clone());

            // Collect all tool_use blocks from the assistant response.
            let tool_uses: Vec<(String, String, String)> = assistant_msg
                .blocks
                .iter()
                .filter_map(|b| {
                    if let ContentBlock::ToolUse {
                        id: tool_id,
                        name,
                        input,
                    } = b
                    {
                        Some((tool_id.clone(), name.clone(), input.clone()))
                    } else {
                        None
                    }
                })
                .collect();

            // No tool calls → model is done.
            if tool_uses.is_empty() {
                break;
            }

            // Execute each tool and gather results into a single user message.
            let mut result_blocks = Vec::new();
            for (tool_use_id, tool_name, input_str) in tool_uses {
                let input_val: serde_json::Value =
                    serde_json::from_str(&input_str).unwrap_or(serde_json::Value::Null);
                let ws_clone = workspace_dir.clone();
                let tool_name_for_closure = tool_name.clone();

                let result = tokio::task::spawn_blocking(move || {
                    workspace_tools::execute_tool_in_workspace(
                        &tool_name_for_closure,
                        input_val,
                        ws_clone.as_deref(),
                    )
                })
                .await
                .unwrap_or_else(|e| Err(format!("spawn_blocking failed: {e}")));

                let (output, is_error) = match result {
                    Ok(out) => (out, false),
                    Err(e) => (e, true),
                };
                result_blocks.push(ContentBlock::ToolResult {
                    tool_use_id,
                    tool_name,
                    output,
                    is_error,
                });
            }

            let tool_result_msg = ConversationMessage {
                role: MessageRole::User,
                blocks: result_blocks,
                usage: None,
            };
            push_and_broadcast(&state_bg, &id, &broadcaster, tool_result_msg.clone()).await;
            current_messages.push(tool_result_msg);
            // Loop: call the model again with the tool results in context.
        }
    });

    Ok(StatusCode::NO_CONTENT)
}

/// Persist a message to the session store and broadcast it over SSE.
async fn push_and_broadcast(
    state: &AppState,
    id: &str,
    broadcaster: &broadcast::Sender<SessionEvent>,
    msg: ConversationMessage,
) {
    {
        let mut sessions = state.sessions.write().await;
        if let Some(session) = sessions.get_mut(id) {
            session.conversation.messages.push(msg.clone());
        }
    }
    let _ = broadcaster.send(SessionEvent::Message {
        session_id: id.to_string(),
        message: msg,
    });
}

/// Build the system prompt, optionally including the workspace path.
fn build_system_prompt(workspace_dir: Option<&std::path::Path>) -> String {
    let ws_info = workspace_dir
        .map(|p| format!("\n\nYour workspace directory is: {}", p.display()))
        .unwrap_or_default();
    format!(
        "You are a helpful coding assistant with access to file editing tools. \
         Use read_file to understand existing code, write_file to create new files, \
         edit_file to make precise targeted edits to existing files, glob_search to \
         find files by pattern, and grep_search to search for code patterns.{ws_info}"
    )
}

async fn stream_session_events(
    State(state): State<AppState>,
    Path(id): Path<SessionId>,
) -> ApiResult<impl IntoResponse> {
    let (snapshot, mut receiver) = {
        let sessions = state.sessions.read().await;
        let session = sessions
            .get(&id)
            .ok_or_else(|| not_found(format!("session `{id}` not found")))?;
        (
            SessionEvent::Snapshot {
                session_id: session.id.clone(),
                session: session.conversation.clone(),
            },
            session.subscribe(),
        )
    };

    let stream = stream! {
        if let Ok(event) = snapshot.to_sse_event() {
            yield Ok::<Event, Infallible>(event);
        }

        loop {
            match receiver.recv().await {
                Ok(event) => {
                    if let Ok(sse_event) = event.to_sse_event() {
                        yield Ok::<Event, Infallible>(sse_event);
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}

async fn list_models() -> Json<ModelsResponse> {
    let mut models: Vec<ModelInfo> = BUILTIN_ANTHROPIC_MODELS
        .iter()
        .map(|name| ModelInfo {
            id: (*name).to_string(),
            provider: "anthropic".to_string(),
            label: (*name).to_string(),
        })
        .collect();

    for name in BUILTIN_XAI_MODELS {
        models.push(ModelInfo {
            id: (*name).to_string(),
            provider: "xai".to_string(),
            label: format!("{name} (xAI)"),
        });
    }

    for name in discover_ollama_models().await {
        models.push(ModelInfo {
            id: format!("ollama:{name}"),
            provider: "ollama".to_string(),
            label: format!("{name} (Ollama)"),
        });
    }

    Json(ModelsResponse { models })
}

/// Bind to `addr` and serve the Claw HTTP/SSE API + static UI files.
pub async fn listen_and_serve(addr: &str, state: AppState) -> Result<(), std::io::Error> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app(state)).await
}

fn unix_timestamp_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_millis() as u64
}

fn not_found(message: String) -> ApiError {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse { error: message }),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        app, AppState, CreateSessionResponse, ListSessionsResponse, SessionDetailsResponse,
    };
    use reqwest::Client;
    use std::net::SocketAddr;
    use std::time::Duration;
    use tokio::net::TcpListener;
    use tokio::task::JoinHandle;
    use tokio::time::timeout;

    struct TestServer {
        address: SocketAddr,
        handle: JoinHandle<()>,
    }

    impl TestServer {
        async fn spawn() -> Self {
            let listener = TcpListener::bind("127.0.0.1:0")
                .await
                .expect("test listener should bind");
            let address = listener
                .local_addr()
                .expect("listener should report local address");
            let handle = tokio::spawn(async move {
                axum::serve(listener, app(AppState::default()))
                    .await
                    .expect("server should run");
            });

            Self { address, handle }
        }

        fn url(&self, path: &str) -> String {
            format!("http://{}{}", self.address, path)
        }
    }

    impl Drop for TestServer {
        fn drop(&mut self) {
            self.handle.abort();
        }
    }

    async fn create_session(client: &Client, server: &TestServer) -> CreateSessionResponse {
        client
            .post(server.url("/sessions"))
            .send()
            .await
            .expect("create request should succeed")
            .error_for_status()
            .expect("create request should return success")
            .json::<CreateSessionResponse>()
            .await
            .expect("create response should parse")
    }

    async fn next_sse_frame(response: &mut reqwest::Response, buffer: &mut String) -> String {
        loop {
            if let Some(index) = buffer.find("\n\n") {
                let frame = buffer[..index].to_string();
                let remainder = buffer[index + 2..].to_string();
                *buffer = remainder;
                return frame;
            }

            let next_chunk = timeout(Duration::from_secs(5), response.chunk())
                .await
                .expect("SSE stream should yield within timeout")
                .expect("SSE stream should remain readable")
                .expect("SSE stream should stay open");
            buffer.push_str(&String::from_utf8_lossy(&next_chunk));
        }
    }

    #[tokio::test]
    async fn creates_and_lists_sessions() {
        let server = TestServer::spawn().await;
        let client = Client::new();

        // given
        let created = create_session(&client, &server).await;

        // when
        let sessions = client
            .get(server.url("/sessions"))
            .send()
            .await
            .expect("list request should succeed")
            .error_for_status()
            .expect("list request should return success")
            .json::<ListSessionsResponse>()
            .await
            .expect("list response should parse");
        let details = client
            .get(server.url(&format!("/sessions/{}", created.session_id)))
            .send()
            .await
            .expect("details request should succeed")
            .error_for_status()
            .expect("details request should return success")
            .json::<SessionDetailsResponse>()
            .await
            .expect("details response should parse");

        // then
        assert_eq!(created.session_id, "session-1");
        assert_eq!(sessions.sessions.len(), 1);
        assert_eq!(sessions.sessions[0].id, created.session_id);
        assert_eq!(sessions.sessions[0].message_count, 0);
        assert_eq!(details.id, "session-1");
        assert!(details.session.messages.is_empty());
    }

    #[tokio::test]
    async fn streams_message_events_and_persists_message_flow() {
        let server = TestServer::spawn().await;
        let client = Client::new();

        // given
        let created = create_session(&client, &server).await;
        let mut response = client
            .get(server.url(&format!("/sessions/{}/events", created.session_id)))
            .send()
            .await
            .expect("events request should succeed")
            .error_for_status()
            .expect("events request should return success");
        let mut buffer = String::new();
        let snapshot_frame = next_sse_frame(&mut response, &mut buffer).await;

        // when
        let send_status = client
            .post(server.url(&format!("/sessions/{}/message", created.session_id)))
            .json(&super::SendMessageRequest {
                message: "hello from test".to_string(),
            })
            .send()
            .await
            .expect("message request should succeed")
            .status();
        let message_frame = next_sse_frame(&mut response, &mut buffer).await;
        let details = client
            .get(server.url(&format!("/sessions/{}", created.session_id)))
            .send()
            .await
            .expect("details request should succeed")
            .error_for_status()
            .expect("details request should return success")
            .json::<SessionDetailsResponse>()
            .await
            .expect("details response should parse");

        // then
        assert_eq!(send_status, reqwest::StatusCode::NO_CONTENT);
        assert!(snapshot_frame.contains("event: snapshot"));
        assert!(snapshot_frame.contains("\"session_id\":\"session-1\""));
        assert!(message_frame.contains("event: message"));
        assert!(message_frame.contains("hello from test"));
        assert_eq!(details.session.messages.len(), 1);
        assert_eq!(
            details.session.messages[0],
            runtime::ConversationMessage::user_text("hello from test")
        );
    }

    #[tokio::test]
    async fn models_endpoint_returns_builtin_models() {
        let server = TestServer::spawn().await;
        let client = Client::new();

        let response = client
            .get(server.url("/api/models"))
            .send()
            .await
            .expect("models request should succeed")
            .error_for_status()
            .expect("models request should return success")
            .json::<super::ModelsResponse>()
            .await
            .expect("models response should parse");

        assert!(
            response
                .models
                .iter()
                .any(|m| m.id == "claude-opus-4-6"),
            "should include claude-opus-4-6"
        );
        assert!(
            response
                .models
                .iter()
                .any(|m| m.provider == "anthropic"),
            "should include anthropic provider"
        );
    }

    #[tokio::test]
    async fn create_session_with_model() {
        let server = TestServer::spawn().await;
        let client = Client::new();

        let created = client
            .post(server.url("/sessions"))
            .json(&super::CreateSessionRequest {
                model: Some("claude-sonnet-4-6".to_string()),
                workspace_dir: None,
            })
            .send()
            .await
            .expect("create request should succeed")
            .error_for_status()
            .expect("create request should return success")
            .json::<CreateSessionResponse>()
            .await
            .expect("create response should parse");

        assert_eq!(created.model, "claude-sonnet-4-6");
    }
}
