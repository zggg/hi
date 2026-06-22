    use super::*;

    use hi_core::Locale;

    #[test]
    fn summarize_channels_empty() {
        let channels = ChannelsConfig::default();
        assert_eq!(
            summarize_configured_channels(&channels, Locale::Zh),
            "未配置"
        );
    }
