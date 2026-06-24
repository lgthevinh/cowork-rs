import { FormEvent, useEffect, useMemo, useRef, useState } from 'react';
import {
  bootstrapApp,
  deleteSession,
  loadSession,
  newSession,
  reloadMcpServersConfig,
  reloadProviderConfig,
  saveMcpServersConfig,
  saveProviderConfig,
  sendMessage,
} from './lib/api';
import type {
  BootstrapResponse,
  ChatStreamEvent,
  LlmProviderConfig,
  LoadedSession,
  MessageRecord,
  SessionRecord,
  SettingsSnapshot,
  UiMessage,
} from './types';

const ROLE_SYSTEM = 0;
const ROLE_ASSISTANT = 1;
const ROLE_USER = 2;
const ROLE_TOOL = 3;

const EMPTY_AGENT_RESPONSE = 'The agent returned an empty response.';

type SettingsTab = 'agent' | 'tools' | 'storage';

interface Toast {
  id: number;
  level: 'error' | 'info' | 'success';
  message: string;
}

export function App() {
  const [activeSession, setActiveSession] = useState<SessionRecord | null>(null);
  const [sessions, setSessions] = useState<SessionRecord[]>([]);
  const [messages, setMessages] = useState<UiMessage[]>([]);
  const [settings, setSettings] = useState<SettingsSnapshot | null>(null);
  const [dbPath, setDbPath] = useState('');
  const [draft, setDraft] = useState('');
  const [isWaiting, setIsWaiting] = useState(false);
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [settingsTab, setSettingsTab] = useState<SettingsTab>('agent');
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [sidebarWidth, setSidebarWidth] = useState(286);
  const [isResizing, setIsResizing] = useState(false);
  const streamMessageId = useRef<string | null>(null);
  const hadToolThisTurn = useRef(false);
  const transcriptRef = useRef<HTMLDivElement | null>(null);
  const nextToastId = useRef(1);

  useEffect(() => {
    bootstrapApp()
      .then((response) => {
        applyBootstrap(response);
      })
      .catch((error) => {
        pushToast('error', `Failed to start Cowork RS: ${formatError(error)}`);
      });
  }, []);

  useEffect(() => {
    transcriptRef.current?.scrollTo({
      top: transcriptRef.current.scrollHeight,
      behavior: 'smooth',
    });
  }, [messages.length, isWaiting]);

  useEffect(() => {
    if (!isResizing) return;

    const onMove = (event: MouseEvent) => {
      setSidebarWidth(Math.min(420, Math.max(220, event.clientX)));
    };
    const onUp = () => setIsResizing(false);

    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
    return () => {
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
    };
  }, [isResizing]);

  const activeTitle = activeSession?.title ?? 'Cowork RS';
  const activeModel = activeSession?.model ?? 'unknown';

  function applyBootstrap(response: BootstrapResponse) {
    setDbPath(response.dbPath);
    setSettings(response.settings);
    applyLoadedSession(response);
  }

  function applyLoadedSession(response: LoadedSession) {
    setActiveSession(response.activeSession);
    setSessions(response.sessions);
    setMessages(messagesFromRecords(response.messages));
  }

  function applyLoadedSessionWithoutTranscript(response: LoadedSession) {
    setActiveSession(response.activeSession);
    setSessions(response.sessions);
  }

  function pushToast(level: Toast['level'], message: string) {
    const toast = { id: nextToastId.current++, level, message };
    setToasts((current) => [...current.slice(-3), toast]);
    window.setTimeout(() => {
      setToasts((current) => current.filter((item) => item.id !== toast.id));
    }, 4200);
  }

  async function selectSession(sessionId: string) {
    if (isWaiting || activeSession?.sessionId === sessionId) return;

    try {
      applyLoadedSession(await loadSession(sessionId));
    } catch (error) {
      pushToast('error', `Failed to load session: ${formatError(error)}`);
    }
  }

  async function createSession() {
    if (isWaiting) return;

    try {
      applyLoadedSession(await newSession());
    } catch (error) {
      pushToast('error', `Failed to create session: ${formatError(error)}`);
    }
  }

  async function removeSession(sessionId: string) {
    if (isWaiting) return;

    try {
      applyLoadedSession(await deleteSession(sessionId));
      pushToast('success', 'Session deleted');
    } catch (error) {
      pushToast('error', `Failed to delete session: ${formatError(error)}`);
    }
  }

  async function submitCurrentMessage() {
    const content = draft.trim();
    if (!content || !activeSession || isWaiting) return;

    const now = Date.now();
    const userMessage: UiMessage = {
      id: `local-user-${now}`,
      role: 'user',
      author: 'You',
      body: content,
    };
    const assistantMessage: UiMessage = {
      id: `local-assistant-${now}`,
      role: 'assistant',
      author: 'Assistant',
      body: '',
    };

    streamMessageId.current = assistantMessage.id;
    hadToolThisTurn.current = false;
    setDraft('');
    setIsWaiting(true);
    setMessages((current) => [...current, userMessage, assistantMessage]);
    if (activeSession.title === 'New session') {
      setActiveSession({ ...activeSession, title: titleFromMessage(content) });
    }

    try {
      const loaded = await sendMessage(activeSession.sessionId, content, handleStreamEvent);
      applyLoadedSessionWithoutTranscript(loaded);
    } catch (error) {
      removeStreamingAssistant();
      appendSystemMessage(`Agent request failed: ${formatError(error)}`);
      pushToast('error', `Agent request failed: ${formatError(error)}`);
    } finally {
      streamMessageId.current = null;
      hadToolThisTurn.current = false;
      setIsWaiting(false);
    }
  }

  function submitMessage(event: FormEvent) {
    event.preventDefault();
    void submitCurrentMessage();
  }

  function handleStreamEvent(event: ChatStreamEvent) {
    switch (event.type) {
      case 'token':
        appendToStreamingAssistant(event.token);
        break;
      case 'completed': {
        const fullText = event.fullText ?? event.full_text ?? '';
        if (!fullText.trim() && hadToolThisTurn.current) {
          removeStreamingAssistant();
        } else {
          setStreamingAssistantBody(fullText.trim() ? fullText : EMPTY_AGENT_RESPONSE);
        }
        break;
      }
      case 'error':
        removeStreamingAssistant();
        appendSystemMessage(`Agent request failed: ${event.error}`);
        break;
      case 'toolStarted': {
        const toolName = event.toolName ?? event.tool_name ?? 'Unknown tool';
        hadToolThisTurn.current = true;
        replaceStreamingWithTool(toolName, event.arguments);
        break;
      }
      case 'toolCompleted':
        completeLatestTool(event.toolName ?? event.tool_name ?? 'Unknown tool', event.result);
        break;
      case 'usage':
        break;
    }
  }

  function appendToStreamingAssistant(token: string) {
    const id = streamMessageId.current;
    if (!id) return;

    setMessages((current) =>
      current.map((message) =>
        message.id === id ? { ...message, body: `${message.body}${token}` } : message,
      ),
    );
  }

  function setStreamingAssistantBody(body: string) {
    const id = streamMessageId.current;
    if (!id) return;

    setMessages((current) =>
      current.map((message) => (message.id === id ? { ...message, body } : message)),
    );
  }

  function removeStreamingAssistant() {
    const id = streamMessageId.current;
    if (!id) return;

    setMessages((current) =>
      current.filter((message) => !(message.id === id && message.role === 'assistant')),
    );
  }

  function replaceStreamingWithTool(toolName: string, toolArguments: string) {
    const oldId = streamMessageId.current;
    const newId = `local-assistant-${Date.now()}-${Math.random().toString(16).slice(2)}`;
    streamMessageId.current = newId;

    setMessages((current) => {
      const next = current.map((message) =>
        message.id === oldId
          ? {
              ...message,
              role: 'tool' as const,
              author: toolName,
              body: `\`${toolName}\` is running`,
              toolName,
              toolArguments,
              isToolOpen: false,
            }
          : message,
      );

      return [
        ...next,
        { id: newId, role: 'assistant', author: 'Assistant', body: '' },
      ];
    });
  }

  function completeLatestTool(toolName: string, toolResult: string) {
    setMessages((current) => {
      const next = [...current];
      for (let index = next.length - 1; index >= 0; index -= 1) {
        if (next[index].role === 'tool') {
          next[index] = {
            ...next[index],
            body: `\`${toolName}\` completed (${toolResult.length} chars)`,
            toolName,
            toolResult,
          };
          break;
        }
      }
      return next;
    });
  }

  function appendSystemMessage(body: string) {
    setMessages((current) => [
      ...current,
      { id: `system-${Date.now()}`, role: 'system', author: 'System', body },
    ]);
  }

  async function saveProvider(config: LlmProviderConfig) {
    try {
      const snapshot = await saveProviderConfig(config);
      setSettings(snapshot);
      pushToast('success', 'Provider settings saved');
    } catch (error) {
      pushToast('error', `Failed to save provider settings: ${formatError(error)}`);
      throw error;
    }
  }

  async function saveMcp(configJson: string) {
    try {
      const snapshot = await saveMcpServersConfig(configJson);
      setSettings(snapshot);
      pushToast('success', 'Tools settings saved');
    } catch (error) {
      pushToast('error', `Failed to save Tools settings: ${formatError(error)}`);
      throw error;
    }
  }

  async function reloadProvider() {
    try {
      setSettings(await reloadProviderConfig());
      pushToast('info', 'Provider settings reloaded');
    } catch (error) {
      pushToast('error', `Failed to reload provider settings: ${formatError(error)}`);
    }
  }

  async function reloadMcp() {
    try {
      setSettings(await reloadMcpServersConfig());
      pushToast('info', 'Tools settings reloaded');
    } catch (error) {
      pushToast('error', `Failed to reload Tools settings: ${formatError(error)}`);
    }
  }

  return (
    <main className="app-shell">
      <aside className="sidebar" style={{ width: sidebarWidth }}>
        <div className="sidebar-header">
          <div>
            <div className="brand">Cowork RS</div>
            <div className="brand-subtitle">Local agent workspace</div>
          </div>
          <button className="icon-button" type="button" onClick={createSession} title="New session">
            +
          </button>
        </div>
        <SessionList
          activeSessionId={activeSession?.sessionId ?? ''}
          sessions={sessions}
          onSelect={selectSession}
          onDelete={removeSession}
        />
      </aside>

      <div
        className="resize-handle"
        role="separator"
        aria-orientation="vertical"
        onMouseDown={() => setIsResizing(true)}
      />

      <section className="chat-column">
        <header className="top-bar">
          <div className="top-title">
            <h1>{activeTitle}</h1>
            <span>{activeModel}</span>
          </div>
          <div className={`status ${isWaiting ? 'status-busy' : ''}`}>
            {isWaiting ? 'Responding' : 'Ready'}
          </div>
          <button className="secondary-button" type="button" onClick={() => setIsSettingsOpen(true)}>
            Settings
          </button>
        </header>

        <div className="transcript" ref={transcriptRef}>
          {messages.length === 0 ? (
            <div className="empty-state">Start the session with a message.</div>
          ) : (
            messages.map((message) => (
              <MessageBubble
                key={message.id}
                message={message}
                onToggleTool={() =>
                  setMessages((current) =>
                    current.map((item) =>
                      item.id === message.id ? { ...item, isToolOpen: !item.isToolOpen } : item,
                    ),
                  )
                }
              />
            ))
          )}
        </div>

        <form className="composer" onSubmit={submitMessage}>
          <textarea
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Enter' && !event.shiftKey) {
                event.preventDefault();
                void submitCurrentMessage();
              }
            }}
            placeholder="Message the agent"
            rows={2}
            disabled={!activeSession || isWaiting}
          />
          <button className="primary-button" type="submit" disabled={!draft.trim() || isWaiting}>
            Send
          </button>
        </form>
      </section>

      {isSettingsOpen && settings ? (
        <SettingsDialog
          activeTab={settingsTab}
          dbPath={dbPath}
          messageCount={messages.length}
          settings={settings}
          sessionModel={activeModel}
          isWaiting={isWaiting}
          onClose={() => setIsSettingsOpen(false)}
          onTabChange={setSettingsTab}
          onProviderSave={saveProvider}
          onProviderReload={reloadProvider}
          onMcpSave={saveMcp}
          onMcpReload={reloadMcp}
        />
      ) : null}

      <div className="toast-layer">
        {toasts.map((toast) => (
          <div className={`toast toast-${toast.level}`} key={toast.id}>
            {toast.message}
          </div>
        ))}
      </div>
    </main>
  );
}

