use super::agent_tool::AgentTool;
use async_openai::{Client, config::OpenAIConfig};

pub struct Agent {
    pub id: String,
    pub name: String,
    pub system_instruction: String,
    pub llm_client: Client<OpenAIConfig>,
    pub tools: Vec<Box<dyn AgentTool>>,
}

impl Agent {
    pub fn new(
        id: String,
        name: String,
        system_instruction: String,
        llm_client: Client<OpenAIConfig>,
        tools: Vec<Box<dyn AgentTool>>,
    ) -> Self {
        Self {
            id,
            name,
            system_instruction,
            llm_client,
            tools,
        }
    }
}
