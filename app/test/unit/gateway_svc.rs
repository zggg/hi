    use super::*;

    #[test]
    fn notify_reload_without_gateway_does_not_panic() {
        clean_stale_pid();
        let path = pid_path();
        let _ = fs::remove_file(&path);
        notify_reload();
    }
