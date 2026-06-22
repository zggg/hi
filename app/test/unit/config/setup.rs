    use super::*;
    use hi_core::Locale;

    fn sample_ai(name: &str, model: &str, key: &str) -> AiConfig {
        AiConfig {
            default: name.into(),
            providers: BTreeMap::new(),
            provider: "openai-compat".into(),
            model: model.into(),
            base_url: Some("https://api.example.com".into()),
            api_key: key.into(),
        }
    }

    #[test]
    fn merge_providers_first_install_writes_only_chosen() {
        let previous = empty_ai_baseline();
        let chosen = sample_ai("deepseek", "deepseek-v4-flash", "sk-new");
        let merged = merge_providers(&previous, chosen, false);
        assert_eq!(merged.default, "deepseek");
        assert_eq!(merged.providers.len(), 1);
        assert!(merged.providers.contains_key("deepseek"));
        assert!(!merged.providers.contains_key("openai-compat"));
    }

    #[test]
    fn merge_providers_update_keeps_previous_instance() {
        let previous = sample_ai("deepseek", "deepseek-v4-flash", "sk-old");
        let mut previous = previous;
        previous.providers.insert("deepseek".into(), AiProviderEntry {
            provider: "openai-compat".into(),
            model: "deepseek-v4-flash".into(),
            base_url: Some("https://api.deepseek.com".into()),
            api_key: "sk-old".into(),
        });
        let chosen = sample_ai("codex", "gpt-5.5", "");
        let merged = merge_providers(&previous, chosen, true);
        assert_eq!(merged.default, "codex");
        assert!(merged.providers.contains_key("deepseek"));
        assert!(merged.providers.contains_key("codex"));
    }

    #[test]
    fn finish_message_mentions_gateway_when_configured() {
        let with_gw = finish_message(Locale::En, true);
        assert!(with_gw.contains("hi gateway --check"));
        assert!(!finish_message(Locale::En, false).contains("hi gateway --check"));
    }
