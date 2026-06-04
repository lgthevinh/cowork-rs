use anyhow::Context;
use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestUserMessageArgs,
    },
};
use std::sync::{Arc, RwLock};

use crate::log::ilog::ILog;
use crate::record::record_file::RecordFile;
use crate::storage::llm_provider_config::{LLM_PROVIDER_CONFIG_PATH, LlmProviderConfig};
use crate::storage::mcp_servers_config::{
    MCP_SERVERS_CONFIG_PATH, McpServersConfig, load_or_init_mcp_servers_config,
};

use super::agent::{Agent, AgentResponse, AgentStreamCallback};
use super::agent_preset::DEFAULT_AGENT_PRESET;
use super::agent_tool::AgentTool;
use super::tool::builtin::time_tool::GetCurrentTimeTool;
use super::tool::mcp::mcp_client::McpClientManager;

const TAG: &str = "AgentOrchestrator";

pub struct AgentOrchestrator {
    agents: Vec<Agent>,
    mcp_state: RwLock<McpRuntimeState>,
}

struct McpRuntimeState {
    configured_server_count: usize,
    mcp_server_count: usize,
    mcp_tool_count: usize,
    _mcp_manager: Option<McpClientManager>,
    _mcp_runtime: Option<tokio::runtime::Runtime>,
}

impl Default for McpRuntimeState {
    fn default() -> Self {
        Self {
            configured_server_count: 0,
            mcp_server_count: 0,
            mcp_tool_count: 0,
            _mcp_manager: None,
            _mcp_runtime: None,
        }
    }
}

impl AgentOrchestrator {
    pub fn new(agents: Vec<Agent>) -> Self {
        ILog::d(TAG, &format!("new: agents={}", agents.len()));

        Self {
            agents,
            mcp_state: RwLock::new(McpRuntimeState::default()),
        }
    }

    pub fn agents(&self) -> &[Agent] {
        &self.agents
    }

    pub fn default_agent(&self) -> Option<&Agent> {
        self.agents.first()
    }

    pub fn mcp_server_count(&self) -> usize {
        self.mcp_state
            .read()
            .map(|state| state.mcp_server_count)
            .unwrap_or(0)
    }

    pub fn mcp_tool_count(&self) -> usize {
        self.mcp_state
            .read()
            .map(|state| state.mcp_tool_count)
            .unwrap_or(0)
    }

