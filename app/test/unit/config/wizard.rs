    use super::*;
    use hi_core::{Locale, MessageId, t};

    #[test]
    fn summarize_masks_api_key() {
        let mut config = Config::default();
        config.ai.api_key = "sk-secret".into();
        let text = summarize_config(Locale::Zh, &config);
        assert!(text.contains(&t(Locale::Zh, MessageId::SetupSummaryMaskedKey, &[])));
        assert!(!text.contains("sk-secret"));
    }

    #[test]
    fn select_skips_single_option_by_default() {
        let session = Session::new(PathBuf::from("/tmp/hi.toml"), Locale::Zh);
        let options = [SelectOption {
            value: "only",
            label: "Only",
            hint: "hint",
        }];
        let picked = session.select("x", &options, "only").unwrap();
        assert_eq!(picked, "only");
    }