function SessionList({
  activeSessionId,
  sessions,
  onSelect,
  onDelete,
}: {
  activeSessionId: string;
  sessions: SessionRecord[];
  onSelect: (sessionId: string) => void;
  onDelete: (sessionId: string) => void;
}) {
  return (
    <div className="session-list">
      {sessions.map((session) => (
        <button
          className={`session-row ${session.sessionId === activeSessionId ? 'is-active' : ''}`}
          key={session.sessionId}
          type="button"
          onClick={() => onSelect(session.sessionId)}
        >
          <span>
            <strong>{session.title}</strong>
            <small>{relativeDate(session.updatedAt)}</small>
          </span>
          <span
            className="delete-session"
            role="button"
            tabIndex={0}
            onClick={(event) => {
              event.stopPropagation();
              onDelete(session.sessionId);
            }}
            onKeyDown={(event) => {
              if (event.key === 'Enter' || event.key === ' ') {
                event.preventDefault();
                event.stopPropagation();
                onDelete(session.sessionId);
              }
            }}
          >
            ×
          </span>
        </button>
      ))}
    </div>
  );
}

function MessageBubble({
  message,
  onToggleTool,
}: {
  message: UiMessage;
  onToggleTool: () => void;
}) {
  const isTool = message.role === 'tool';

  return (
    <article className={`message message-${message.role}`}>
      <div className="message-author">{message.author}</div>
      <div className="message-body">{message.body || <span className="typing">Thinking</span>}</div>
      {isTool ? (
        <button className="detail-button" type="button" onClick={onToggleTool}>
          {message.isToolOpen ? 'Hide detail' : 'Show detail'}
        </button>
      ) : null}
      {isTool && message.isToolOpen ? (
        <div className="tool-detail">
          <label>Arguments</label>
          <pre>{message.toolArguments || '(none)'}</pre>
          <label>Result</label>
          <pre>{message.toolResult || 'Running'}</pre>
        </div>
      ) : null}
    </article>
  );
}

