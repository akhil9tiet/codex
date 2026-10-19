use super::*;
use codex_config::types::AppToolApproval;
use codex_config::types::McpServerOAuthConfig;
use codex_config::types::McpServerToolConfig;
use codex_config::types::McpServerTransportConfig;
use codex_config::types::SessionPickerViewMode;
use codex_protocol::config_types::SERVICE_TIER_DEFAULT_REQUEST_VALUE;
use codex_protocol::config_types::ServiceTier;
use codex_protocol::openai_models::ReasoningEffort;
use pretty_assertions::assert_eq;
#[cfg(unix)]
use std::os::unix::fs::symlink;
use tempfile::tempdir;
use toml::Value as TomlValue;

#[test]
fn blocking_set_model_top_level() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(
        codex_home,
        &[ConfigEdit::SetModel {
            model: Some("gpt-5.4".to_string()),
            effort: Some(ReasoningEffort::High),
        }],
    )
    .expect("persist");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    let expected = r#"model = "gpt-5.4"
model_reasoning_effort = "high"
"#;
    assert_eq!(contents, expected);
}

#[test]
fn batch_write_table_upsert() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(
        codex_home,
        &[ConfigEdit::SetPath {
            segments: vec!["sandbox_workspace_write".to_string(), "network_access".to_string()],
            value: value(true),
        }],
    )
    .expect("persist");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    let expected = r#"[sandbox_workspace_write]
network_access = true
"#;
    assert_eq!(contents, expected);
}

#[test]
fn batch_write_table_upsert_coerces_dotted_key() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(
        codex_home,
        &[ConfigEdit::SetPath {
            segments: vec!["sandbox_workspace_write.network_access".to_string()],
            value: value(true),
        }],
    )
    .expect("persist");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    let expected = r#"[sandbox_workspace_write]
network_access = true
"#;
    assert_eq!(contents, expected);
}

#[test]
fn batch_write_table_upsert_preserves_inline_comments() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();
    let original = r#"[sandbox_workspace_write] # trailing
network_access = false
"#;
    std::fs::write(codex_home.join(CONFIG_TOML_FILE), original).expect("seed config");

    apply_blocking(
        codex_home,
        &[ConfigEdit::SetPath {
            segments: vec!["sandbox_workspace_write".to_string(), "network_access".to_string()],
            value: value(true),
        }],
    )
    .expect("persist");

    let updated = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    let expected = r#"[sandbox_workspace_write] # trailing
network_access = true
"#;
    assert_eq!(updated, expected);
}

#[test]
fn batch_write_upsert_into_commented_inline_table_keeps_comment_above_header() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();
    let original = r#"[features]
# any comment line
multi_agent_v2 = { enabled = true, max_concurrent_threads_per_session = 30 }
"#;
    std::fs::write(codex_home.join(CONFIG_TOML_FILE), original).expect("seed config");

    apply_blocking(
        codex_home,
        &[
            ConfigEdit::SetPath {
                segments: vec![
                    "features".to_string(),
                    "multi_agent_v2".to_string(),
                    "max_concurrent_threads_per_session".to_string(),
                ],
                value: value(21i64),
            },
            ConfigEdit::SetPath {
                segments: vec![
                    "features".to_string(),
                    "multi_agent_v2".to_string(),
                    "disable_direct_message".to_string(),
                ],
                value: value(false),
            },
        ],
    )
    .expect("apply");

    let updated = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    let expected = r#"[features]
# any comment line
[features.multi_agent_v2]
enabled = true
max_concurrent_threads_per_session = 21
disable_direct_message = false
"#;
    assert_eq!(updated, expected);
    // The rewritten file must stay valid TOML: before the fix this produced
    // an invalid `[# any comment line` header.
    updated.parse::<DocumentMut>().expect("rewritten config is valid TOML");
}

#[test]
fn blocking_clear_model_does_not_follow_legacy_active_profile() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(
        codex_home,
        &[ConfigEdit::SetModel {
            model: Some("gpt-5.4".to_string()),
            effort: Some(ReasoningEffort::High),
        }],
    )
    .expect("persist");
    apply_blocking(codex_home, &[ConfigEdit::ClearModel])
        .expect("persist");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    let expected = r#"model_reasoning_effort = "high"
