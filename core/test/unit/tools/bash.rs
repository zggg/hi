    use std::path::PathBuf;

    use crate::approval::shared_approval_policy;
    use crate::config::ApprovalsConfig;
    use crate::tools::tool::ToolContext;

    use super::*;

    fn temp_workdir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hi-bash-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn uses_bash_echo_dash_n_semantics() {
        let dir = temp_workdir("echo-n");
        let target = dir.join("aaaa.txt");
        let ctx = ToolContext {
            working_directory: dir,
            approval_policy: shared_approval_policy(&ApprovalsConfig::default(), crate::messages::Locale::Zh),
            memory: None,
            tool_output_max_chars: 16_384,
        };
        let policy = shared_approval_policy(&ApprovalsConfig::default(), crate::messages::Locale::Zh);
        let mut events = Vec::new();
        let command = format!("echo -n \"abcd\" > '{}'", target.display());

        let out = run_bash("bash", &command, &ctx, &policy, None, &mut events, None)
            .await
            .unwrap();

        assert!(out.contains("exit 0"));
        assert_eq!(tokio::fs::read_to_string(target).await.unwrap(), "abcd");
    }
