    use super::*;

    #[test]
    fn is_dangerous_command_uses_default_policy() {
        assert!(is_dangerous_command("rm -rf /"));
        assert!(!is_dangerous_command("ls -la"));
    }
