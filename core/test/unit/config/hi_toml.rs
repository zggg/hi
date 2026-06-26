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

    #[test]
    fn write_document_places_http_before_context_and_wecom_after_tools() {
        with_temp_hi(|| {
            let mut http = toml::Table::new();
            http.insert("enabled".into(), true.into());
            http.insert("host".into(), "127.0.0.1".into());
            http.insert("port".into(), 9527.into());

            let mut wecom = toml::Table::new();
            wecom.insert("enabled".into(), true.into());
            wecom.insert("bot_id".into(), "bot".into());

            let mut channels = toml::Table::new();
            channels.insert("http".into(), Value::Table(http));
            channels.insert("wecom".into(), Value::Table(wecom));

            let mut context = toml::Table::new();
            context.insert("max_tool_iterations".into(), 12.into());

            let mut tools = toml::Table::new();
            let mut approvals = toml::Table::new();
            approvals.insert("mode".into(), "on".into());
            tools.insert("approvals".into(), Value::Table(approvals));

            let mut root = toml::Table::new();
            root.insert("workspace".into(), "/tmp/ws".into());
            root.insert("ai".into(), Value::Table(toml::Table::new()));
            root.insert("logging".into(), Value::Table(toml::Table::new()));
            root.insert("storage".into(), Value::Table(toml::Table::new()));
            root.insert("gateway".into(), Value::Table(toml::Table::new()));
            root.insert("channels".into(), Value::Table(channels));
            root.insert("context".into(), Value::Table(context));
            root.insert("tools".into(), Value::Table(tools));

            write_document(&Value::Table(root)).unwrap();
            let text = std::fs::read_to_string(paths::hi_config_path()).unwrap();

            let http = text.find("[channels.http]").unwrap();
            let context = text.find("[context]").unwrap();
            let tools = text.find("[tools.approvals]").unwrap();
            let wecom = text.find("[channels.wecom]").unwrap();

            assert!(http < context, "http before context");
            assert!(context < tools, "context before tools");
            assert!(tools < wecom, "wecom after tools");
        });
    }
