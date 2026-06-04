use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::record::record_file::RecordFile;

pub const MCP_SERVERS_CONFIG_PATH: &str = "preference/mcp-servers.json";
pub const LEGACY_MCP_SERVERS_CONFIG_PATH: &str = "mcp-servers.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServersConfig {
    #[serde(default, alias = "servers")]
    pub mcp_servers: BTreeMap<String, McpServerConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub url: Option<String>,
    pub env: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub disabled: bool,
}

impl McpServersConfig {
    pub fn default_config() -> Self {
        Self::default()
    }

    pub fn configured_server_count(&self) -> usize {
        self.mcp_servers.len()
    }
}

pub fn load_or_init_mcp_servers_config() -> anyhow::Result<McpServersConfig> {
    let record_file = RecordFile::open(MCP_SERVERS_CONFIG_PATH)?;

    if !record_file.exists() {
        migrate_legacy_mcp_servers_config(&record_file)?;
    }

    record_file.init(&McpServersConfig::default_config())?;
    record_file.read()
}

fn migrate_legacy_mcp_servers_config(record_file: &RecordFile) -> anyhow::Result<()> {
    let legacy_file = RecordFile::open(LEGACY_MCP_SERVERS_CONFIG_PATH)?;

    if !legacy_file.exists() {
        return Ok(());
    }

    let legacy_config: McpServersConfig = legacy_file.read()?;
    record_file.write(&legacy_config)?;

    Ok(())
}
