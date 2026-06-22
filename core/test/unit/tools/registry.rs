    use std::sync::Arc;

    use super::*;
    use crate::approval::shared_approval_policy;
    use crate::config::{ApprovalsConfig, ApprovalMode};
    use crate::config::MemoryConfig;
    use crate::memory::OwnerId;
    use crate::store::SessionStore;
    use crate::SessionId;

    fn registry(dir: &std::path::Path, policy: SharedApprovalPolicy) -> ToolRegistry {
        ToolRegistry::with_builtin(dir.to_path_buf(), policy, None, 16_384)
    }

    #[tokio::test]
    async fn mode_off_allows_read_outside_workspace() {
        let dir = std::env::temp_dir().join(format!("hi-read-star-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let outside = std::env::temp_dir().join(format!("hi-read-out-star-{}", std::process::id()));
        std::fs::create_dir_all(&outside).unwrap();
        let outside_file = outside.join("x.txt");
        std::fs::write(&outside_file, "remote").unwrap();

        let mut cfg = ApprovalsConfig::default();
        cfg.mode = ApprovalMode::Off;
        let registry = registry(&dir, shared_approval_policy(&cfg, crate::messages::Locale::Zh));
        let mut events = Vec::new();
        let out = registry
            .execute(
                "read",
                &format!(r#"{{"path":"{}"}}"#, outside_file.display()),
                None,
                &mut events,
                None,
            )
            .await
            .unwrap();
        assert_eq!(out.trim(), "remote");
    }

    #[tokio::test]
    async fn outside_workspace_grants_in_memory() {
        let dir = std::env::temp_dir().join(format!("hi-read-grant-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let outside = std::env::temp_dir().join(format!("hi-outside-grant-{}", std::process::id()));
        std::fs::create_dir_all(&outside).unwrap();
        let file = outside.join("a.txt");
        std::fs::write(&file, "ok").unwrap();

        let mut cfg = ApprovalsConfig::default();
        cfg.workspace.trust = false;
        let policy = shared_approval_policy(&cfg, crate::messages::Locale::Zh);
        assert!(policy
            .read()
            .unwrap()
            .requires_file_approval(&dir, &file.canonicalize().unwrap(), FileOp::Read)
            .unwrap()
            .is_some());

        let mut guard = policy.write().unwrap();
        guard.grant_for_path(&file).unwrap();
        drop(guard);
        assert!(policy
            .read()
            .unwrap()
            .requires_file_approval(&dir, &file.canonicalize().unwrap(), FileOp::Read)
            .unwrap()
            .is_none());

        let registry = registry(&dir, policy);
        let mut events = Vec::new();
        let out = registry
            .execute(
                "read",
                &format!(r#"{{"path":"{}"}}"#, file.display()),
                None,
                &mut events,
                None,
            )
            .await
            .unwrap();
        assert_eq!(out.trim(), "ok");
    }

    #[tokio::test]
    async fn registry_includes_memory_search_when_configured() {
        let dir = std::env::temp_dir().join(format!("hi-msreg-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let db = dir.join("sessions.db");
        let _ = std::fs::remove_file(&db);
        let store = Arc::new(SessionStore::open(&db).unwrap());
        let session_id = SessionId("chat:main".into());
        let session = store.get_or_create_session(&session_id, "/tmp").unwrap();

        let registry = ToolRegistry::with_builtin(
            dir.clone(),
            shared_approval_policy(&ApprovalsConfig::default(), crate::messages::Locale::Zh),
            Some(MemoryToolDeps {
                store,
                session_id: session.session_id.clone(),
                config: MemoryConfig::default(),
            }),
            16_384,
        );
        let names: Vec<_> = registry.definitions().into_iter().map(|d| d.name).collect();
        assert!(names.iter().any(|n| n == "memory_search"));
        assert!(names.iter().any(|n| n == "memory_write"));
    }

    #[tokio::test]
    async fn memory_write_tool_persists_knot() {
        let dir = std::env::temp_dir().join(format!("hi-mwtool-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let db = dir.join("sessions.db");
        let _ = std::fs::remove_file(&db);
        let store = Arc::new(SessionStore::open(&db).unwrap());
        let session_id = SessionId("chat:main".into());
        let session = store.get_or_create_session(&session_id, "/tmp").unwrap();

        let registry = ToolRegistry::with_builtin(
            dir,
            shared_approval_policy(&ApprovalsConfig::default(), crate::messages::Locale::Zh),
            Some(MemoryToolDeps {
                store: store.clone(),
                session_id: session.session_id,
                config: MemoryConfig::default(),
            }),
            16_384,
        );

        let mut events = Vec::new();
        let out = registry
            .execute(
                "memory_write",
                r#"{"content":"偏好简体中文","kind":"preference","confidence":"confirmed"}"#,
                None,
                &mut events,
                None,
            )
            .await
            .unwrap();
        assert!(out.contains("已记录"));

        let owner = OwnerId("local".into());
        let knots = store.list_knots(&owner).unwrap();
        assert!(knots.iter().any(|k| k.content == "偏好简体中文"));
    }
