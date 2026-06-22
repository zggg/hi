    use super::*;

    #[test]
    fn leaves_short_output_unchanged() {
        assert_eq!(limit_tool_output("ok".into(), 10), "ok");
    }

    #[test]
    fn truncates_long_output() {
        let out = limit_tool_output("x".repeat(100), 20);
        assert!(out.contains("truncated"));
        assert!(out.chars().count() > 20);
    }
