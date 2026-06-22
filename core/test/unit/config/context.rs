    use super::*;

    #[test]
    fn migrates_legacy_max_tokens_and_turns() {
        let raw: ContextConfigRaw = toml::from_str(
            r#"
enabled = true
max_tokens = 64000
compress_threshold = 0.75
keep_recent_turns = 6
"#,
        )
        .unwrap();
        let cfg = ContextConfig::from_raw(raw);
        assert_eq!(cfg.window_k, 64);
        assert_eq!(cfg.compress_at_k, 48);
        assert_eq!(cfg.protect_tail_k, 24);
    }

    #[test]
    fn new_k_fields_take_precedence() {
        let raw: ContextConfigRaw = toml::from_str(
            r#"
window_k = 200
compress_at_k = 150
protect_tail_k = 40
max_tool_iterations = 8
"#,
        )
        .unwrap();
        let cfg = ContextConfig::from_raw(raw);
        assert_eq!(cfg.window_k, 200);
        assert_eq!(cfg.compress_at_k, 150);
        assert_eq!(cfg.protect_tail_k, 40);
        assert_eq!(cfg.max_tool_iterations, 8);
    }

    #[test]
    fn default_max_tool_iterations_is_twelve() {
        let cfg = ContextConfig::default();
        assert_eq!(cfg.max_tool_iterations, 12);
    }