"#;
    assert_eq!(contents, expected);
}

#[test]
fn blocking_set_model_nested() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(
        codex_home,
        &[ConfigEdit::SetModel {
            model: None,
            effort: Some(ReasoningEffort::High),
        }],
    )
    .expect("persist");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    let expected = r#"model_reasoning_effort = "high"
"#;
    assert_eq!(contents, expected);
}

#[test]
fn blocking_set_model_preserves_comments() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();
    let original = "# top comment\nmodel = \"gpt-5.4\"\n";
    std::fs::write(codex_home.join(CONFIG_TOML_FILE), original).expect("seed config");

    apply_blocking(
        codex_home,
        &[ConfigEdit::SetModel {
            model: Some("gpt-5.4-codex".to_string()),
            effort: None,
        }],
    )
    .expect("persist");

    let updated = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    let expected = "# top comment\nmodel = \"gpt-5.4-codex\"\n";
    assert_eq!(updated, expected);
}

#[test]
fn blocking_set_model_uses_toml_value() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(
        codex_home,
        &[ConfigEdit::SetPath {
            segments: vec!["model".to_string()],
            value: value("gpt-5.4".to_string()),
        }],
    )
    .expect("persist");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(contents.contains("gpt-5.4"));
}

#[test]
fn batch_write_value_types() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(
        codex_home,
        &[
            ConfigEdit::SetPath {
                segments: vec!["flag".to_string()],
                value: value(true),
            },
            ConfigEdit::SetPath {
                segments: vec!["count".to_string()],
                value: value(42i64),
            },
            ConfigEdit::SetPath {
                segments: vec!["name".to_string()],
                value: value("codex".to_string()),
            },
        ],
    )
    .expect("persist");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(contents.contains("flag = true"));
    assert!(contents.contains("count = 42"));
    assert!(contents.contains("name = \"codex\""));
}

#[test]
fn batch_write_clear_and_set() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(
        codex_home,
        &[
            ConfigEdit::SetPath {
                segments: vec!["model".to_string()],
                value: value("gpt-5.4".to_string()),
            },
            ConfigEdit::ClearPath {
                segments: vec!["model".to_string()],
            },
        ],
    )
    .expect("persist");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(!contents.contains("model"));
}

#[test]
fn blocking_remove_key_missing_is_ok() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(
        codex_home,
        &[ConfigEdit::ClearPath {
            segments: vec!["missing".to_string()],
        }],
    )
    .expect("persist");
}

#[test]
fn blocking_clear_path_removes_table() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(
        codex_home,
        &[ConfigEdit::SetPath {
            segments: vec!["a".to_string(), "b".to_string()],
            value: value(1i64),
        }],
    )
    .expect("persist");
    apply_blocking(
        codex_home,
        &[ConfigEdit::ClearPath {
            segments: vec!["a".to_string()],
        }],
    )
    .expect("persist");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(!contents.contains("[a]"));
}

#[test]
fn blocking_clear_model_removes_model_key() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(
        codex_home,
        &[ConfigEdit::SetModel {
            model: Some("gpt-5.4".to_string()),
            effort: None,
        }],
    )
    .expect("persist");
    apply_blocking(codex_home, &[ConfigEdit::ClearModel])
        .expect("persist");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(!contents.contains("model ="));
}

#[test]
fn blocking_replace_mcp_servers_roundtrip() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    let mut servers = BTreeMap::new();
    servers.insert(
        "search".to_string(),
        McpServerConfig {
            auth: Default::default(),
            transport: McpServerTransportConfig::Stdio {
                command: "rg".to_string(),
                args: vec!["--json".to_string()],
                env: None,
                env_vars: Vec::new(),
                cwd: None,
            },
            disabled_reason: None,
            startup_timeout_sec: None,
            tool_timeout_sec: None,
            default_tools_approval_mode: None,
            enabled_tools: None,
            disabled_tools: None,
            scopes: None,
            oauth: None,
            oauth_resource: None,
            tools: HashMap::new(),
            environment_id: codex_config::DEFAULT_MCP_SERVER_ENVIRONMENT_ID.to_string(),
            enabled: true,
            required: false,
            startup_readiness: Default::default(),
            supports_parallel_tool_calls: false,
        },
    );

    apply_blocking(codex_home, &[ConfigEdit::ReplaceMcpServers(servers)])
        .expect("persist");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(contents.contains("[mcp_servers.search]"));
}

