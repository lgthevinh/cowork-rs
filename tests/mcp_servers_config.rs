use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct McpServersConfig {
    mcp_servers: BTreeMap<String, McpServerConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct McpServerConfig {
    command: Option<String>,
    args: Option<Vec<String>>,
}

#[test]
fn example_config_enables_filesystem_stdio_server() {
    let config: McpServersConfig =
        serde_json::from_str(include_str!("../mcp-servers.example.json"))
            .expect("example config should parse");

    let filesystem = config
        .mcp_servers
        .get("filesystem")
        .expect("filesystem server should exist");

    assert_eq!(filesystem.command.as_deref(), Some("npx"));
    assert_eq!(
        filesystem.args.as_deref(),
        Some(
            [
                "-y".to_owned(),
                "@modelcontextprotocol/server-filesystem".to_owned(),
                ".".to_owned(),
            ]
            .as_slice()
        )
    );
}
