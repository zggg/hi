    use super::*;

    #[test]
    fn deepseek_models_include_v4() {
        let models = models_for("deepseek");
        assert!(models.iter().any(|m| m.id == "deepseek-v4-flash"));
        assert!(models.iter().any(|m| m.id == "deepseek-v4-pro"));
    }

    #[test]
    fn openai_compat_has_no_curated_models() {
        assert!(models_for("openai-compat").is_empty());
    }
