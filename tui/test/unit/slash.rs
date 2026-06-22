    use super::*;

    #[test]
    fn slash_token_only_on_first_line_prefix() {
        assert_eq!(slash_token("/res", 4), Some("/res"));
        assert_eq!(slash_token("/reset arg", 6), Some("/reset"));
        assert_eq!(slash_token("/reset arg", 8), None);
        assert_eq!(slash_token("hi /reset", 3), None);
        assert_eq!(slash_token("/a\nb", 2), Some("/a"));
    }

    #[test]
    fn filter_empty_shows_all() {
        assert_eq!(filter_commands("").len(), COMMANDS.len());
    }

    #[test]
    fn menu_hides_when_command_complete() {
        assert!(menu_visible("/", 1, false));
        assert!(!menu_visible("/reset", 6, false));
        assert!(!menu_visible("/compact", 8, false));
    }

    #[test]
    fn model_submenu_on_model_command() {
        assert_eq!(model_submenu_filter("/model", 6), Some(""));
        assert_eq!(
            model_submenu_filter("/model deep", 11),
            Some("deep")
        );
        assert!(model_submenu_filter("/models", 7).is_none());
        assert!(!top_level_slash_visible("/model", 6, false));
        assert!(top_level_slash_visible("/mod", 4, false));
    }

    #[test]
    fn filter_includes_model() {
        assert_eq!(filter_commands("").len(), 3);
        assert_eq!(filter_commands("")[0].name, "/model");
        assert!(filter_commands("mod").iter().any(|c| c.name == "/model"));
    }

    #[test]
    fn menu_scrolls_third_item_into_view() {
        let all = filter_commands("");
        let lines = menu_lines(&all, 2, 80, 2, Locale::Zh);
        assert_eq!(lines.len(), 2);
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(text.contains("/compact"));
    }

    #[test]
    fn filter_prefix_and_clear_alias() {
        let m = filter_commands("re");
        assert!(m.iter().any(|c| c.name == "/reset"));
        assert!(filter_commands("cl").iter().any(|c| c.name == "/reset"));
        assert_eq!(filter_commands("comp").len(), 1);
    }
