use anyhow::Context;
use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestUserMessageArgs,
    },
};
use serde::Deserialize;
use std::{collections::BTreeMap, fs, path::Path};

use crate::log::ilog::ILog;
use crate::record::record_file::RecordFile;
use crate::storage::llm_provider_config::{LLM_PROVIDER_CONFIG_PATH, LlmProviderConfig};

use super::agent::{Agent, AgentResponse, AgentStreamCallback};
use super::agent_preset::DEFAULT_AGENT_PRESET;
use super::agent_tool::AgentTool;
use super::tool::builtin::time_tool::GetCurrentTimeTool;
use super::tool::mcp::mcp_client::McpClientManager;

const MCP_SERVERS_CONFIG_PATH: &str = "mcp-servers.json";
const TAG: &str = "AgentOrchestrator";

pub struct AgentOrchestrator {
    agents: Vec<Agent>,
    mcp_server_count: usize,
    mcp_tool_count: usize,
    _mcp_manager: Option<McpClientManager>,
    _mcp_runtime: Option<tokio::runtime::Runtime>,
}

impl AgentOrchestrator {
    pub fn new(agents: Vec<Agent>) -> Self {
        ILog::d(TAG, &format!("new: agents={}", agents.len()));

        Self {
            agents,
            mcp_server_count: 0,
            mcp_tool_count: 0,
            _mcp_manager: None,
            _mcp_runtime: None,
        }
    }

    pub fn agents(&self) -> &[Agent] {
        &self.agents
    }

    pub fn default_agent(&self) -> Option<&Agent> {
        self.agents.first()
    }

    pub fn mcp_server_count(&self) -> usize {
        self.mcp_server_count
    }

    pub fn mcp_tool_count(&self) -> usize {
        self.mcp_tool_count
    }

    pub fn load_provider_config(&self) -> anyhow::Result<LlmProviderConfig> {
        ILog::d(TAG, "load_provider_config: start");
        let provider_config = load_provider_config()?;
        let runtime_config = AgentRuntimeConfig::from_provider_config(&provider_config)?;
        let llm_client = openai_client_from_runtime_config(&runtime_config);

        for agent in &self.agents {
            agent.set_llm_config(runtime_config.model.clone(), llm_client.clone())?;
        }

        ILog::d(
            TAG,
            &format!(
                "load_provider_config: completed provider={} base_url={} model={}",
                provider_config.provider_name, runtime_config.openai_base_url, runtime_config.model
            ),
        );

        Ok(provider_config)
    }

    pub async fn chat_completion_stream_response(
        &self,
        messages: Vec<ChatCompletionRequestMessage>,
        callback: &dyn AgentStreamCallback,
    ) -> anyhow::Result<AgentResponse> {
        ILog::d(
            TAG,
            &format!(
                "chat_completion_stream_response: messages={}",
                messages.len()
            ),
        );

        let agent = self
            .default_agent()
            .context("agent orchestrator has no default agent")?;

        let response = agent.call_stream_response(messages, callback).await;

        match &response {
            Ok(response) => ILog::d(
                TAG,
                &format!(
                    "chat_completion_stream_response: completed content_len={} tool_call_chunks={}",
                    response.content.len(),
                    response.tool_calls.len()
                ),
            ),
            Err(error) => ILog::d(
                TAG,
                &format!("chat_completion_stream_response: failed error={error}"),
            ),
        }

        response
    }
}

pub struct AgentRuntimeConfig {
    pub openai_api_key: String,
    pub openai_base_url: String,
    pub model: String,
}

impl AgentRuntimeConfig {
    pub fn from_provider_config(config: &LlmProviderConfig) -> anyhow::Result<Self> {
        ILog::d(TAG, "from_provider_config: resolving OpenAI configuration");
        config.validate_resolved()?;

        let openai_api_key = config.resolved_api_key()?;
        let openai_base_url = config.resolved_base_url();
        let model = config.resolved_default_model();

        ILog::d(
            TAG,
            &format!(
                "from_provider_config: loaded openai_base_url={openai_base_url} model={model}"
            ),
        );

        Ok(Self {
            openai_api_key,
            openai_base_url,
            model,
        })
    }
}

