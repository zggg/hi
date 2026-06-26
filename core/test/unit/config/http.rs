use super::HttpConfig;

#[test]
fn default_http_enabled_on_loopback() {
    let cfg = HttpConfig::default();
    assert!(cfg.enabled);
    assert_eq!(cfg.host, "127.0.0.1");
    assert_eq!(cfg.port, 9527);
    assert!(cfg.is_loopback());
}

#[test]
fn non_loopback_requires_token_on_start() {
    let cfg = HttpConfig {
        host: "0.0.0.0".into(),
        ..Default::default()
    };
    assert!(cfg.validate_for_start().is_err());
    let ok = HttpConfig {
        host: "0.0.0.0".into(),
        token: "secret".into(),
        ..Default::default()
    };
    assert!(ok.validate_for_start().is_ok());
}
