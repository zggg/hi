    use super::*;

    #[test]
    fn row_count_starts_at_min_and_grows() {
        assert_eq!(input_row_count("", 0, 80), INPUT_ROWS_MIN);
        assert_eq!(input_row_count("a\n\n", 3, 80), 3);
    }

    #[test]
    fn viewport_pads_blank_line_below_prompt() {
        let lines = input_viewport_lines("", 0, 2, 80);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].spans[0].content.contains('>'));
        assert_eq!(lines[1].spans[0].content, "  ");
    }

    #[test]
    fn viewport_scrolls_to_cursor_on_many_lines() {
        let text = "a\nb\nc\nd";
        let cursor = text.len();
        let lines = input_viewport_lines(text, cursor, 2, 80);
        assert_eq!(lines.len(), 2);
        assert!(lines[1].spans.iter().any(|s| s.content.contains('d')));
    }
