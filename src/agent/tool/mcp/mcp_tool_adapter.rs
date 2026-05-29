use rmcp::{
    Peer, RoleClient,
    model::{CallToolRequestParams, Content},
};

use crate::agent::agent_tool::AgentTool;

pub struct McpToolAdapter {
    name: String,
    description: String,
    parameters_json: String,
    peer: Peer<RoleClient>,
}

impl McpToolAdapter {
    pub fn new(
        name: String,
        description: String,
        parameters_json: String,
        peer: Peer<RoleClient>,
    ) -> Self {
        Self {
            name,
            description,
            parameters_json,
            peer,
        }
    }
}

#[async_trait::async_trait]
impl AgentTool for McpToolAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_json(&self) -> &str {
        &self.parameters_json
    }

    async fn execute(&self, json_input: &str) -> anyhow::Result<String> {
        let mut params = CallToolRequestParams::new(self.name.clone());

        if !(json_input.trim().is_empty() || json_input.trim() == "{}") {
            let value: serde_json::Value = serde_json::from_str(json_input)?;
            if let Some(obj) = value.as_object().cloned() {
                params = params.with_arguments(obj);
            }
        }

        let result = self
            .peer
            .call_tool(params)
            .await
            .map_err(|e| anyhow::anyhow!("MCP tool call failed: {e}"))?;

        if result.is_error == Some(true) {
            let text = extract_text(&result.content);
            return Err(anyhow::anyhow!("MCP tool error: {text}"));
        }

        Ok(extract_text(&result.content))
    }
}

fn extract_text(content: &[Content]) -> String {
    content
        .iter()
        .filter_map(|item| item.as_text().map(|t| t.text.as_str()))
        .collect::<Vec<_>>()
        .join("\n")
}
