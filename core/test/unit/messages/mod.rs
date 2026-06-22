    use super::*;
    use std::sync::Mutex;

    static LOCALE_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn locale_test_lock() -> std::sync::MutexGuard<'static, ()> {
        LOCALE_TEST_LOCK.lock().unwrap()
    }

    #[test]
    fn zh_missing_api_key() {
        let msg = t(Locale::Zh, MessageId::MissingApiKey, &[]);
        assert!(msg.contains("hi setup"));
        assert!(msg.contains("api_key") || msg.contains("API"));
    }

    #[test]
    fn en_missing_api_key() {
        let msg = t(Locale::En, MessageId::MissingApiKey, &[]);
        assert!(msg.contains("hi setup"));
        assert!(msg.to_lowercase().contains("api"));
    }

    #[test]
    fn resolve_prefers_hi_locale_env() {
        let _guard = locale_test_lock();
        std::env::set_var("HI_LOCALE", "en");
        let loc = resolve_locale(None);
        std::env::remove_var("HI_LOCALE");
        assert_eq!(loc, Locale::En);
    }

    #[test]
    fn resolve_uses_config_when_set() {
        let _guard = locale_test_lock();
        assert_eq!(resolve_locale(Some("en")), Locale::En);
        assert_eq!(resolve_locale(Some("zh")), Locale::Zh);
    }
