use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::AppState;

const CONFIG_FILENAME: &str = "ai-config.json";
const DEFAULT_BASE_URL: &str = "https://api.featherless.ai/v1";
const DEFAULT_MODEL: &str = "meta-llama/Meta-Llama-3.1-8B-Instruct";
const CHAT_TIMEOUT: Duration = Duration::from_secs(90);
/// Featherless defaults `max_tokens` very high; that can exceed `context − prompt` and yield 400.
const DEFAULT_MAX_COMPLETION_TOKENS: u32 = 4096;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct AiConfigFile {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    api_key: String,
    /// OpenAI-compatible API root, e.g. `https://api.featherless.ai/v1`
    #[serde(default)]
    base_url: String,
    #[serde(default)]
    model: String,
}

fn config_path(state: &AppState) -> PathBuf {
    state.app_config_dir.join(CONFIG_FILENAME)
}

fn load_config(state: &AppState) -> Result<AiConfigFile, String> {
    let path = config_path(state);
    if !path.is_file() {
        return Ok(AiConfigFile::default());
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| format!("ai config: {e}"))
}

fn save_config(state: &AppState, cfg: &AiConfigFile) -> Result<(), String> {
    let path = config_path(state);
    let raw = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(&path, raw).map_err(|e| e.to_string())
}

fn effective_api_key(cfg: &AiConfigFile) -> Option<String> {
    let from_file = cfg.api_key.trim();
    if !from_file.is_empty() {
        return Some(from_file.to_string());
    }
    std::env::var("FEATHERLESS_API_KEY")
        .ok()
        .filter(|s| !s.trim().is_empty())
}

fn effective_base_url(cfg: &AiConfigFile) -> String {
    let u = cfg.base_url.trim();
    if u.is_empty() {
        DEFAULT_BASE_URL.to_string()
    } else {
        u.trim_end_matches('/').to_string()
    }
}

fn effective_model(cfg: &AiConfigFile) -> String {
    let m = cfg.model.trim();
    if m.is_empty() {
        DEFAULT_MODEL.to_string()
    } else {
        m.to_string()
    }
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiStatus {
    pub enabled: bool,
    pub has_api_key: bool,
    pub base_url: String,
    pub model: String,
    pub default_base_url: String,
    pub default_model: String,
}

#[tauri::command]
pub fn ai_get_status(state: tauri::State<'_, AppState>) -> Result<AiStatus, String> {
    let cfg = load_config(&state)?;
    let has_api_key = effective_api_key(&cfg).is_some();
    Ok(AiStatus {
        enabled: cfg.enabled,
        has_api_key,
        base_url: effective_base_url(&cfg),
        model: effective_model(&cfg),
        default_base_url: DEFAULT_BASE_URL.to_string(),
        default_model: DEFAULT_MODEL.to_string(),
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSetConfigPayload {
    pub enabled: bool,
    /// `None` = leave unchanged; `Some("")` = clear stored key; `Some(s)` = set.
    pub api_key: Option<String>,
    /// `None` = unchanged; `Some(s)` = set (empty string resets to default URL).
    pub base_url: Option<String>,
    /// `None` = unchanged; `Some(s)` = set (empty string resets to default model).
    pub model: Option<String>,
}

#[tauri::command]
pub fn ai_set_config(
    state: tauri::State<'_, AppState>,
    payload: AiSetConfigPayload,
) -> Result<AiStatus, String> {
    let mut cfg = load_config(&state)?;
    cfg.enabled = payload.enabled;
    if let Some(key) = payload.api_key {
        cfg.api_key = key;
    }
    if let Some(url) = payload.base_url {
        cfg.base_url = url.trim().to_string();
    }
    if let Some(model) = payload.model {
        cfg.model = model.trim().to_string();
    }
    save_config(&state, &cfg)?;
    ai_get_status(state)
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AiChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiChatPayload {
    pub messages: Vec<AiChatMessage>,
    pub temperature: Option<f32>,
}

#[derive(Serialize)]
struct OpenAiChatRequest<'a> {
    model: &'a str,
    messages: &'a [AiChatMessage],
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    max_tokens: u32,
}

#[derive(Deserialize)]
struct OpenAiChatResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiAssistantMessage,
}

#[derive(Deserialize)]
struct OpenAiAssistantMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiErrorBody {
    error: Option<OpenAiErrorDetail>,
}

#[derive(Deserialize)]
struct OpenAiErrorDetail {
    message: Option<String>,
}

/// Stable prefixes for UI mapping (`AiAssistantPanel.mapInvokeError`).
fn openai_http_error_message(status: reqwest::StatusCode, text: &str) -> String {
    let trimmed = text.trim();
    if let Ok(err) = serde_json::from_str::<OpenAiErrorBody>(trimmed) {
        if let Some(msg) = err.error.and_then(|e| e.message) {
            let m = msg.trim();
            if !m.is_empty() {
                return format!("API error ({status}): {m}");
            }
        }
    }
    let head = trimmed.trim_start();
    if head.starts_with('<') {
        return format!("api_html_response:{status}");
    }
    if status.is_server_error() {
        return format!("api_server_error:{status}");
    }
    let preview: String = trimmed.chars().take(160).collect();
    if preview.is_empty() {
        format!("API error ({status})")
    } else {
        format!("API error ({status}): {preview}")
    }
}

#[tauri::command]
pub async fn ai_chat(
    state: tauri::State<'_, AppState>,
    payload: AiChatPayload,
) -> Result<String, String> {
    let cfg = load_config(&state)?;
    if !cfg.enabled {
        return Err("AI assistant is disabled".to_string());
    }
    let Some(api_key) = effective_api_key(&cfg) else {
        return Err("No API key configured".to_string());
    };
    if payload.messages.is_empty() {
        return Err("No messages".to_string());
    }

    let base = effective_base_url(&cfg);
    let model = effective_model(&cfg);
    let url = format!("{}/chat/completions", base.trim_end_matches('/'));

    let body = OpenAiChatRequest {
        model: &model,
        messages: &payload.messages,
        temperature: payload.temperature,
        max_tokens: DEFAULT_MAX_COMPLETION_TOKENS,
    };

    let client = reqwest::Client::builder()
        .timeout(CHAT_TIMEOUT)
        .build()
        .map_err(|e| e.to_string())?;

    let res = client
        .post(&url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                "Request timed out".to_string()
            } else if e.is_connect() || e.is_request() {
                format!("Network error: {e}")
            } else {
                e.to_string()
            }
        })?;

    let status = res.status();
    let text = res.text().await.map_err(|e| e.to_string())?;

    if !status.is_success() {
        return Err(openai_http_error_message(status, &text));
    }

    let parsed: OpenAiChatResponse =
        serde_json::from_str(&text).map_err(|e| format!("Invalid API response: {e}"))?;
    let content = parsed
        .choices
        .first()
        .and_then(|c| c.message.content.clone())
        .unwrap_or_default();
    Ok(content)
}