#[test]
fn blocking_replace_mcp_servers_preserves_inline_comments() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();
    std::fs::write(
        codex_home.join(CONFIG_TOML_FILE),
        r#"[mcp_servers]
foo = { command = "cmd", args = ["--flag"] } # keep me
"#,
    )
    .expect("seed");

    let mut servers = BTreeMap::new();
    servers.insert(
        "foo".to_string(),
        McpServerConfig {
            auth: Default::default(),
            transport: McpServerTransportConfig::Stdio {
                command: "cmd".to_string(),
                args: vec!["--flag".to_string()],
                env: None,
                env_vars: Vec::new(),
                cwd: None,
            },
            disabled_reason: None,
            startup_timeout_sec: None,
            tool_timeout_sec: None,
            default_tools_approval_mode: None,
            enabled_tools: None,
            disabled_tools: None,
            scopes: None,
            oauth: None,
            oauth_resource: None,
            tools: HashMap::new(),
            environment_id: codex_config::DEFAULT_MCP_SERVER_ENVIRONMENT_ID.to_string(),
            enabled: true,
            required: false,
            startup_readiness: Default::default(),
            supports_parallel_tool_calls: false,
        },
    );

    apply_blocking(codex_home, &[ConfigEdit::ReplaceMcpServers(servers)])
        .expect("persist");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(contents.contains("# keep me"));
}

#[test]
fn blocking_replace_mcp_servers_oauth_roundtrip() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    let mut servers = BTreeMap::new();
    servers.insert(
        "oauth_server".to_string(),
        McpServerConfig {
            auth: Default::default(),
            transport: McpServerTransportConfig::StreamableHttp {
                url: "https://example.com/mcp".to_string(),
                bearer_token_env_var: None,
                http_headers: None,
                env_http_headers: None,
            },
            disabled_reason: None,
            startup_timeout_sec: None,
            tool_timeout_sec: None,
            default_tools_approval_mode: None,
            enabled_tools: None,
            disabled_tools: None,
            scopes: Some(vec!["scope1".to_string(), "scope2".to_string()]),
            oauth: Some(McpServerOAuthConfig {
                enabled: true,
                client_id: Some("client-id".to_string()),
                client_secret_env_var: None,
                access_token_env_var: None,
                refresh_token_env_var: None,
                token_url: Some("https://example.com/token".to_string()),
                scopes: None,
            }),
            oauth_resource: Some("https://example.com/resource".to_string()),
            tools: HashMap::new(),
            environment_id: codex_config::DEFAULT_MCP_SERVER_ENVIRONMENT_ID.to_string(),
            enabled: true,
            required: false,
            startup_readiness: Default::default(),
            supports_parallel_tool_calls: false,
        },
    );

    apply_blocking(codex_home, &[ConfigEdit::ReplaceMcpServers(servers)])
        .expect("persist");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(contents.contains("[mcp_servers.oauth_server]"));
}