function SettingsDialog({
  activeTab,
  dbPath,
  messageCount,
  settings,
  sessionModel,
  isWaiting,
  onClose,
  onTabChange,
  onProviderSave,
  onProviderReload,
  onMcpSave,
  onMcpReload,
}: {
  activeTab: SettingsTab;
  dbPath: string;
  messageCount: number;
  settings: SettingsSnapshot;
  sessionModel: string;
  isWaiting: boolean;
  onClose: () => void;
  onTabChange: (tab: SettingsTab) => void;
  onProviderSave: (config: LlmProviderConfig) => Promise<void>;
  onProviderReload: () => Promise<void>;
  onMcpSave: (configJson: string) => Promise<void>;
  onMcpReload: () => Promise<void>;
}) {
  return (
    <div className="settings-backdrop">
      <section className="settings-dialog">
        <aside className="settings-nav">
          <div className="settings-title">Settings</div>
          <button className={activeTab === 'agent' ? 'is-active' : ''} onClick={() => onTabChange('agent')}>
            Agent
          </button>
          <button className={activeTab === 'tools' ? 'is-active' : ''} onClick={() => onTabChange('tools')}>
            Tools
          </button>
          <button className={activeTab === 'storage' ? 'is-active' : ''} onClick={() => onTabChange('storage')}>
            Storage
          </button>
        </aside>
        <div className="settings-content">
          <div className="settings-heading">
            <div>
              <h2>{activeTabTitle(activeTab)}</h2>
              <p>{activeTabSubtitle(activeTab)}</p>
            </div>
            <button className="icon-button" type="button" onClick={onClose} title="Close settings">
              ×
            </button>
          </div>

          {activeTab === 'agent' ? (
            <ProviderSettings
              config={settings.providerConfig}
              sessionModel={sessionModel}
              isWaiting={isWaiting}
              onSave={onProviderSave}
              onReload={onProviderReload}
            />
          ) : null}
          {activeTab === 'tools' ? (
            <ToolsSettings
              configJson={settings.mcpConfigJson}
              configured={settings.mcpConfiguredServerCount}
              connected={settings.mcpServerCount}
              toolCount={settings.mcpToolCount}
              isWaiting={isWaiting}
              onSave={onMcpSave}
              onReload={onMcpReload}
            />
          ) : null}
          {activeTab === 'storage' ? (
            <StorageSettings dbPath={dbPath} messageCount={messageCount} />
          ) : null}
        </div>
      </section>
    </div>
  );
}

