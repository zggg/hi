use crate::config::{Config, GatewayConfig};

#[test]
fn default_max_concurrent_turns_is_sixteen() {
    assert_eq!(GatewayConfig::default().max_concurrent_turns, 16);
    assert_eq!(GatewayConfig::default().effective_max_concurrent_turns(), 16);
}

#[test]
fn effective_max_concurrent_turns_clamps() {
    let mut cfg = GatewayConfig::default();
    cfg.max_concurrent_turns = 0;
    assert_eq!(cfg.effective_max_concurrent_turns(), 1);

    cfg.max_concurrent_turns = 999;
    assert_eq!(cfg.effective_max_concurrent_turns(), 64);
}

#[test]
fn load_gateway_section_from_toml() {
    let text = r#"
workspace = "/tmp/ws"
[ai]
provider = "openai-compat"
model = "test"
api_key = "sk-test"
[gateway]
max_concurrent_turns = 8
"#;
    let config: Config = toml::from_str(text).expect("parse");
    assert_eq!(config.gateway.max_concurrent_turns, 8);
    assert_eq!(config.gateway.effective_max_concurrent_turns(), 8);
}
