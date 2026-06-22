    use super::*;

    #[test]
    fn default_level_is_info() {
        assert_eq!(LoggingConfig::default().level, "info");
    }

    #[test]
    fn normalizes_unknown_level() {
        assert_eq!(normalize_log_level("verbose"), "info");
        assert_eq!(normalize_log_level("DEBUG"), "debug");
    }
