export interface SessionRecord {
  sessionId: string;
  title: string;
  model: string;
  createdAt: number;
  updatedAt: number;
  temperature: number;
  topP: number;
  topK: number;
}

export interface MessageRecord {
  messageId: string;
  sessionId: string;
  sequence: number;
  role: number;
  content: string;
  createdAt: number;
}

export interface LlmModelConfig {
  id: string;
  displayName: string;
}

export interface LlmProviderConfig {
  providerName: string;
  baseUrl: string;
  apiKey: string;
  defaultModel: string;
  models: LlmModelConfig[];
}

export interface SettingsSnapshot {
  providerConfig: LlmProviderConfig;
  mcpConfigJson: string;
  mcpConfiguredServerCount: number;
  mcpServerCount: number;
  mcpToolCount: number;
}

export interface LoadedSession {
  activeSession: SessionRecord;
  sessions: SessionRecord[];
  messages: MessageRecord[];
}

export interface BootstrapResponse extends LoadedSession {
  dbPath: string;
  settings: SettingsSnapshot;
}

export type ChatStreamEvent =
  | { type: 'token'; token: string }
  | { type: 'completed'; full_text?: string; fullText?: string }
  | { type: 'error'; error: string }
  | { type: 'usage'; summary: string }
  | { type: 'toolStarted'; tool_name?: string; toolName?: string; arguments: string }
  | { type: 'toolCompleted'; tool_name?: string; toolName?: string; result: string };

export type UiRole = 'assistant' | 'system' | 'tool' | 'user';

export interface UiMessage {
  id: string;
  role: UiRole;
  author: string;
  body: string;
  toolName?: string;
  toolArguments?: string;
  toolResult?: string;
  isToolOpen?: boolean;
}
