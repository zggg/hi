    use super::*;

    #[test]
    fn default_base_url() {
        let cfg = WeixinConfig::default();
        assert_eq!(cfg.base_url(), WeixinConfig::DEFAULT_BASE_URL);
    }
