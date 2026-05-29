use anyhow::Context;
use rmcp::{
    RoleClient, ServiceExt, model::Tool, service::RunningService, transport::TokioChildProcess,
};
use std::collections::BTreeMap;

use crate::agent::agent_tool::AgentTool;

use super::mcp_tool_adapter::McpToolAdapter;

struct McpServerHandle {
    client: RunningService<RoleClient, ()>,
    tools: Vec<Tool>,
}

pub struct McpClientManager {
    servers: Vec<McpServerHandle>,
}

impl McpClientManager {
    pub fn new() -> Self {
        Self {
            servers: Vec::new(),
        }
    }

    /// Connect to a local MCP server via stdio with environment overrides.
    pub async fn connect_stdio_with_env(
        &mut self,
        name: &str,
        command: &str,
        args: &[String],
        env: &BTreeMap<String, String>,
    ) -> anyhow::Result<()> {
        use rmcp::transport::ConfigureCommandExt;
        use tokio::process::Command;

        let mut cmd = Command::new(command);
        cmd.args(args);
        cmd.envs(env);

        let client = ()
            .serve(TokioChildProcess::new(cmd.configure(|c| {
                c.stderr(std::process::Stdio::inherit());
            }))?)
            .await
            .context(format!("failed to connect to MCP server '{name}'"))?;

        let tools = client
            .list_all_tools()
            .await
            .context(format!("failed to list tools from MCP server '{name}'"))?;

        tracing::info!(
            "Connected to MCP server '{name}' via stdio (command: {command}), discovered {} tools",
            tools.len()
        );

        self.servers.push(McpServerHandle { client, tools });

        Ok(())
    }

    /// Connect to a remote MCP server via HTTP (Streamable HTTP transport).
    pub async fn connect_http(&mut self, name: &str, url: &str) -> anyhow::Result<()> {
        use rmcp::transport::streamable_http_client::StreamableHttpClientTransport;

        let transport = StreamableHttpClientTransport::from_uri(url.to_owned());

        let client = ()
            .serve(transport)
            .await
            .context(format!("failed to connect to MCP server '{name}' at {url}"))?;

        let tools = client
            .list_all_tools()
            .await
            .context(format!("failed to list tools from MCP server '{name}'"))?;

        tracing::info!(
            "Connected to MCP server '{name}' via HTTP ({url}), discovered {} tools",
            tools.len()
        );

        self.servers.push(McpServerHandle { client, tools });

        Ok(())
    }

    /// Get all discovered tools as `AgentTool` implementations.
    pub fn agent_tools(&self) -> Vec<Box<dyn AgentTool + Send + Sync>> {
        let mut tools: Vec<Box<dyn AgentTool + Send + Sync>> = Vec::new();

        for server in &self.servers {
            for tool in &server.tools {
                let name = tool.name.to_string();
                let description = tool
                    .description
                    .as_ref()
                    .map(|d| d.to_string())
                    .unwrap_or_default();
                let parameters_json =
                    serde_json::to_string(&*tool.input_schema).unwrap_or_else(|_| "{}".to_owned());

                tools.push(Box::new(McpToolAdapter::new(
                    name,
                    description,
                    parameters_json,
                    server.client.peer().clone(),
                )));
            }
        }

        tools
    }

    /// Get the number of connected servers.
    pub fn server_count(&self) -> usize {
        self.servers.len()
    }

    /// Get the total number of discovered tools.
    pub fn tool_count(&self) -> usize {
        self.servers.iter().map(|s| s.tools.len()).sum()
    }
}
