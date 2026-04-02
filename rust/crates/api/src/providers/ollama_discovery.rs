use serde::Deserialize;

use super::openai_compat::DEFAULT_OLLAMA_BASE_URL;

#[derive(Debug, Deserialize)]
struct TagsResponse {
    models: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    name: String,
}

/// Query the Ollama `/api/tags` endpoint and return a list of available model names.
/// Returns an empty list if Ollama is not running or unreachable.
pub async fn discover_ollama_models() -> Vec<String> {
    let base_url = std::env::var("OLLAMA_BASE_URL")
        .unwrap_or_else(|_| DEFAULT_OLLAMA_BASE_URL.to_string());

    // The tags endpoint is on the root, not under /v1
    let tags_url = base_url
        .trim_end_matches('/')
        .trim_end_matches("/v1")
        .trim_end_matches('/')
        .to_string()
        + "/api/tags";

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .unwrap_or_default();

    match client.get(&tags_url).send().await {
        Ok(response) if response.status().is_success() => {
            match response.json::<TagsResponse>().await {
                Ok(tags) => tags.models.into_iter().map(|m| m.name).collect(),
                Err(_) => Vec::new(),
            }
        }
        _ => Vec::new(),
    }
}
