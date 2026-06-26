    use super::*;
    use std::sync::MutexGuard;

    fn test_lock() -> MutexGuard<'static, ()> {
        hi_toml::test_env_lock()
    }

    fn with_temp_hi<F: FnOnce()>(f: F) {
        let _guard = test_lock();
        let dir = std::env::temp_dir().join(format!("hi-channels-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let hi_path = dir.join("hi.toml");
        std::env::set_var("HI_TOML", hi_path.to_string_lossy().to_string());
        f();
        std::env::remove_var("HI_TOML");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn migrate_legacy_split_files_into_hi_toml() {
        with_temp_hi(|| {
            let dir = hi_toml::hi_dir();
            std::fs::write(
                dir.join("config.toml"),
                r#"
workspace = "/tmp/ws"
[ai]
provider = "openai-compat"
model = "m"
api_key = "k"
[wecom]
bot_id = "bot1"
secret = "sec1"
"#,
            )
            .unwrap();

            hi_toml::migrate_legacy_config_files().unwrap();

            assert!(paths::hi_config_path().exists());
            let text = std::fs::read_to_string(paths::hi_config_path()).unwrap();
            assert!(text.contains("[ai]"));
            assert!(text.contains("bot_id"));
            assert!(!dir.join("config.toml").exists());

            let channels = ChannelsConfig::load().unwrap();
            assert_eq!(channels.require_wecom().unwrap().bot_id, "bot1");
            assert!(channels.require_wecom().unwrap().enabled);
        });
    }

    #[test]
    fn save_writes_channels_section() {
        with_temp_hi(|| {
            let mut channels = ChannelsConfig::default();
            channels.set_wecom_account(
                "default",
                WeComConfig {
                    bot_id: "b".into(),
                    secret: "s".into(),
                    ..Default::default()
                },
            );
            channels.save().unwrap();
            let text = std::fs::read_to_string(paths::hi_config_path()).unwrap();
            assert!(text.contains("[channels.wecom]"));
            assert!(!text.contains("[wecom]"));
        });
    }

    #[test]
    fn save_writes_weixin_section() {
        with_temp_hi(|| {
            let mut channels = ChannelsConfig::default();
            channels.set_weixin_account(
                "default",
                WeixinConfig {
                    bot_token: "tok".into(),
                    ilink_bot_id: "bot1".into(),
                    ilink_user_id: "u@im.wechat".into(),
                    ..Default::default()
                },
            );
            channels.save().unwrap();
            let text = std::fs::read_to_string(paths::hi_config_path()).unwrap();
            assert!(text.contains("[channels.weixin]"));
            assert!(text.contains("bot_token"));
            let loaded = ChannelsConfig::load().unwrap();
            assert_eq!(loaded.weixin_account("default").unwrap().ilink_bot_id, "bot1");
        });
    }

    #[test]
    fn save_writes_feishu_section() {
        with_temp_hi(|| {
            let mut channels = ChannelsConfig::default();
            channels.set_feishu_account(
                "default",
                FeishuConfig {
                    app_id: "cli_x".into(),
                    app_secret: "sec".into(),
                    allow_from: vec!["ou_1".into()],
                    ..Default::default()
                },
            );
            channels.save().unwrap();
            let text = std::fs::read_to_string(paths::hi_config_path()).unwrap();
            assert!(text.contains("[channels.feishu]"));
            assert!(text.contains("app_id"));
            assert!(text.contains("mention_enabled = true"));
        });
    }

    #[test]
    fn parses_feishu_mention_enabled() {
        with_temp_hi(|| {
            std::fs::write(
                paths::hi_config_path(),
                r#"
[channels.feishu]
app_id = "cli_x"
app_secret = "sec"
allow_from = ["ou_1"]
mention_enabled = false
"#,
            )
            .unwrap();

            let channels = ChannelsConfig::load().unwrap();
            assert!(!channels.feishu_account("default").unwrap().mention_enabled);
        });
    }

    #[test]
    fn save_omits_empty_channels_section() {
        with_temp_hi(|| {
            let channels = ChannelsConfig::default();
            channels.save().unwrap();
            let text = std::fs::read_to_string(paths::hi_config_path()).unwrap();
            assert!(!text.contains("[channels"));
        });
    }

    #[test]
    fn save_removes_unsupported_channel_sections() {
        with_temp_hi(|| {
            std::fs::write(
                paths::hi_config_path(),
                r#"
[channels.discord]
bot_token = "old"
"#,
            )
            .unwrap();

            let mut channels = ChannelsConfig::default();
            channels.set_wecom_account(
                "default",
                WeComConfig {
                    bot_id: "b".into(),
                    secret: "s".into(),
                    ..Default::default()
                },
            );
            channels.save().unwrap();
            let text = std::fs::read_to_string(paths::hi_config_path()).unwrap();
            assert!(!text.contains("[channels.discord]"));
            assert!(!text.contains("bot_token"));
        });
    }

    #[test]
    fn parses_multiple_wecom_accounts_with_per_instance_enabled() {
        with_temp_hi(|| {
            std::fs::write(
                paths::hi_config_path(),
                r#"
[channels.wecom]
enabled = true
bot_id = "main-bot"
secret = "sec-main"

[channels.wecom.support]
enabled = false
bot_id = "support-bot"
secret = "sec-support"
"#,
            )
            .unwrap();

            let channels = ChannelsConfig::load().unwrap();
            assert_eq!(channels.wecom_account("default").unwrap().bot_id, "main-bot");
            assert_eq!(channels.wecom_account("support").unwrap().bot_id, "support-bot");

            let endpoints = channels.enabled_endpoints().unwrap();
            assert_eq!(endpoints.len(), 2);
            assert_eq!(endpoints[0].id, "http");
            assert_eq!(endpoints[1].id, "wecom");
        });
    }

    #[test]
    fn enabled_endpoints_includes_http_by_default() {
        with_temp_hi(|| {
            std::fs::write(
                paths::hi_config_path(),
                r#"
[channels.wecom]
bot_id = "a"
secret = "s1"

[channels.wecom.ops]
bot_id = "b"
secret = "s2"
"#,
            )
            .unwrap();

            let channels = ChannelsConfig::load().unwrap();
            let ids: Vec<_> = channels
                .enabled_endpoints()
                .unwrap()
                .into_iter()
                .map(|e| e.id)
                .collect();
            assert_eq!(ids, vec!["http", "wecom", "wecom:ops"]);
        });
    }

    #[test]
    fn migrates_legacy_root_enabled_list() {
        with_temp_hi(|| {
            std::fs::write(
                paths::hi_config_path(),
                r#"
enabled = ["wecom", "wecom:support"]

[wecom]
bot_id = "main-bot"
secret = "sec-main"

[wecom.support]
bot_id = "support-bot"
secret = "sec-support"
"#,
            )
            .unwrap();

            let channels = ChannelsConfig::load().unwrap();
            let endpoints = channels.enabled_endpoints().unwrap();
            assert_eq!(endpoints.len(), 3);
            assert_eq!(endpoints[0].id, "http");
            assert_eq!(endpoints[1].id, "wecom");
            assert_eq!(endpoints[2].id, "wecom:support");
        });
    }

    #[test]
    fn migrates_legacy_root_default_only() {
        with_temp_hi(|| {
            std::fs::write(
                paths::hi_config_path(),
                r#"
default = "wecom"

[wecom]
bot_id = "a"
secret = "s1"

[wecom.ops]
bot_id = "b"
secret = "s2"
"#,
            )
            .unwrap();

            let channels = ChannelsConfig::load().unwrap();
            let endpoints = channels.enabled_endpoints().unwrap();
            assert_eq!(endpoints.len(), 2);
            assert_eq!(endpoints[0].id, "http");
            assert_eq!(endpoints[1].id, "wecom");
            assert!(!channels.wecom_account("ops").unwrap().enabled);
        });
    }
