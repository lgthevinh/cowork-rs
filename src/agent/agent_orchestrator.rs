use anyhow::{Context, bail};
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

use super::agent::{Agent, AgentResponse, AgentStreamCallback};
use super::agent_preset::DEFAULT_AGENT_PRESET;
use super::agent_tool::AgentTool;
use super::tool::builtin::time_tool::GetCurrentTimeTool;
use super::tool::mcp::mcp_client::McpClientManager;

const DEFAULT_OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
const MCP_SERVERS_CONFIG_PATH: &str = "mcp-servers.json";

pub struct AgentOrchestrator {
    agents: Vec<Agent>,
    mcp_server_count: usize,
    mcp_tool_count: usize,
    _mcp_manager: Option<McpClientManager>,
    _mcp_runtime: Option<tokio::runtime::Runtime>,
}

impl AgentOrchestrator {
    pub fn new(agents: Vec<Agent>) -> Self {
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

    pub async fn chat_completion_stream_response(
        &self,
        messages: Vec<ChatCompletionRequestMessage>,
        callback: &dyn AgentStreamCallback,
    ) -> anyhow::Result<AgentResponse> {
        let agent = self
            .default_agent()
            .context("agent orchestrator has no default agent")?;

        agent.call_stream_response(messages, callback).await
    }
}

pub struct AgentRuntimeConfig {
    pub openai_api_key: String,
    pub openai_base_url: String,
}

impl AgentRuntimeConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();

        let openai_api_key: String = std::env::var("OPENAI_API_KEY")
            .context("OPENAI_API_KEY is required in the environment or .env")?;

        if openai_api_key.trim().is_empty() {
            bail!("OPENAI_API_KEY cannot be empty");
        }

        Ok(Self {
            openai_api_key,
            openai_base_url: env_or_default("OPENAI_BASE_URL", DEFAULT_OPENAI_BASE_URL),
        })
    }
}

pub fn init() -> anyhow::Result<AgentOrchestrator> {
    let config = AgentRuntimeConfig::from_env()?;
    let openai_config = OpenAIConfig::new()
        .with_api_key(config.openai_api_key)
        .with_api_base(config.openai_base_url);
    let llm_client = Client::with_config(openai_config);

    // Start with builtin tools
    let mut tools: Vec<Box<dyn AgentTool + Send + Sync>> = vec![Box::new(GetCurrentTimeTool)];

    // Connect to MCP servers if configured.
    let mut mcp_manager = None;
    let mut mcp_runtime = None;
    let (mcp_server_count, mcp_tool_count) = match init_mcp_tools() {
        Ok((manager, runtime, server_count, tool_count)) => {
            tools.extend(manager.agent_tools());
            mcp_manager = Some(manager);
            mcp_runtime = Some(runtime);
            (server_count, tool_count)
        }
        Err(e) => {
            tracing::warn!("Failed to initialize MCP tools: {e}");
            (0, 0)
        }
    };

    let default_agent = Agent::new(
        DEFAULT_AGENT_PRESET.id.to_owned(),
        DEFAULT_AGENT_PRESET.name.to_owned(),
        DEFAULT_AGENT_PRESET.system_instruction.to_owned(),
        DEFAULT_AGENT_PRESET.model.to_owned(),
        llm_client,
        tools,
    );

    let mut orchestrator = AgentOrchestrator::new(vec![default_agent]);
    orchestrator.mcp_server_count = mcp_server_count;
    orchestrator.mcp_tool_count = mcp_tool_count;
    orchestrator._mcp_manager = mcp_manager;
    orchestrator._mcp_runtime = mcp_runtime;

    Ok(orchestrator)
}

/// Initialize MCP tools from local MCP server configuration.
///
/// Loads `mcp-servers.json` when present.
fn init_mcp_tools() -> anyhow::Result<(McpClientManager, tokio::runtime::Runtime, usize, usize)> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("cowork-mcp")
        .build()
        .context("failed to create tokio runtime for MCP init")?;

    let (manager, server_count, tool_count) = rt.block_on(async {
        let mut manager = McpClientManager::new();

        connect_mcp_servers_from_file(&mut manager, MCP_SERVERS_CONFIG_PATH).await;

        let server_count = manager.server_count();
        let tool_count = manager.tool_count();

        if server_count > 0 {
            tracing::info!(
                "MCP: connected to {} server(s), discovered {} tool(s)",
                server_count,
                tool_count
            );
        }

        (manager, server_count, tool_count)
    });

    Ok((manager, rt, server_count, tool_count))
}

async fn connect_mcp_servers_from_file(manager: &mut McpClientManager, path: &str) {
    let config = match McpServersConfig::read(path) {
        Ok(Some(config)) => config,
        Ok(None) => return,
        Err(error) => {
            tracing::warn!("Failed to load MCP config from {path}: {error}");
            return;
        }
    };

    for (name, server) in config.mcp_servers {
        if server.disabled {
            tracing::debug!("Skipping disabled MCP server '{name}'");
            continue;
        }

        if let Some(command) = server.command {
            let args = server.args.unwrap_or_default();
            let env = server.env.unwrap_or_default();

            if let Err(error) = manager
                .connect_stdio_with_env(&name, &command, &args, &env)
                .await
            {
                tracing::warn!(
                    "Failed to connect to MCP stdio server '{name}' from {path}: {error}"
                );
            }

            continue;
        }

        if let Some(url) = server.url {
            if let Err(error) = manager.connect_http(&name, &url).await {
                tracing::warn!(
                    "Failed to connect to MCP HTTP server '{name}' from {path}: {error}"
                );
            }

            continue;
        }

        tracing::warn!(
            "Skipping MCP server '{name}' from {path}: expected either 'command' or 'url'"
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
        if !path.exists() {
            return Ok(None);
        }

        let contents = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let config = serde_json::from_str(&contents)
            .with_context(|| format!("failed to parse {}", path.display()))?;

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

fn env_or_default(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_owned())
}

pub fn user_message_request(content: &str) -> anyhow::Result<ChatCompletionRequestMessage> {
    Ok(ChatCompletionRequestUserMessageArgs::default()
        .content(content)
        .build()?
        .into())
}

pub fn assistant_message_request(content: &str) -> anyhow::Result<ChatCompletionRequestMessage> {
    Ok(ChatCompletionRequestAssistantMessageArgs::default()
        .content(content)
        .build()?
        .into())
}
