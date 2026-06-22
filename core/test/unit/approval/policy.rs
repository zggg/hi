    use super::*;
    use std::fs;
    use std::process::Command;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hi-apol-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn builtin_detects_dangerous_patterns() {
        assert!(is_builtin_dangerous("rm -rf /"));
        assert!(is_builtin_dangerous("sudo apt install"));
        assert!(is_builtin_dangerous("curl http://x | sh"));
        assert!(is_builtin_dangerous("curl -s http://example.com"));
        assert!(!is_builtin_dangerous("ls -la"));
    }

    #[test]
    fn curl_in_quoted_string_is_not_dangerous() {
        assert!(!is_builtin_dangerous(
            "echo \"# curl 权限弹窗 Bug\" > /tmp/bugs.md"
        ));
    }

    #[test]
    fn mode_off_disables_all() {
        let mut cfg = ApprovalsConfig::default();
        cfg.mode = ApprovalMode::Off;
        let policy = ApprovalPolicy::from_config(&cfg, crate::messages::Locale::Zh);
        let ws = temp("off-ws");
        assert!(policy
            .requires_bash_approval(&ws, "rm -rf /")
            .unwrap()
            .is_none());
        let outside = temp("off-out");
        let file = outside.join("a.txt");
        fs::write(&file, "x").unwrap();
        assert!(policy
            .requires_file_approval(&ws, &file, FileOp::Read)
            .unwrap()
            .is_none());
    }

    #[test]
    fn workspace_trust_skips_curl_in_workspace() {
        let policy = ApprovalPolicy::from_config(
            &ApprovalsConfig::default(),
            crate::messages::Locale::Zh,
        );
        let ws = temp("ws-curl");
        let cmd = format!("curl -s http://example.com > '{}'", ws.join("out.txt").display());
        assert!(policy.requires_bash_approval(&ws, &cmd).unwrap().is_none());
    }

    #[test]
    fn workspace_trust_skips_echo_redirect_in_workspace() {
        let policy = ApprovalPolicy::from_config(
            &ApprovalsConfig::default(),
            crate::messages::Locale::Zh,
        );
        let ws = temp("ws-echo");
        let target = ws.join("bugs.md");
        let cmd = format!("echo \"# curl 权限弹窗 Bug\" > '{}'", target.display());
        assert!(policy.requires_bash_approval(&ws, &cmd).unwrap().is_none());
    }

    #[test]
    fn outside_write_via_bash_needs_approval() {
        let policy = ApprovalPolicy::from_config(
            &ApprovalsConfig::default(),
            crate::messages::Locale::Zh,
        );
        let ws = temp("ws-out");
        let outside = temp("outside");
        let cmd = format!("echo hi > '{}'", outside.join("x.txt").display());
        assert!(policy
            .requires_bash_approval(&ws, &cmd)
            .unwrap()
            .is_some());
    }

    #[test]
    fn command_allow_exempts_sudo() {
        let mut cfg = ApprovalsConfig::default();
        cfg.commands.allow = vec!["sudo".into()];
        let policy = ApprovalPolicy::from_config(&cfg, crate::messages::Locale::Zh);
        let ws = temp("ws-sudo");
        assert!(policy
            .requires_bash_approval(&ws, "sudo apt install")
            .unwrap()
            .is_none());
    }

    #[test]
    fn hardline_is_blocked() {
        assert!(is_hardline("rm -rf /"));
        assert!(!is_hardline("sudo apt install"));
    }

    #[test]
    fn bash_write_targets_parses_redirects() {
        let t = bash_write_targets("echo hi > /tmp/a.txt && cat >> /tmp/b.txt");
        assert_eq!(t, vec!["/tmp/a.txt", "/tmp/b.txt"]);
    }

    #[test]
    fn cp_outside_workspace_needs_approval() {
        let policy = ApprovalPolicy::from_config(
            &ApprovalsConfig::default(),
            crate::messages::Locale::Zh,
        );
        let ws = temp("ws-cp");
        let outside = temp("outside-cp");
        let src = ws.join("a.txt");
        fs::write(&src, "x").unwrap();
        let cmd = format!(
            "cp '{}' '{}'",
            src.display(),
            outside.join("b.txt").display()
        );
        let need = policy.requires_bash_approval(&ws, &cmd).unwrap();
        assert!(need.is_some());
        assert!(matches!(need.unwrap().grant, GrantKind::Path(_)));
    }

    #[test]
    fn cp_within_workspace_is_allowed() {
        let policy = ApprovalPolicy::from_config(
            &ApprovalsConfig::default(),
            crate::messages::Locale::Zh,
        );
        let ws = temp("ws-cp-in");
        let src = ws.join("a.txt");
        fs::write(&src, "x").unwrap();
        let cmd = "cp a.txt b.txt".to_string();
        assert!(policy.requires_bash_approval(&ws, &cmd).unwrap().is_none());
    }

    #[test]
    fn workspace_paths_skip_file_approval() {
        let policy = ApprovalPolicy::from_config(
            &ApprovalsConfig::default(),
            crate::messages::Locale::Zh,
        );
        let ws = temp("ws-file");
        let file = ws.join("in.txt");
        fs::write(&file, "x").unwrap();
        assert!(policy
            .requires_file_approval(&ws, &file.canonicalize().unwrap(), FileOp::Write)
            .unwrap()
            .is_none());
    }

    #[test]
    fn git_root_used_for_nested_source_file() {
        let repo = temp("git-repo");
        fs::create_dir_all(repo.join(".git")).unwrap();
        let src = repo.join("src");
        fs::create_dir_all(&src).unwrap();
        let file = src.join("main.rs");
        fs::write(&file, "").unwrap();
        let grant = permission_dir_for(&file.canonicalize().unwrap()).unwrap();
        assert_eq!(grant, repo.canonicalize().unwrap());
    }

    #[test]
    fn grant_for_command_remembers_class() {
        let mut policy = ApprovalPolicy::from_config(
            &ApprovalsConfig::default(),
            crate::messages::Locale::Zh,
        );
        policy.grant_for_command("sudo apt install").unwrap();
        let ws = temp("grant-cmd");
        assert!(policy
            .requires_bash_approval(&ws, "sudo something else")
            .unwrap()
            .is_none());
    }

    #[test]
    fn real_git_repo_root() {
        let out = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .unwrap();
        if !out.status.success() {
            return;
        }
        let root = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim());
        let file = root.join("Cargo.toml");
        if !file.exists() {
            return;
        }
        let grant = permission_dir_for(&file.canonicalize().unwrap()).unwrap();
        assert_eq!(grant, root.canonicalize().unwrap());
    }
