use anyhow::{Context, bail};
use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestUserMessageArgs,
    },
};

use super::agent::{Agent, AgentResponse, AgentStreamCallback};
use super::agent_preset::DEFAULT_AGENT_PRESET;

const DEFAULT_OPENAI_BASE_URL: &str = "https://api.openai.com/v1";

pub struct AgentOrchestrator {
    agents: Vec<Agent>,
}

impl AgentOrchestrator {
    pub fn new(agents: Vec<Agent>) -> Self {
        Self { agents }
    }

    pub fn agents(&self) -> &[Agent] {
        &self.agents
    }

    pub fn default_agent(&self) -> Option<&Agent> {
        self.agents.first()
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

        let openai_api_key = std::env::var("OPENAI_API_KEY")
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

    let default_agent = Agent::new(
        DEFAULT_AGENT_PRESET.id.to_owned(),
        DEFAULT_AGENT_PRESET.name.to_owned(),
        DEFAULT_AGENT_PRESET.system_instruction.to_owned(),
        DEFAULT_AGENT_PRESET.model.to_owned(),
        llm_client,
        vec![],
    );

    Ok(AgentOrchestrator::new(vec![default_agent]))
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
