import { Channel, invoke } from '@tauri-apps/api/core';
import type {
  BootstrapResponse,
  ChatStreamEvent,
  LoadedSession,
  LlmProviderConfig,
  SettingsSnapshot,
} from '../types';

export function bootstrapApp() {
  return invoke<BootstrapResponse>('bootstrap_app');
}

export function loadSession(sessionId: string) {
  return invoke<LoadedSession>('load_session', { sessionId });
}

export function newSession() {
  return invoke<LoadedSession>('new_session');
}

export function deleteSession(sessionId: string) {
  return invoke<LoadedSession>('delete_session', { sessionId });
}

export function sendMessage(
  sessionId: string,
  content: string,
  onEvent: (event: ChatStreamEvent) => void,
) {
  const events = new Channel<ChatStreamEvent>();
  events.onmessage = onEvent;

  return invoke<LoadedSession>('send_message', {
    request: { sessionId, content },
    events,
  });
}

export function getSettings() {
  return invoke<SettingsSnapshot>('get_settings');
}

export function saveProviderConfig(config: LlmProviderConfig) {
  return invoke<SettingsSnapshot>('save_provider_config', { config });
}

export function saveMcpServersConfig(configJson: string) {
  return invoke<SettingsSnapshot>('save_mcp_servers_config', { configJson });
}

export function reloadProviderConfig() {
  return invoke<SettingsSnapshot>('reload_provider_config');
}

export function reloadMcpServersConfig() {
  return invoke<SettingsSnapshot>('reload_mcp_servers_config');
}