function ProviderSettings({
  config,
  sessionModel,
  isWaiting,
  onSave,
  onReload,
}: {
  config: LlmProviderConfig;
  sessionModel: string;
  isWaiting: boolean;
  onSave: (config: LlmProviderConfig) => Promise<void>;
  onReload: () => Promise<void>;
}) {
  const [isChanging, setIsChanging] = useState(false);
  const [providerName, setProviderName] = useState(config.providerName);
  const [baseUrl, setBaseUrl] = useState(config.baseUrl);
  const [apiKey, setApiKey] = useState(config.apiKey);
  const [defaultModel, setDefaultModel] = useState(config.defaultModel);
  const [models, setModels] = useState(() => JSON.stringify(config.models, null, 2));
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    if (isChanging) return;
    setProviderName(config.providerName);
    setBaseUrl(config.baseUrl);
    setApiKey(config.apiKey);
    setDefaultModel(config.defaultModel);
    setModels(JSON.stringify(config.models, null, 2));
  }, [config, isChanging]);

  const apiKeyStatus = apiKey.trim() ? 'Saved in preference file' : 'Using environment fallback';

  if (!isChanging) {
    return (
      <div className="settings-panel">
        <Detail label="Provider" value={config.providerName} />
        <Detail label="Base URL" value={config.baseUrl} />
        <Detail label="API key" value={apiKeyStatus} />
        <Detail label="Current model" value={sessionModel} />
        <Detail label="Default model" value={config.defaultModel} />
        <div className="settings-actions">
          <button className="primary-button" type="button" onClick={() => setIsChanging(true)}>
            Change
          </button>
          <button className="secondary-button" type="button" onClick={() => void onReload()} disabled={isWaiting}>
            Reload
          </button>
        </div>
      </div>
    );
  }

  async function submit(event: FormEvent) {
    event.preventDefault();
    setIsSaving(true);
    try {
      await onSave({
        providerName,
        baseUrl,
        apiKey,
        defaultModel,
        models: JSON.parse(models),
      });
      setIsChanging(false);
    } finally {
      setIsSaving(false);
    }
  }

  return (
    <form className="settings-panel form-grid" onSubmit={submit}>
      <Field label="Provider" value={providerName} onChange={setProviderName} />
      <Field label="Base URL" value={baseUrl} onChange={setBaseUrl} />
      <Field label="API key" value={apiKey} onChange={setApiKey} type="password" />
      <Field label="Default model" value={defaultModel} onChange={setDefaultModel} />
      <label className="field">
        <span>Models</span>
        <textarea value={models} onChange={(event) => setModels(event.target.value)} rows={8} />
      </label>
      <div className="settings-actions">
        <button className="primary-button" type="submit" disabled={isSaving || isWaiting}>
          Save changes
        </button>
        <button className="secondary-button" type="button" onClick={() => setIsChanging(false)} disabled={isSaving}>
          Cancel
        </button>
      </div>
    </form>
  );
}

