    use super::*;
    use hi_core::error::Error;

    #[test]
    fn normalize_reply_parts_fills_empty() {
        assert_eq!(
            normalize_reply_parts(Locale::Zh, vec![]),
            vec![t(Locale::Zh, MessageId::EmptyChannelReply, &[])]
        );
        assert_eq!(
            normalize_reply_parts(Locale::Zh, vec!["  ".into()]),
            vec![t(Locale::Zh, MessageId::EmptyChannelReply, &[])]
        );
    }

    #[test]
    fn user_visible_error_shortens_transport_failure() {
        let err = Error::Message(
            "无法连接大模型服务，请检查网络、代理，以及 hi.toml 中的 base_url。\n详情：timeout".into(),
        );
        assert!(user_visible_error(Locale::Zh, &err).contains("无法连接大模型服务"));
    }
