    use super::*;

    #[test]
    fn setup_is_top_level_command() {
        let cli = Cli::parse_from(["hi", "setup"]);
        assert!(matches!(cli.command, Some(Commands::Setup)));
    }

    #[test]
    fn gateway_setup_is_subcommand() {
        let cli = Cli::parse_from(["hi", "gateway", "setup"]);
        let Some(Commands::Gateway { action, check }) = cli.command else {
            panic!("expected gateway command");
        };
        assert!(!check);
        assert!(matches!(action, Some(GatewayAction::Setup)));
    }

    #[test]
    fn tui_accepts_session_on_default_command() {
        let cli = Cli::parse_from(["hi", "--session", "tui:work"]);
        assert_eq!(cli.session.as_deref(), Some("tui:work"));
        assert!(cli.command.is_none());
    }

    #[test]
    fn tui_accepts_session_on_subcommand() {
        let cli = Cli::parse_from(["hi", "tui", "--session", "tui:debug"]);
        let Some(Commands::Tui { session }) = cli.command else {
            panic!("expected tui command");
        };
        assert_eq!(session.as_deref(), Some("tui:debug"));
    }

    #[test]
    fn tui_subcommand_session_overrides_root() {
        let cli = Cli::parse_from([
            "hi",
            "--session",
            "tui:root",
            "tui",
            "--session",
            "tui:sub",
        ]);
        let Some(Commands::Tui { session }) = cli.command else {
            panic!("expected tui command");
        };
        assert_eq!(cli.session.as_deref(), Some("tui:root"));
        assert_eq!(session.as_deref(), Some("tui:sub"));
    }

    #[test]
    fn chat_accepts_explicit_session_with_trailing_message() {
        let cli = Cli::parse_from([
            "hi",
            "chat",
            "--session",
            "web:tenant:user:thread",
            "hello",
            "world",
        ]);

        let Some(Commands::Chat { session, message }) = cli.command else {
            panic!("expected chat command");
        };

        assert_eq!(session.as_deref(), Some("web:tenant:user:thread"));
        assert_eq!(message, vec!["hello", "world"]);
    }

    #[test]
    fn chat_rejects_message_flag() {
        let cli = Cli::parse_from(["hi", "chat", "-m", "hello"]);
        let Some(Commands::Chat { message, .. }) = cli.command else {
            panic!("expected chat command");
        };
        assert!(chat_single_from_args(&message, hi_core::Locale::En).is_err());
    }

    #[test]
    fn chat_rejects_unknown_long_flag() {
        let cli = Cli::parse_from(["hi", "chat", "--sessoin", "typo", "hello"]);
        let Some(Commands::Chat { message, .. }) = cli.command else {
            panic!("expected chat command");
        };
        assert!(chat_single_from_args(&message, hi_core::Locale::En).is_err());
    }
