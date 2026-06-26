    use super::*;
    use std::sync::MutexGuard;

    fn test_lock() -> MutexGuard<'static, ()> {
        hi_toml::test_env_lock()
    }

    fn with_temp_hi<F: FnOnce()>(f: F) {
        let _guard = test_lock();
        let dir = std::env::temp_dir().join(format!("hi-config-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("HI_TOML", dir.join("hi.toml").to_string_lossy().to_string());
        f();
        std::env::remove_var("HI_TOML");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_persisted_none_when_file_missing() {
        with_temp_hi(|| {
            assert!(Config::load_persisted().unwrap().is_none());
        });
    }

    #[test]
    fn load_persisted_none_without_ai_section() {
        with_temp_hi(|| {
            let mut doc = hi_toml::read_document().unwrap();
            doc.as_table_mut()
                .unwrap()
                .insert("workspace".into(), "/tmp/ws".into());
            hi_toml::write_document(&doc).unwrap();
            assert!(Config::load_persisted().unwrap().is_none());
        });
    }

    #[test]
    fn load_persisted_some_when_ai_present() {
        with_temp_hi(|| {
            let text = r#"
workspace = "/tmp/hi-test"
[ai]
provider = "openai-compat"
model = "test"
api_key = "sk-test"
"#;
            std::fs::write(Config::config_path(), text).unwrap();
            let loaded = Config::load_persisted().unwrap().expect("configured");
            assert_eq!(loaded.ai.model, "test");
        });
    }

    #[test]
    fn load_legacy_ai_flat_format() {
        let text = r#"
workspace = "/tmp/hi-test"
[ai]
provider = "openai-compat"
model = "test"
api_key = "sk-test"
"#;
        let config: Config = toml::from_str(text).expect("parse config");
        assert_eq!(config.ai.model, "test");
        assert_eq!(config.ai.default, "openai-compat");
        assert!(config.ai.providers.contains_key("openai-compat"));
    }

    #[test]
    fn load_ai_providers_format() {
        let text = r#"
workspace = "/tmp/hi-test"
[ai]
default = "deepseek-work"

[ai.providers.deepseek-work]
provider = "openai-compat"
model = "deepseek-v4"
api_key = "sk-test"
"#;
        let config: Config = toml::from_str(text).expect("parse config");
        assert_eq!(config.ai.model, "deepseek-v4");
        assert_eq!(config.ai.default, "deepseek-work");
    }

    #[test]
    fn save_config_preserves_existing_channels_section() {
        with_temp_hi(|| {
            let mut doc = hi_toml::read_document().unwrap();
            let mut wecom = toml::map::Map::new();
            wecom.insert("bot_id".into(), "x".into());
            wecom.insert("secret".into(), "y".into());
            let mut channels = toml::map::Map::new();
            channels.insert("wecom".into(), toml::Value::Table(wecom));
            doc.as_table_mut().unwrap().insert(
                "channels".into(),
                toml::Value::Table(channels),
            );
            hi_toml::write_document(&doc).unwrap();

            let config = Config::default();
            config.save().expect("save");

            let text = std::fs::read_to_string(Config::config_path()).unwrap();
            assert!(text.contains("[channels.wecom]"));
            assert!(text.contains("bot_id"));
        });
    }

    #[test]
    fn save_config_seeds_tools_approvals_on_first_write() {
        with_temp_hi(|| {
            let config = Config::default();
            config.save().expect("save");

            let text = std::fs::read_to_string(Config::config_path()).unwrap();
            assert!(text.contains("[tools.approvals]"));
            assert!(text.contains("mode = \"on\""));
            assert!(text.contains("[tools.approvals.workspace]"));
            assert!(text.contains("trust = true"));

            let loaded = Config::load().expect("load");
            assert!(loaded.tools.approvals.commands.allow.is_empty());
            assert!(loaded.tools.approvals.filesystem.allow_write.is_empty());
            assert_eq!(loaded.context.max_tool_iterations, 12);
        });
    }

    #[test]
    fn save_config_writes_max_tool_iterations() {
        with_temp_hi(|| {
            let config = Config::default();
            config.save().expect("save");

            let text = std::fs::read_to_string(Config::config_path()).unwrap();
            assert!(text.contains("max_tool_iterations = 12"));
        });
    }

    #[test]
    fn default_storage_read_pool_size_is_four() {
        assert_eq!(StorageConfig::default().read_pool_size, 4);
        assert_eq!(StorageConfig::default().effective_read_pool_size(), 4);
    }

    #[test]
    fn save_config_writes_storage_read_pool_size() {
        with_temp_hi(|| {
            let config = Config::default();
            config.save().expect("save");

            let text = std::fs::read_to_string(Config::config_path()).unwrap();
            assert!(text.contains("[storage]"));
            assert!(text.contains("read_pool_size = 4"));
        });
    }

    #[test]
    fn save_config_writes_gateway_max_concurrent_turns() {
        with_temp_hi(|| {
            let config = Config::default();
            config.save().expect("save");

            let text = std::fs::read_to_string(Config::config_path()).unwrap();
            assert!(text.contains("[gateway]"));
            assert!(text.contains("max_concurrent_turns = 16"));
        });
    }