pub fn init() -> anyhow::Result<AgentOrchestrator> {
    ILog::d(TAG, "init: start");
    dotenvy::dotenv().ok();

    let provider_config = load_provider_config()?;
    let config = AgentRuntimeConfig::from_provider_config(&provider_config)?;
    ILog::d(TAG, "init: runtime config loaded");

    let llm_client = openai_client_from_runtime_config(&config);
    ILog::d(TAG, "init: OpenAI client created");

    // Start with builtin tools
    let mut tools: Vec<Box<dyn AgentTool + Send + Sync>> = vec![Box::new(GetCurrentTimeTool)];
    ILog::d(TAG, &format!("init: builtin_tools={}", tools.len()));

    // Connect to MCP servers if configured.
    let mut mcp_manager = None;
    let mut mcp_runtime = None;
    let (mcp_server_count, mcp_tool_count) = match init_mcp_tools() {
        Ok((manager, runtime, server_count, tool_count)) => {
            tools.extend(manager.agent_tools());
            ILog::d(
                TAG,
                &format!("init: mcp initialized servers={server_count} tools={tool_count}"),
            );
            mcp_manager = Some(manager);
            mcp_runtime = Some(runtime);
            (server_count, tool_count)
        }
        Err(e) => {
            ILog::d(TAG, &format!("init: mcp initialization failed error={e}"));
            ILog::w(
                TAG,
                &format!("init: failed to initialize MCP tools error={e}"),
            );
            (0, 0)
        }
    };
    ILog::d(TAG, &format!("init: total_tools={}", tools.len()));

    let default_agent = Agent::new(
        DEFAULT_AGENT_PRESET.id.to_owned(),
        DEFAULT_AGENT_PRESET.name.to_owned(),
        DEFAULT_AGENT_PRESET.system_instruction.to_owned(),
        config.model,
        llm_client,
        tools,
    );

    let mut orchestrator = AgentOrchestrator::new(vec![default_agent]);
    orchestrator.mcp_server_count = mcp_server_count;
    orchestrator.mcp_tool_count = mcp_tool_count;
    orchestrator._mcp_manager = mcp_manager;
    orchestrator._mcp_runtime = mcp_runtime;
    orchestrator.load_provider_config()?;

    ILog::d(
        TAG,
        &format!(
            "init: completed agents={} mcp_servers={} mcp_tools={}",
            orchestrator.agents.len(),
            orchestrator.mcp_server_count,
            orchestrator.mcp_tool_count
        ),
    );

    Ok(orchestrator)
}

fn load_provider_config() -> anyhow::Result<LlmProviderConfig> {
    ILog::d(
        TAG,
        &format!("load_provider_config_file: path={LLM_PROVIDER_CONFIG_PATH}"),
    );

    let record_file = RecordFile::open(LLM_PROVIDER_CONFIG_PATH)?;
    record_file.init(&LlmProviderConfig::default_config())?;
    let config: LlmProviderConfig = record_file.read()?;

    Ok(config)
}

fn openai_client_from_runtime_config(config: &AgentRuntimeConfig) -> Client<OpenAIConfig> {
    let openai_config = OpenAIConfig::new()
        .with_api_key(config.openai_api_key.clone())
        .with_api_base(config.openai_base_url.clone());

    Client::with_config(openai_config)
}

/// Initialize MCP tools from local MCP server configuration.
///
/// Loads `mcp-servers.json` when present.
fn init_mcp_tools() -> anyhow::Result<(McpClientManager, tokio::runtime::Runtime, usize, usize)> {
    ILog::d(TAG, "init_mcp_tools: start");
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("cowork-mcp")
        .build()
        .context("failed to create tokio runtime for MCP init")?;
    ILog::d(TAG, "init_mcp_tools: tokio runtime created");

    let (manager, server_count, tool_count) = rt.block_on(async {
        let mut manager = McpClientManager::new();

        connect_mcp_servers_from_file(&mut manager, MCP_SERVERS_CONFIG_PATH).await;

        let server_count = manager.server_count();
        let tool_count = manager.tool_count();

        if server_count > 0 {
            ILog::d(
                TAG,
                &format!("init_mcp_tools: connected servers={server_count} tools={tool_count}"),
            );
            ILog::i(
                TAG,
                &format!(
                    "init_mcp_tools: connected to {server_count} server(s), discovered {tool_count} tool(s)"
                ),
            );
        }

        (manager, server_count, tool_count)
    });

    ILog::d(
        TAG,
        &format!("init_mcp_tools: completed servers={server_count} tools={tool_count}"),
    );

    Ok((manager, rt, server_count, tool_count))
}