function ToolsSettings({
  configJson,
  configured,
  connected,
  toolCount,
  isWaiting,
  onSave,
  onReload,
}: {
  configJson: string;
  configured: number;
  connected: number;
  toolCount: number;
  isWaiting: boolean;
  onSave: (configJson: string) => Promise<void>;
  onReload: () => Promise<void>;
}) {
  const [isChanging, setIsChanging] = useState(false);
  const [text, setText] = useState(configJson);
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    if (!isChanging) setText(configJson);
  }, [configJson, isChanging]);

  const summary = useMemo(
    () => `${configured} configured, ${connected} connected, ${toolCount} tools`,
    [configured, connected, toolCount],
  );

  if (!isChanging) {
    return (
      <div className="settings-panel">
        <Detail label="Tools status" value={summary} />
        <Detail label="Settings file" value="preference/mcp-servers.json" />
        <pre className="json-preview">{configJson}</pre>
        <div className="settings-actions">
          <button className="primary-button" type="button" onClick={() => setIsChanging(true)}>
            Change
          </button>
          <button className="secondary-button" type="button" onClick={() => void onReload()} disabled={isWaiting}>
            Reload
          </button>
        </div>
      </div>
    );
  }

  async function submit(event: FormEvent) {
    event.preventDefault();
    setIsSaving(true);
    try {
      await onSave(text);
      setIsChanging(false);
    } finally {
      setIsSaving(false);
    }
  }

  return (
    <form className="settings-panel form-grid" onSubmit={submit}>
      <label className="field">
        <span>MCP servers JSON</span>
        <textarea className="code-editor" value={text} onChange={(event) => setText(event.target.value)} rows={18} />
      </label>
      <div className="settings-actions">
        <button className="primary-button" type="submit" disabled={isSaving || isWaiting}>
          Save changes
        </button>
        <button className="secondary-button" type="button" onClick={() => setIsChanging(false)} disabled={isSaving}>
          Cancel
        </button>
      </div>
    </form>
  );
}

