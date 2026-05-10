pub trait AgentTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_json(&self) -> &str;
    fn execute(&self, json_input: &str) -> anyhow::Result<String>;
}
