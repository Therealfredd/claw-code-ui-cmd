//! Workspace-scoped tool execution for the agentic code-editing loop.
//!
//! Wraps the `tools` crate's `execute_tool` with path safety:
//! file paths in tool inputs are resolved relative to the session's workspace
//! directory and rejected if they escape it via `../` traversal.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use api::ToolDefinition;
use serde_json::Value;

/// Tool names allowed in code-editing sessions.
const CODE_TOOLS: &[&str] = &[
    "read_file",
    "write_file",
    "edit_file",
    "glob_search",
    "grep_search",
];

/// Returns `api::ToolDefinition` objects for the code-editing tool set.
/// These are passed to the model in `MessageRequest.tools`.
pub fn code_editing_tool_definitions() -> Vec<ToolDefinition> {
    let allowed: BTreeSet<String> = CODE_TOOLS.iter().map(|s| s.to_string()).collect();
    tools::GlobalToolRegistry::builtin().definitions(Some(&allowed))
}

/// Execute a named tool with the given JSON input, resolving and validating
/// any `path` / `directory` arguments against `workspace_dir`.
///
/// If `workspace_dir` is `None` the paths are passed through unmodified
/// (no restriction).  The function is intentionally synchronous — callers
/// should run it inside `tokio::task::spawn_blocking`.
pub fn execute_tool_in_workspace(
    name: &str,
    input: Value,
    workspace_dir: Option<&Path>,
) -> Result<String, String> {
    let safe_input = match workspace_dir {
        Some(ws) => patch_paths_in_input(input, ws)?,
        None => input,
    };
    tools::execute_tool(name, &safe_input)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Rewrite every `"path"` or `"directory"` string field in `input` so that
/// it points to the resolved, safe, canonical path within `ws`.
fn patch_paths_in_input(mut input: Value, ws: &Path) -> Result<Value, String> {
    for field in &["path", "directory"] {
        if let Some(val) = input
            .get(*field)
            .and_then(|v| v.as_str())
            .map(str::to_owned)
        {
            let resolved = resolve_safe_path(&val, ws)?;
            input[*field] = Value::String(resolved.to_string_lossy().into_owned());
        }
    }
    Ok(input)
}

/// Resolve `path_str` relative to `workspace`, canonicalize it, and verify
/// it does not escape the workspace.  Supports paths that don't exist yet
/// (e.g. new files that `write_file` will create).
fn resolve_safe_path(path_str: &str, workspace: &Path) -> Result<PathBuf, String> {
    let p = if Path::new(path_str).is_absolute() {
        PathBuf::from(path_str)
    } else {
        workspace.join(path_str)
    };

    let canonical_ws = workspace
        .canonicalize()
        .map_err(|e| format!("workspace directory not accessible: {e}"))?;

    // For files that already exist, canonicalize fully.
    // For new files (write_file target), canonicalize the parent directory.
    let canonical = if p.exists() {
        p.canonicalize().map_err(|e| e.to_string())?
    } else {
        let parent = p.parent().ok_or_else(|| "path has no parent".to_string())?;

        let parent_canon = if parent.as_os_str().is_empty() {
            // Bare filename — sits directly in the workspace root.
            canonical_ws.clone()
        } else {
            parent
                .canonicalize()
                .map_err(|e| format!("parent directory `{}` not found: {e}", parent.display()))?
        };

        let filename = p
            .file_name()
            .ok_or_else(|| format!("path `{path_str}` has no filename"))?;

        parent_canon.join(filename)
    };

    if !canonical.starts_with(&canonical_ws) {
        return Err(format!(
            "path `{path_str}` is outside the workspace directory"
        ));
    }

    Ok(canonical)
}
