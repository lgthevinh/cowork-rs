pub struct AgentPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub system_instruction: &'static str,
    pub model: &'static str,
}

pub const DEFAULT_AGENT_PRESET: AgentPreset = AgentPreset {
    id: "default",
    name: "Cowork Agent",
    system_instruction: "You are a helpful cowork agent.",
    model: "gpt-4.1-mini",
};