async fn connect_mcp_servers_from_file(manager: &mut McpClientManager, path: &str) {
    ILog::d(
        TAG,
        &format!("connect_mcp_servers_from_file: loading path={path}"),
    );

    let config = match McpServersConfig::read(path) {
        Ok(Some(config)) => {
            ILog::d(
                TAG,
                &format!(
                    "connect_mcp_servers_from_file: loaded server_count={}",
                    config.mcp_servers.len()
                ),
            );
            config
        }
        Ok(None) => {
            ILog::d(
                TAG,
                &format!("connect_mcp_servers_from_file: config not found path={path}"),
            );
            return;
        }
        Err(error) => {
            ILog::d(
                TAG,
                &format!("connect_mcp_servers_from_file: load failed path={path} error={error}"),
            );
            ILog::w(
                TAG,
                &format!("connect_mcp_servers_from_file: failed to load path={path} error={error}"),
            );
            return;
        }
    };

    for (name, server) in config.mcp_servers {
        if server.disabled {
            ILog::d(
                TAG,
                &format!("connect_mcp_servers_from_file: skipping disabled server={name}"),
            );
            continue;
        }

        if let Some(command) = server.command {
            let args = server.args.unwrap_or_default();
            let env = server.env.unwrap_or_default();
            ILog::d(
                TAG,
                &format!(
                    "connect_mcp_servers_from_file: connecting stdio server={name} command={command} args={} env_keys={}",
                    args.len(),
                    env.len()
                ),
            );

            if let Err(error) = manager
                .connect_stdio_with_env(&name, &command, &args, &env)
                .await
            {
                ILog::d(
                    TAG,
                    &format!(
                        "connect_mcp_servers_from_file: stdio connect failed server={name} error={error}"
                    ),
                );
                ILog::w(
                    TAG,
                    &format!(
                        "connect_mcp_servers_from_file: failed to connect stdio server={name} path={path} error={error}"
                    ),
                );
            } else {
                ILog::d(
                    TAG,
                    &format!("connect_mcp_servers_from_file: stdio connected server={name}"),
                );
            }

            continue;
        }

        if let Some(url) = server.url {
            ILog::d(
                TAG,
                &format!("connect_mcp_servers_from_file: connecting http server={name} url={url}"),
            );
            if let Err(error) = manager.connect_http(&name, &url).await {
                ILog::d(
                    TAG,
                    &format!(
                        "connect_mcp_servers_from_file: http connect failed server={name} error={error}"
                    ),
                );
                ILog::w(
                    TAG,
                    &format!(
                        "connect_mcp_servers_from_file: failed to connect http server={name} path={path} error={error}"
                    ),
                );
            } else {
                ILog::d(
                    TAG,
                    &format!("connect_mcp_servers_from_file: http connected server={name}"),
                );
            }

            continue;
        }

        ILog::d(
            TAG,
            &format!("connect_mcp_servers_from_file: skipping invalid server={name}"),
        );
        ILog::w(
            TAG,
            &format!(
                "connect_mcp_servers_from_file: skipping invalid server={name} path={path} expected command or url"
            ),
        );
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct McpServersConfig {
    #[serde(default, alias = "servers")]
    mcp_servers: BTreeMap<String, McpServerConfig>,
}

impl McpServersConfig {
    fn read(path: impl AsRef<Path>) -> anyhow::Result<Option<Self>> {
        let path = path.as_ref();
        ILog::d(
            TAG,
            &format!("McpServersConfig::read: path={}", path.display()),
        );

        if !path.exists() {
            ILog::d(
                TAG,
                &format!("McpServersConfig::read: missing path={}", path.display()),
            );
            return Ok(None);
        }

        let contents = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        ILog::d(
            TAG,
            &format!(
                "McpServersConfig::read: read bytes={} path={}",
                contents.len(),
                path.display()
            ),
        );

        let config = serde_json::from_str(&contents)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        ILog::d(
            TAG,
            &format!("McpServersConfig::read: parsed path={}", path.display()),
        );

        Ok(Some(config))
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct McpServerConfig {
    command: Option<String>,
    args: Option<Vec<String>>,
    url: Option<String>,
    env: Option<BTreeMap<String, String>>,
    #[serde(default)]
    disabled: bool,
}

pub fn user_message_request(content: &str) -> anyhow::Result<ChatCompletionRequestMessage> {
    ILog::d(
        TAG,
        &format!("user_message_request: content_len={}", content.len()),
    );

    Ok(ChatCompletionRequestUserMessageArgs::default()
        .content(content)
        .build()?
        .into())
}

pub fn assistant_message_request(content: &str) -> anyhow::Result<ChatCompletionRequestMessage> {
    ILog::d(
        TAG,
        &format!("assistant_message_request: content_len={}", content.len()),
    );

    Ok(ChatCompletionRequestAssistantMessageArgs::default()
        .content(content)
        .build()?
        .into())
}
