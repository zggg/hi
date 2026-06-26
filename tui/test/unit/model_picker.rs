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

    fn render(lines: &[ratatui::text::Line<'static>]) -> String {
        lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect()
    }

    #[test]
    fn model_list_marks_current_model() {
        let models = vec!["gpt-4o".to_string(), "gpt-5".to_string()];
        let lines = model_list_lines(&models, Some("gpt-5"), 0, 80, 4);
        let text = render(&lines);
        assert!(text.contains("gpt-4o"));
        assert!(text.contains("gpt-5"));
        assert!(text.contains("(当前)"));
    }

    #[test]
    fn model_list_no_current_has_no_marker() {
        let models = vec!["gpt-4o".to_string(), "gpt-5".to_string()];
        let lines = model_list_lines(&models, None, 1, 80, 4);
        let text = render(&lines);
        assert!(!text.contains("(当前)"));
    }

    #[test]
    fn model_list_windows_selection_into_view() {
        let models: Vec<String> = (0..10).map(|i| format!("model-{i}")).collect();
        // 选中超出首屏的项时，窗口应滚动到包含该项。
        let lines = model_list_lines(&models, None, 9, 80, 4);
        let text = render(&lines);
        assert!(text.contains("model-9"));
        assert!(!text.contains("model-0"));
    }
