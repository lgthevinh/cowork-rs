use super::agent_tool::AgentTool;
use async_openai::{Client, config::OpenAIConfig};

pub struct Agent {
    id: String,
    name: String,
    system_instruction: String,
    model: String,
    llm_client: Client<OpenAIConfig>,
    tools: Vec<Box<dyn AgentTool + Send + Sync>>,
}

impl Agent {
    pub fn new(
        id: String,
        name: String,
        system_instruction: String,
        model: String,
        llm_client: Client<OpenAIConfig>,
        tools: Vec<Box<dyn AgentTool + Send + Sync>>,
    ) -> Self {
        Self {
            id,
            name,
            system_instruction,
            model,
            llm_client,
            tools,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn system_instruction(&self) -> &str {
        &self.system_instruction
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn llm_client(&self) -> &Client<OpenAIConfig> {
        &self.llm_client
    }
}
