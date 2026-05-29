#[async_trait::async_trait]
pub trait AgentTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_json(&self) -> &str;
    async fn execute(&self, json_input: &str) -> anyhow::Result<String>;
}
