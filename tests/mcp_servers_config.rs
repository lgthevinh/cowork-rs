#[path = "../src/log/mod.rs"]
mod log;

#[path = "../src/record/record_file.rs"]
pub mod record_file;

pub mod record {
    pub use crate::record_file;
}

#[allow(dead_code)]
#[path = "../src/storage/mcp_servers_config.rs"]
mod mcp_servers_config;

use mcp_servers_config::McpServersConfig;

#[test]
fn example_config_enables_filesystem_stdio_server() {
    let config: McpServersConfig =
        serde_json::from_str(include_str!("../preference/mcp-servers.example.json"))
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

#[test]
fn servers_alias_is_accepted_for_legacy_shape() {
    let config: McpServersConfig = serde_json::from_str(
        r#"{
            "servers": {
                "remote": {
                    "url": "http://localhost:3000/mcp",
                    "disabled": true
                }
            }
        }"#,
    )
    .expect("legacy alias should parse");

    let remote = config
        .mcp_servers
        .get("remote")
        .expect("remote server should exist");

    assert_eq!(remote.url.as_deref(), Some("http://localhost:3000/mcp"));
    assert!(remote.disabled);
}