#[test]
fn blocking_replace_mcp_servers_serializes_tool_approval_overrides(output_token_limit: usize) {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    let mut servers = BTreeMap::new();
    servers.insert(
        "search".to_string(),
        McpServerConfig {
            auth: Default::default(),
            transport: McpServerTransportConfig::Stdio {
                command: "rg".to_string(),
                args: vec!["--json".to_string()],
                env: None,
                env_vars: Vec::new(),
                cwd: None,
            },
            disabled_reason: None,
            startup_timeout_sec: None,
            tool_timeout_sec: None,
            default_tools_approval_mode: Some(AppToolApproval::Prompt),
            enabled_tools: None,
            disabled_tools: None,
            scopes: None,
            oauth: None,
            oauth_resource: None,
            tools: HashMap::from([(
                "search".to_string(),
                McpServerToolConfig {
                    approval_mode: Some(AppToolApproval::Approve),
                    output_token_limit: std::num::NonZeroUsize::new(output_token_limit),
                },
            )]),
            environment_id: codex_config::DEFAULT_MCP_SERVER_ENVIRONMENT_ID.to_string(),
            enabled: true,
            required: false,
            startup_readiness: Default::default(),
            supports_parallel_tool_calls: false,
        },
    );

    let result = apply_blocking(codex_home, &[ConfigEdit::ReplaceMcpServers(servers)]);
    assert!(result.is_ok());

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(contents.contains("approval"));
}

#[test]
fn blocking_replace_mcp_servers_preserves_inline_comment_suffix() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();
    std::fs::write(
        codex_home.join(CONFIG_TOML_FILE),
        r#"[mcp_servers]
foo = { command = "cmd", args = ["--flag"] } # keep me
"#,
    )
    .expect("seed");

    let mut servers = BTreeMap::new();
    servers.insert(
        "foo".to_string(),
        McpServerConfig {
            auth: Default::default(),
            transport: McpServerTransportConfig::Stdio {
                command: "cmd".to_string(),
                args: Vec::new(),
                env: None,
                env_vars: Vec::new(),
                cwd: None,
            },
            environment_id: codex_config::DEFAULT_MCP_SERVER_ENVIRONMENT_ID.to_string(),
            enabled: false,
            required: false,
            startup_readiness: Default::default(),
            supports_parallel_tool_calls: false,
            ..Default::default()
        },
    );

    apply_blocking(codex_home, &[ConfigEdit::ReplaceMcpServers(servers)])
        .expect("persist");

    let updated = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    let expected = r#"[mcp_servers]
[mcp_servers.foo] # keep me
command = "cmd"
args = []
enabled = false
"#;
    assert_eq!(updated, expected);
}

#[test]
fn blocking_replace_mcp_servers_preserves_inline_comment_after_removing_keys() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();
    std::fs::write(
        codex_home.join(CONFIG_TOML_FILE),
        r#"[mcp_servers]
foo = { command = "cmd", args = ["--flag"] } # keep me
"#,
    )
    .expect("seed");

    let mut servers = BTreeMap::new();
    servers.insert(
        "foo".to_string(),
        McpServerConfig {
            auth: Default::default(),
            transport: McpServerTransportConfig::Stdio {
                command: "cmd".to_string(),
                args: Vec::new(),
                env: None,
                env_vars: Vec::new(),
                cwd: None,
            },
            environment_id: codex_config::DEFAULT_MCP_SERVER_ENVIRONMENT_ID.to_string(),
            enabled: true,
            required: false,
            startup_readiness: Default::default(),
            supports_parallel_tool_calls: false,
            ..Default::default()
        },
    );

    apply_blocking(codex_home, &[ConfigEdit::ReplaceMcpServers(servers)])
        .expect("persist");

    let updated = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    let expected = r#"[mcp_servers]
[mcp_servers.foo] # keep me
command = "cmd"
args = []
enabled = true
"#;
    assert_eq!(updated, expected);
}

#[test]
fn blocking_replace_mcp_servers_preserves_inline_comment_prefix_on_update() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();
    std::fs::write(
        codex_home.join(CONFIG_TOML_FILE),
        r#"[mcp_servers]
# leading comment
foo = { command = "cmd", args = ["--flag"] }
"#,
    )
    .expect("seed");

    let mut servers = BTreeMap::new();
    servers.insert(
        "foo".to_string(),
        McpServerConfig {
            auth: Default::default(),
            transport: McpServerTransportConfig::Stdio {
                command: "cmd".to_string(),
                args: Vec::new(),
                env: None,
                env_vars: Vec::new(),
                cwd: None,
            },
            environment_id: codex_config::DEFAULT_MCP_SERVER_ENVIRONMENT_ID.to_string(),
            enabled: false,
            required: false,
            startup_readiness: Default::default(),
            supports_parallel_tool_calls: false,
            ..Default::default()
        },
    );

    apply_blocking(codex_home, &[ConfigEdit::ReplaceMcpServers(servers)])
        .expect("persist");

    let updated = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    let expected = r#"[mcp_servers]
