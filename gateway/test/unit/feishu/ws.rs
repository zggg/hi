    use super::*;
    use serde_json::json;

    #[test]
    fn strips_at_placeholders() {
        assert_eq!(
            strip_at_placeholders("@_user_1 hello world"),
            "hello world"
        );
    }

    #[test]
    fn group_requires_mention_when_enabled() {
        let mentions = vec![json!({ "id": { "open_id": "ou_bot" } })];
        assert!(should_respond_in_group(true, Some("ou_bot"), &mentions));
        assert!(!should_respond_in_group(true, Some("ou_bot"), &[]));
        assert!(should_respond_in_group(false, Some("ou_bot"), &[]));
    }

    #[test]
    fn mention_matches_bot_open_id() {
        let mention = json!({ "id": { "open_id": "ou_123" } });
        assert!(mention_matches_bot(&mention, "ou_123"));
        assert!(!mention_matches_bot(&mention, "ou_456"));
    }
