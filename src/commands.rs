use tauri::{State, ipc::Channel};

use crate::backend::{
    BootstrapResponse, ChatStreamEvent, CoworkBackend, LoadedSession, SendMessageRequest,
    SettingsSnapshot,
};
use crate::storage::llm_provider_config::LlmProviderConfig;

#[tauri::command]
pub fn bootstrap_app(state: State<'_, CoworkBackend>) -> Result<BootstrapResponse, String> {
    state.bootstrap().map_err(format_error)
}

#[tauri::command]
pub fn load_session(
    state: State<'_, CoworkBackend>,
    session_id: String,
) -> Result<LoadedSession, String> {
    state.load_session(&session_id).map_err(format_error)
}

#[tauri::command]
pub fn new_session(state: State<'_, CoworkBackend>) -> Result<LoadedSession, String> {
    state.new_session().map_err(format_error)
}

#[tauri::command]
pub fn delete_session(
    state: State<'_, CoworkBackend>,
    session_id: String,
) -> Result<LoadedSession, String> {
    state.delete_session(&session_id).map_err(format_error)
}

#[tauri::command]
pub async fn send_message(
    state: State<'_, CoworkBackend>,
    request: SendMessageRequest,
    events: Channel<ChatStreamEvent>,
) -> Result<LoadedSession, String> {
    state
        .send_message(request, events)
        .await
        .map_err(format_error)
}

#[tauri::command]
pub fn get_settings(state: State<'_, CoworkBackend>) -> Result<SettingsSnapshot, String> {
    state.settings_snapshot().map_err(format_error)
}

#[tauri::command]
pub fn save_provider_config(
    state: State<'_, CoworkBackend>,
    config: LlmProviderConfig,
) -> Result<SettingsSnapshot, String> {
    state.save_provider_config(config).map_err(format_error)
}

#[tauri::command]
pub fn save_mcp_servers_config(
    state: State<'_, CoworkBackend>,
    config_json: String,
) -> Result<SettingsSnapshot, String> {
    state
        .save_mcp_servers_config(&config_json)
        .map_err(format_error)
}

#[tauri::command]
pub fn reload_provider_config(state: State<'_, CoworkBackend>) -> Result<SettingsSnapshot, String> {
    state.reload_provider_config().map_err(format_error)
}

#[tauri::command]
pub fn reload_mcp_servers_config(
    state: State<'_, CoworkBackend>,
) -> Result<SettingsSnapshot, String> {
    state.reload_mcp_servers_config().map_err(format_error)
}

fn format_error(error: anyhow::Error) -> String {
    format!("{error:#}")
}