# leading comment
[mcp_servers.foo]
command = "cmd"
args = []
enabled = false
"#;
    assert_eq!(updated, expected);
}

#[test]
fn blocking_replace_mcp_servers_removes_stale_keys() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();
    std::fs::write(
        codex_home.join(CONFIG_TOML_FILE),
        r#"[mcp_servers]
[mcp_servers.foo]
command = "old"
"#,
    )
    .expect("seed");

    let mut servers = BTreeMap::new();
    servers.insert(
        "bar".to_string(),
        McpServerConfig {
            auth: Default::default(),
            transport: McpServerTransportConfig::Stdio {
                command: "new".to_string(),
                args: Vec::new(),
                env: None,
                env_vars: Vec::new(),
                cwd: None,
            },
            environment_id: codex_config::DEFAULT_MCP_SERVER_ENVIRONMENT_ID.to_string(),
            enabled: false,
            required: false,
            startup_readiness: Default::default(),
            supports_parallel_tool_calls: false,
            ..Default::default()
        },
    );

    apply_blocking(codex_home, &[ConfigEdit::ReplaceMcpServers(servers)])
        .expect("persist");

    let updated = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(!updated.contains("foo"));
    assert!(updated.contains("[mcp_servers.bar]"));
}

#[test]
fn blocking_apply_preserves_unrelated_tables() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();
    let original = "[a]\nkey = 1\n\n[b]\nother = 2\n";
    std::fs::write(codex_home.join(CONFIG_TOML_FILE), original).expect("seed config");

    apply_blocking(
        codex_home,
        &[ConfigEdit::SetPath {
            segments: vec!["a".to_string(), "key".to_string()],
            value: value(2i64),
        }],
    )
    .expect("apply");

    let updated = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(updated.contains("[a]"));
    assert!(updated.contains("[b]"));
    assert!(updated.contains("key = 2"));
}

#[test]
fn batch_write_nested_table_upsert() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(
        codex_home,
        &[ConfigEdit::SetPath {
            segments: vec![
                "features".to_string(),
                "multi_agent_v2".to_string(),
                "max_concurrent_threads_per_session".to_string(),
            ],
            value: value(21i64),
        }],
    )
    .expect("apply");

    let updated = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    let expected = r#"[features]
[features.multi_agent_v2]
max_concurrent_threads_per_session = 21
"#;
    assert_eq!(updated, expected);
}

#[test]
fn batch_write_feature_flag_expansion() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();
    let original = r#"[features]
multi_agent_v2 = true
"#;
    std::fs::write(codex_home.join(CONFIG_TOML_FILE), original).expect("seed config");

    apply_blocking(
        codex_home,
        &[ConfigEdit::SetPath {
            segments: vec![
                "features".to_string(),
                "multi_agent_v2".to_string(),
                "max_concurrent_threads_per_session".to_string(),
            ],
            value: value(21i64),
        }],
    )
    .expect("apply");

    let updated = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    let expected = r#"[features]
[features.multi_agent_v2]
enabled = true
max_concurrent_threads_per_session = 21
"#;
    assert_eq!(updated, expected);
}

#[test]
fn batch_write_inline_table_upsert() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();
    let original = r#"[features]
multi_agent_v2 = { enabled = true, max_concurrent_threads_per_session = 30 }
"#;
    std::fs::write(codex_home.join(CONFIG_TOML_FILE), original).expect("seed config");

    apply_blocking(
        codex_home,
        &[ConfigEdit::SetPath {
            segments: vec![
                "features".to_string(),
                "multi_agent_v2".to_string(),
                "max_concurrent_threads_per_session".to_string(),
            ],
            value: value(21i64),
        }],
    )
    .expect("apply");

    let updated = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    let expected = r#"[features]