    pub fn mcp_configured_server_count(&self) -> usize {
        self.mcp_state
            .read()
            .map(|state| state.configured_server_count)
            .unwrap_or(0)
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

    pub fn reload_mcp_servers_config(&self) -> anyhow::Result<McpServersConfig> {
        ILog::d(TAG, "reload_mcp_servers_config: start");

        let config = load_or_init_mcp_servers_config()?;
        let configured_server_count = config.configured_server_count();
        let (mcp_state, mcp_tools) = init_mcp_tools_from_config(config.clone())?;
        let mut tools = builtin_agent_tools();
        tools.extend(mcp_tools);

        for agent in &self.agents {
            agent.set_tools(tools.clone())?;
        }

        *self
            .mcp_state
            .write()
            .map_err(|_| anyhow::anyhow!("mcp runtime state lock is poisoned"))? = mcp_state;

        ILog::d(
            TAG,
            &format!(
                "reload_mcp_servers_config: completed configured_servers={} connected_servers={} tools={}",
                configured_server_count,
                self.mcp_server_count(),
                self.mcp_tool_count()
            ),
        );

        Ok(config)
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
                &format!("chat_completion_stream_response: failed error={error:#}"),
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
    let mut tools = builtin_agent_tools();
    ILog::d(TAG, &format!("init: builtin_tools={}", tools.len()));

    // Connect to MCP servers if configured.
    let mcp_state = match init_mcp_tools() {
        Ok((state, mcp_tools)) => {
            let server_count = state.mcp_server_count;
            let tool_count = state.mcp_tool_count;
            tools.extend(mcp_tools);
            ILog::d(
                TAG,
                &format!("init: mcp initialized servers={server_count} tools={tool_count}"),
            );
            state
        }
        Err(e) => {
            ILog::d(TAG, &format!("init: mcp initialization failed error={e}"));
            ILog::w(
                TAG,
                &format!("init: failed to initialize MCP tools error={e}"),
            );
            McpRuntimeState::default()
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

    let orchestrator = AgentOrchestrator::new(vec![default_agent]);
    *orchestrator
        .mcp_state
        .write()
        .map_err(|_| anyhow::anyhow!("mcp runtime state lock is poisoned"))? = mcp_state;
    orchestrator.load_provider_config()?;

    ILog::d(
        TAG,
        &format!(
            "init: completed agents={} mcp_servers={} mcp_tools={}",
            orchestrator.agents.len(),
            orchestrator.mcp_server_count(),
            orchestrator.mcp_tool_count()
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

fn builtin_agent_tools() -> Vec<Arc<dyn AgentTool + Send + Sync>> {
    vec![Arc::new(GetCurrentTimeTool)]
}

/// Initialize MCP tools from local MCP server configuration.
///
/// Loads `preference/mcp-servers.json`, importing legacy `mcp-servers.json`
/// only when the preference file does not exist yet.
fn init_mcp_tools() -> anyhow::Result<(McpRuntimeState, Vec<Arc<dyn AgentTool + Send + Sync>>)> {
    let config = load_or_init_mcp_servers_config()?;
    init_mcp_tools_from_config(config)
}

fn init_mcp_tools_from_config(
    config: McpServersConfig,
) -> anyhow::Result<(McpRuntimeState, Vec<Arc<dyn AgentTool + Send + Sync>>)> {
    ILog::d(TAG, "init_mcp_tools: start");
    let configured_server_count = config.configured_server_count();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("cowork-mcp")
        .build()
        .context("failed to create tokio runtime for MCP init")?;
    ILog::d(TAG, "init_mcp_tools: tokio runtime created");

    let (manager, server_count, tool_count) = rt.block_on(async {
        let mut manager = McpClientManager::new();

        connect_mcp_servers_from_config(&mut manager, config).await;

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
    let tools = manager.agent_tools();

    ILog::d(
        TAG,
        &format!("init_mcp_tools: completed servers={server_count} tools={tool_count}"),
    );

    Ok((
        McpRuntimeState {
            configured_server_count,
            mcp_server_count: server_count,
            mcp_tool_count: tool_count,
            _mcp_manager: Some(manager),
            _mcp_runtime: Some(rt),
        },
        tools,
    ))
}

async fn connect_mcp_servers_from_config(manager: &mut McpClientManager, config: McpServersConfig) {
    ILog::d(
        TAG,
        &format!("connect_mcp_servers_from_config: path={MCP_SERVERS_CONFIG_PATH}"),
    );

    ILog::d(
        TAG,
        &format!(
            "connect_mcp_servers_from_config: loaded server_count={}",
            config.mcp_servers.len()
        ),
    );

    for (name, server) in config.mcp_servers {
        if server.disabled {
            ILog::d(
                TAG,
                &format!("connect_mcp_servers_from_config: skipping disabled server={name}"),
            );
            continue;
        }

        if let Some(command) = server.command {
            let args = server.args.unwrap_or_default();
            let env = server.env.unwrap_or_default();
            ILog::d(
                TAG,
                &format!(
                    "connect_mcp_servers_from_config: connecting stdio server={name} command={command} args={} env_keys={}",
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
                        "connect_mcp_servers_from_config: stdio connect failed server={name} error={error}"
                    ),
                );
                ILog::w(
                    TAG,
                    &format!(
                        "connect_mcp_servers_from_config: failed to connect stdio server={name} path={MCP_SERVERS_CONFIG_PATH} error={error}"
                    ),
                );
            } else {
                ILog::d(
                    TAG,
                    &format!("connect_mcp_servers_from_config: stdio connected server={name}"),
                );
            }

            continue;
        }

        if let Some(url) = server.url {
            ILog::d(
                TAG,
                &format!(
                    "connect_mcp_servers_from_config: connecting http server={name} url={url}"
                ),
            );
            if let Err(error) = manager.connect_http(&name, &url).await {
                ILog::d(
                    TAG,
                    &format!(
                        "connect_mcp_servers_from_config: http connect failed server={name} error={error}"
                    ),
                );
                ILog::w(
                    TAG,
                    &format!(
                        "connect_mcp_servers_from_config: failed to connect http server={name} path={MCP_SERVERS_CONFIG_PATH} error={error}"
                    ),
                );
            } else {
                ILog::d(
                    TAG,
                    &format!("connect_mcp_servers_from_config: http connected server={name}"),
                );
            }

            continue;
        }

        ILog::d(
            TAG,
            &format!("connect_mcp_servers_from_config: skipping invalid server={name}"),
        );
        ILog::w(
            TAG,
            &format!(
                "connect_mcp_servers_from_config: skipping invalid server={name} path={MCP_SERVERS_CONFIG_PATH} expected command or url"
            ),
        );
    }
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