function StorageSettings({ dbPath, messageCount }: { dbPath: string; messageCount: number }) {
  return (
    <div className="settings-panel">
      <Detail label="Database" value={dbPath} />
      <Detail label="Messages in view" value={String(messageCount)} />
      <Detail label="Provider settings" value="preference/llm-provider.json" />
      <Detail label="Tools settings" value="preference/mcp-servers.json" />
    </div>
  );
}

function Field({
  label,
  value,
  onChange,
  type = 'text',
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  type?: string;
}) {
  return (
    <label className="field">
      <span>{label}</span>
      <input type={type} value={value} onChange={(event) => onChange(event.target.value)} />
    </label>
  );
}

function Detail({ label, value }: { label: string; value: string }) {
  return (
    <div className="detail-row">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function messagesFromRecords(records: MessageRecord[]): UiMessage[] {
  return records.map((record) => {
    switch (record.role) {
      case ROLE_ASSISTANT:
        return {
          id: record.messageId,
          role: 'assistant',
          author: 'Assistant',
          body: record.content,
        };
      case ROLE_USER:
        return {
          id: record.messageId,
          role: 'user',
          author: 'You',
          body: record.content,
        };
      case ROLE_TOOL:
        return {
          id: record.messageId,
          role: 'tool',
          author: 'Tool',
          body: record.content,
        };
      case ROLE_SYSTEM:
      default:
        return {
          id: record.messageId,
          role: 'system',
          author: 'System',
          body: record.content,
        };
    }
  });
}

function titleFromMessage(message: string) {
  const title = message.trim().slice(0, 48);
  return title || 'New session';
}

function relativeDate(timestamp: number) {
  if (!timestamp) return 'No activity';
  return new Intl.DateTimeFormat(undefined, {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  }).format(new Date(timestamp));
}

function activeTabTitle(tab: SettingsTab) {
  if (tab === 'agent') return 'Agent';
  if (tab === 'tools') return 'Tools';
  return 'Storage';
}

function activeTabSubtitle(tab: SettingsTab) {
  if (tab === 'agent') return 'Model provider and active runtime settings.';
  if (tab === 'tools') return 'MCP server configuration and connected tools.';
  return 'Local files used by this desktop app.';
}

function formatError(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}
