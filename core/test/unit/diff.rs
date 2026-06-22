    use super::*;

    #[test]
    fn empty_inputs() {
        assert!(line_diff("", "").is_empty());
    }

    #[test]
    fn add_only() {
        let d = line_diff("", "a\nb");
        assert_eq!(d.len(), 2);
        assert!(d.iter().all(|l| l.kind == DiffKind::Add));
    }

    #[test]
    fn remove_and_add() {
        let d = line_diff("old", "new");
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].kind, DiffKind::Remove);
        assert_eq!(d[1].kind, DiffKind::Add);
    }

    #[test]
    fn unchanged_line() {
        let d = line_diff("keep\nold", "keep\nnew");
        assert_eq!(d[0].kind, DiffKind::Context);
        assert_eq!(d[0].text, "keep");
    }