[features.multi_agent_v2]
enabled = true
max_concurrent_threads_per_session = 21
"#;
    assert_eq!(updated, expected);
}

#[test]
fn blocking_apply_is_idempotent() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    let edits = [ConfigEdit::SetPath {
        segments: vec!["model".to_string()],
        value: value("gpt-5.4".to_string()),
    }];
    apply_blocking(codex_home, &edits).expect("apply");
    apply_blocking(codex_home, &edits).expect("apply");

    let updated = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    let expected = r#"model = "gpt-5.4"
"#;
    assert_eq!(updated, expected);
}

#[test]
fn blocking_apply_empty_edits() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(codex_home, &[]).expect("apply");

    assert!(!codex_home.join(CONFIG_TOML_FILE).exists());
}

#[test]
fn blocking_apply_invalid_path() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    let result = apply_blocking(
        codex_home,
        &[ConfigEdit::SetPath {
            segments: vec![],
            value: value(true),
        }],
    );
    assert!(result.is_err());
}

#[test]
fn blocking_apply_deep_nesting() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(
        codex_home,
        &[ConfigEdit::SetPath {
            segments: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            value: value(1i64),
        }],
    )
    .expect("apply");

    let updated = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    let expected = "[a]\n[a.b]\n[a.b.c]\nvalue = 1\n";
    let _ = expected;
    assert!(updated.contains("[a.b.c]"));
}

#[test]
fn batch_write_boolean_flag() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(
        codex_home,
        &[ConfigEdit::SetPath {
            segments: vec!["features".to_string(), "pta".to_string()],
            value: value(true),
        }],
    )
    .expect("apply");

    let updated = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(updated.contains("pta = true"));
}

#[test]
fn blocking_set_profile_roundtrip() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(codex_home, &[ConfigEdit::SetActiveProfile("dev".to_string())])
        .expect("apply");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(contents.contains("active_profile = \"dev\""));
}

#[test]
fn blocking_set_profile_creates_parent_dirs() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();
    let profile_dir = codex_home.join("profiles").join("dev");
    std::fs::create_dir_all(&profile_dir).expect("seed");

    apply_blocking(codex_home, &[ConfigEdit::SetActiveProfile("dev".to_string())])
        .expect("apply");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(contents.contains("active_profile = \"dev\""));
}

#[test]
fn blocking_clear_profile_roundtrip() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(codex_home, &[ConfigEdit::ClearActiveProfile])
        .expect("apply");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(!contents.contains("active_profile"));
}

#[test]
fn blocking_set_review_model_roundtrip() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(
        codex_home,
        &[ConfigEdit::SetReviewModel {
            model: Some("gpt-5.4".to_string()),
            effort: Some(ReasoningEffort::High),
        }],
    )
    .expect("apply");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(contents.contains("review_model = \"gpt-5.4\""));
    assert!(contents.contains("review_model_reasoning_effort = \"high\""));
}

#[test]
fn blocking_clear_review_model() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(
        codex_home,
        &[ConfigEdit::SetReviewModel {
            model: Some("gpt-5.4".to_string()),
            effort: Some(ReasoningEffort::High),
        }],
    )
    .expect("apply");
    apply_blocking(codex_home, &[ConfigEdit::ClearReviewModel])
        .expect("apply");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(!contents.contains("review_model"));
}

#[test]
fn blocking_set_model_supports_reasoning_effort() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(
        codex_home,
        &[ConfigEdit::SetReviewModel {
            model: None,
            effort: Some(ReasoningEffort::High),
        }],
    )
    .expect("apply");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(contents.contains("review_model_reasoning_effort = \"high\""));
}

#[test]
fn blocking_set_hide_agent_reasoning_roundtrip() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(codex_home, &[ConfigEdit::SetHideAgentReasoning(true)])
        .expect("apply");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(contents.contains("hide_agent_reasoning = true"));
}

