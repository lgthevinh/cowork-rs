use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};

use crate::agent::agent_preset::DEFAULT_AGENT_PRESET;

pub const LLM_PROVIDER_CONFIG_PATH: &str = "preference/llm-provider.json";
pub const DEFAULT_OPENAI_BASE_URL: &str = "https://api.openai.com/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    pub provider_name: String,
    pub base_url: String,
    pub api_key: String,
    pub default_model: String,
    pub models: Vec<LlmModelConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmModelConfig {
    pub id: String,
    pub display_name: String,
}

impl LlmProviderConfig {
    pub fn default_config() -> Self {
        Self {
            provider_name: String::from("OpenAI compatible"),
            base_url: DEFAULT_OPENAI_BASE_URL.to_owned(),
            api_key: String::new(),
            default_model: DEFAULT_AGENT_PRESET.model.to_owned(),
            models: vec![LlmModelConfig {
                id: DEFAULT_AGENT_PRESET.model.to_owned(),
                display_name: DEFAULT_AGENT_PRESET.model.to_owned(),
            }],
        }
    }

    pub fn resolved_api_key(&self) -> anyhow::Result<String> {
        let api_key = non_empty_or_env(&self.api_key, "OPENAI_API_KEY")
            .context("OPENAI_API_KEY is required when llm-provider.json api_key is empty")?;

        Ok(api_key)
    }

    pub fn resolved_base_url(&self) -> String {
        non_empty_or_env(&self.base_url, "OPENAI_BASE_URL")
            .unwrap_or_else(|| DEFAULT_OPENAI_BASE_URL.to_owned())
    }

    pub fn resolved_default_model(&self) -> String {
        if self.default_model.trim().is_empty() {
            DEFAULT_AGENT_PRESET.model.to_owned()
        } else {
            self.default_model.trim().to_owned()
        }
    }

    pub fn validate_resolved(&self) -> anyhow::Result<()> {
        if self.resolved_api_key()?.trim().is_empty() {
            bail!("LLM provider api_key cannot be empty");
        }

        if self.resolved_base_url().trim().is_empty() {
            bail!("LLM provider base_url cannot be empty");
        }

        if self.resolved_default_model().trim().is_empty() {
            bail!("LLM provider default_model cannot be empty");
        }

        Ok(())
    }
}

fn non_empty_or_env(value: &str, env_key: &str) -> Option<String> {
    let value = value.trim();

    if !value.is_empty() {
        return Some(value.to_owned());
    }

    std::env::var(env_key)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}
