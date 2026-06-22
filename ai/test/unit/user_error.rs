    use super::*;

    #[test]
    fn http_401_includes_hint() {
        let msg = http_completion_error(StatusCode::UNAUTHORIZED, "Invalid API key");
        assert!(msg.contains("401"));
        assert!(msg.contains("Invalid API key"));
        assert!(msg.contains("hi setup"));
    }

    #[test]
    fn rewrite_legacy_openai_error() {
        let raw = "chat completion failed (401 Unauthorized): bad key";
        let msg = present_provider_error(anyhow::anyhow!(raw));
        assert!(msg.contains("401"));
        assert!(msg.contains("bad key"));
        assert!(!msg.contains("chat completion failed"));
    }
