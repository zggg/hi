    use super::*;

    #[test]
    fn left_right_and_insert_at_cursor() {
        let mut input = InputArea::default();
        for ch in "abcd".chars() {
            input.handle(KeyCode::Char(ch), KeyModifiers::empty());
        }
        input.handle(KeyCode::Left, KeyModifiers::empty());
        input.handle(KeyCode::Left, KeyModifiers::empty());
        input.handle(KeyCode::Char('X'), KeyModifiers::empty());
        assert_eq!(input.as_str(), "abXcd");
        assert_eq!(input.cursor(), "abX".len());
    }

    #[test]
    fn backspace_deletes_before_cursor() {
        let mut input = InputArea::default();
        for ch in "ab".chars() {
            input.handle(KeyCode::Char(ch), KeyModifiers::empty());
        }
        input.handle(KeyCode::Left, KeyModifiers::empty());
        input.handle(KeyCode::Backspace, KeyModifiers::empty());
        assert_eq!(input.as_str(), "b");
    }

    #[test]
    fn paste_with_trailing_newline_inserts_not_submits() {
        let mut input = InputArea::default();
        assert_eq!(
            input.handle_paste("line1\nline2\n"),
            InputAction::None
        );
        assert_eq!(input.as_str(), "line1\nline2\n");
    }

    #[test]
    fn shift_enter_inserts_newline_at_cursor() {
        let mut input = InputArea::default();
        input.handle(KeyCode::Char('a'), KeyModifiers::empty());
        input.handle(KeyCode::Enter, KeyModifiers::SHIFT);
        input.handle(KeyCode::Char('b'), KeyModifiers::empty());
        assert_eq!(input.as_str(), "a\nb");
    }

    #[test]
    fn ctrl_j_inserts_newline_at_cursor() {
        let mut input = InputArea::default();
        input.handle(KeyCode::Char('a'), KeyModifiers::empty());
        assert_eq!(
            input.handle(KeyCode::Char('j'), KeyModifiers::CONTROL),
            InputAction::None
        );
        input.handle(KeyCode::Char('b'), KeyModifiers::empty());
        assert_eq!(input.as_str(), "a\nb");
    }

    #[test]
    fn shift_held_enter_inserts_newline() {
        let mut input = InputArea::default();
        input.set_shift_held(true);
        input.handle(KeyCode::Char('a'), KeyModifiers::empty());
        assert_eq!(input.handle(KeyCode::Enter, KeyModifiers::empty()), InputAction::None);
        assert_eq!(input.as_str(), "a\n");
    }

    #[test]
    fn enter_submit_blocked_briefly_after_paste() {
        let mut input = InputArea::default();
        input.handle_paste("hello");
        assert_eq!(input.handle(KeyCode::Enter, KeyModifiers::empty()), InputAction::None);
        assert_eq!(input.as_str(), "hello");
    }
