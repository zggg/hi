    use super::*;

    #[test]
    fn allowlist_requires_allow_from() {
        let cfg = WeComConfig {
            dm_policy: "allowlist".into(),
            allow_from: vec![],
            ..Default::default()
        };
        assert!(cfg.validate_dm_access().is_err());
        let ok = WeComConfig {
            allow_from: vec!["u1".into()],
            ..cfg
        };
        assert!(ok.validate_dm_access().is_ok());
    }