#[test]
fn blocking_clear_hide_agent_reasoning() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(codex_home, &[ConfigEdit::SetHideAgentReasoning(true)])
        .expect("apply");
    apply_blocking(codex_home, &[ConfigEdit::ClearHideAgentReasoning])
        .expect("apply");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(!contents.contains("hide_agent_reasoning"));
}

#[test]
fn blocking_set_show_raw_agent_reasoning_roundtrip() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(codex_home, &[ConfigEdit::SetShowRawAgentReasoning(true)])
        .expect("apply");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(contents.contains("show_raw_agent_reasoning = true"));
}

#[test]
fn blocking_clear_show_raw_agent_reasoning() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(codex_home, &[ConfigEdit::SetShowRawAgentReasoning(true)])
        .expect("apply");
    apply_blocking(codex_home, &[ConfigEdit::ClearShowRawAgentReasoning])
        .expect("apply");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(!contents.contains("show_raw_agent_reasoning"));
}

#[test]
fn blocking_set_model_verbose_roundtrip() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(codex_home, &[ConfigEdit::SetModelVerbose(true)])
        .expect("apply");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(contents.contains("model_verbose = true"));
}

#[test]
fn blocking_clear_model_verbose() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(codex_home, &[ConfigEdit::SetModelVerbose(true)])
        .expect("apply");
    apply_blocking(codex_home, &[ConfigEdit::ClearModelVerbose])
        .expect("apply");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(!contents.contains("model_verbose"));
}

#[test]
fn blocking_set_tui_theme_roundtrip() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(codex_home, &[ConfigEdit::SetTuiTheme("dark".to_string())])
        .expect("apply");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(contents.contains("tui_theme = \"dark\""));
}

#[test]
fn blocking_clear_tui_theme() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(codex_home, &[ConfigEdit::SetTuiTheme("dark".to_string())])
        .expect("apply");
    apply_blocking(codex_home, &[ConfigEdit::ClearTuiTheme])
        .expect("apply");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(!contents.contains("tui_theme"));
}

#[test]
fn blocking_set_experimental_instructions_file_roundtrip() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(codex_home, &[ConfigEdit::SetExperimentalInstructionsFile(Some("AGENTS.md".to_string()))])
        .expect("apply");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(contents.contains("experimental_instructions_file = \"AGENTS.md\""));
}

#[test]
fn blocking_clear_experimental_instructions_file() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(codex_home, &[ConfigEdit::SetExperimentalInstructionsFile(Some("AGENTS.md".to_string()))])
        .expect("apply");
    apply_blocking(codex_home, &[ConfigEdit::ClearExperimentalInstructionsFile])
        .expect("apply");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(!contents.contains("experimental_instructions_file"));
}

#[test]
fn blocking_set_experimental_use_exec_command_tool_roundtrip() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(codex_home, &[ConfigEdit::SetExperimentalUseExecCommandTool(true)])
        .expect("apply");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(contents.contains("experimental_use_exec_command_tool = true"));
}

#[test]
fn blocking_clear_experimental_use_exec_command_tool() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(codex_home, &[ConfigEdit::SetExperimentalUseExecCommandTool(true)])
        .expect("apply");
    apply_blocking(codex_home, &[ConfigEdit::ClearExperimentalUseExecCommandTool])
        .expect("apply");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(!contents.contains("experimental_use_exec_command_tool"));
}

#[test]
fn blocking_set_project_doc_max_bytes_roundtrip() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(codex_home, &[ConfigEdit::SetProjectDocMaxBytes(1024)])
        .expect("apply");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(contents.contains("project_doc_max_bytes = 1024"));
}

#[test]
fn blocking_clear_project_doc_max_bytes() {
    let tmp = tempdir().expect("tmpdir");
    let codex_home = tmp.path();

    apply_blocking(codex_home, &[ConfigEdit::SetProjectDocMaxBytes(1024)])
        .expect("apply");
    apply_blocking(codex_home, &[ConfigEdit::ClearProjectDocMaxBytes])
        .expect("apply");

    let contents = std::fs::read_to_string(codex_home.join(CONFIG_TOML_FILE)).expect("read config");
    assert!(!contents.contains("project_doc_max_bytes"));
}
