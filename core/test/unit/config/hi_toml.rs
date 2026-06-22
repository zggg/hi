    use super::*;

    fn with_temp_hi<F: FnOnce()>(f: F) {
        let _guard = test_env_lock();
        let dir = std::env::temp_dir().join(format!("hi-toml-order-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("HI_TOML", dir.join("hi.toml").to_string_lossy().to_string());
        f();
        std::env::remove_var("HI_TOML");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_document_uses_logical_key_order() {
        with_temp_hi(|| {
            let mut entry = toml::Table::new();
            entry.insert("provider".into(), "openai-compat".into());
            entry.insert("model".into(), "m".into());
            entry.insert("api_key".into(), "k".into());
            let mut providers = toml::Table::new();
            providers.insert("openai-compat".into(), Value::Table(entry));
            let mut ai = toml::Table::new();
            ai.insert("default".into(), "openai-compat".into());
            ai.insert("providers".into(), Value::Table(providers));

            let mut root = toml::Table::new();
            root.insert("workspace".into(), "/tmp/ws".into());
            root.insert("ai".into(), Value::Table(ai));

            write_document(&Value::Table(root)).unwrap();
            let text = std::fs::read_to_string(paths::hi_config_path()).unwrap();

            let ws = text.find("workspace").unwrap();
            let ai_sec = text.find("[ai]").unwrap();
            let default = text.find("default").unwrap();
            let providers = text.find("[ai.providers").unwrap();

            assert!(ws < ai_sec, "workspace before [ai]");
            assert!(default < providers, "default before providers in [ai]");
        });
    }
