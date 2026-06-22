    use super::*;

    #[test]
    fn window_scrolls_to_keep_selection_visible() {
        assert_eq!(menu_window_start(3, 0, 2), 0);
        assert_eq!(menu_window_start(3, 1, 2), 0);
        assert_eq!(menu_window_start(3, 2, 2), 1);
    }
