    use super::*;

    #[test]
    fn picker_marks_active_profile() {
        let profiles = vec![
            ModelProfile {
                name: "a".into(),
                adapter: "openai-compat".into(),
                model: "gpt-4o".into(),
                active: false,
            },
            ModelProfile {
                name: "b".into(),
                adapter: "openai-compat".into(),
                model: "gpt-5".into(),
                active: true,
            },
        ];
        let lines = model_picker_lines(&profiles, 0, 80, 4);
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(text.contains("(当前)"));
        assert!(text.contains("gpt-5"));
    }
