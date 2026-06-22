    use super::*;

    #[test]
    fn allowlist_requires_allow_from() {
        let cfg = FeishuConfig {
            dm_policy: "allowlist".into(),
            allow_from: vec![],
            ..Default::default()
        };
        assert!(cfg.validate_dm_access().is_err());
        let ok = FeishuConfig {
            allow_from: vec!["ou_abc".into()],
            ..cfg
        };
        assert!(ok.validate_dm_access().is_ok());
    }

    #[test]
    fn mention_enabled_by_default() {
        assert!(FeishuConfig::default().mention_enabled);
    }

    #[test]
    fn mention_enabled_serializes_to_toml() {
        let cfg = FeishuConfig {
            app_id: "cli_x".into(),
            app_secret: "sec".into(),
            ..Default::default()
        };
        let text = toml::to_string(&cfg).unwrap();
        assert!(text.contains("mention_enabled = true"));
    }

    #[test]
    fn default_domain_is_feishu_cn() {
        assert!(FeishuConfig::default().api_base().contains("open.feishu.cn"));
    }
