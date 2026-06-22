    use super::*;
    use crate::llm::Role;

    fn temp_db(name: &str) -> (std::path::PathBuf, SessionStore) {
        let dir = std::env::temp_dir().join(format!("hi-store-{name}-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let db = dir.join("sessions.db");
        let _ = std::fs::remove_file(&db);
        (db.clone(), SessionStore::open(&db).unwrap())
    }

    #[test]
    fn append_and_reload_context() {
        let (_db, store) = temp_db("ctx");
        let session_id = SessionId("chat:main".into());
        store.get_or_create_session(&session_id, "/tmp/p").unwrap();

        let ids = store
            .append_messages(
                &session_id,
                &[
                    ChatMessage {
                        role: Role::System,
                        content: "sys".into(),
                        tool_calls: None,
                        tool_call_id: None,
                        reasoning_content: None,
                    },
                    ChatMessage {
                        role: Role::User,
                        content: "hi".into(),
                        tool_calls: None,
                        tool_call_id: None,
                        reasoning_content: None,
                    },
                ],
            )
            .unwrap();
        assert_eq!(ids.len(), 2);

        let ctx = store.load_context_messages(&session_id).unwrap();
        assert_eq!(ctx.len(), 2);
        assert_eq!(ctx[1].message.content, "hi");

        let all = store.load_all_messages(&session_id).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn compression_retains_full_transcript() {
        let (_db, store) = temp_db("compress");
        let session_id = SessionId("chat:main".into());
        store.get_or_create_session(&session_id, "/tmp/p").unwrap();

        let ids = store
            .append_messages(
                &session_id,
                &[
                    ChatMessage {
                        role: Role::System,
                        content: "sys".into(),
                        tool_calls: None,
                        tool_call_id: None,
                        reasoning_content: None,
                    },
                    ChatMessage {
                        role: Role::User,
                        content: "old".into(),
                        tool_calls: None,
                        tool_call_id: None,
                        reasoning_content: None,
                    },
                    ChatMessage {
                        role: Role::Assistant,
                        content: "ack".into(),
                        tool_calls: None,
                        tool_call_id: None,
                        reasoning_content: None,
                    },
                    ChatMessage {
                        role: Role::User,
                        content: "recent".into(),
                        tool_calls: None,
                        tool_call_id: None,
                        reasoning_content: None,
                    },
                ],
            )
            .unwrap();

        let compression_id = store
            .apply_compression(
                &session_id,
                NewSessionCompression {
                    message_id_from: ids[1],
                    message_id_to: ids[2],
                    message_count: 2,
                    token_estimate: Some(100),
                    summary_text: Some("summary".into()),
                },
            )
            .unwrap();
        assert!(compression_id > 0);

        let all = store.load_all_messages(&session_id).unwrap();
        assert_eq!(all.len(), 4);
        assert!(!all[1].in_context);
        assert!(!all[2].in_context);

        let ctx = store.load_context_messages(&session_id).unwrap();
        assert_eq!(ctx.len(), 2);
        assert_eq!(ctx[1].message.content, "recent");

        let comps = store.list_compressions(&session_id).unwrap();
        assert_eq!(comps.len(), 1);
        assert_eq!(comps[0].summary_text.as_deref(), Some("summary"));
    }

    #[test]
    fn update_system_message_without_delete() {
        let (_db, store) = temp_db("sysupd");
        let session_id = SessionId("tui:main".into());
        store.get_or_create_session(&session_id, "/a").unwrap();
        store
            .append_messages(
                &session_id,
                &[ChatMessage {
                    role: Role::System,
                    content: "cwd=/a".into(),
                    tool_calls: None,
                    tool_call_id: None,
                    reasoning_content: None,
                }],
            )
            .unwrap();
        store
            .update_system_message(&session_id, "cwd=/b")
            .unwrap();
        let ctx = store.load_context_messages(&session_id).unwrap();
        assert_eq!(ctx.len(), 1);
        assert_eq!(ctx[0].message.content, "cwd=/b");
    }

    #[test]
    fn reset_context_hides_non_system_rows() {
        let (_db, store) = temp_db("resetctx");
        let session_id = SessionId("chat:main".into());
        store.get_or_create_session(&session_id, "/tmp/p").unwrap();
        store
            .append_messages(
                &session_id,
                &[
                    ChatMessage {
                        role: Role::System,
                        content: "sys".into(),
                        tool_calls: None,
                        tool_call_id: None,
                        reasoning_content: None,
                    },
                    ChatMessage {
                        role: Role::User,
                        content: "big".into(),
                        tool_calls: None,
                        tool_call_id: None,
                        reasoning_content: None,
                    },
                ],
            )
            .unwrap();

        let hidden = store.reset_session_context(&session_id).unwrap();
        assert_eq!(hidden, 1);
        let ctx = store.load_context_messages(&session_id).unwrap();
        assert_eq!(ctx.len(), 1);
        assert_eq!(ctx[0].message.role, Role::System);
    }

    #[test]
    fn cross_process_reload() {
        let (db, store) = temp_db("cross");
        let session_id = SessionId("chat:main".into());
        store.get_or_create_session(&session_id, "/tmp").unwrap();
        store
            .append_messages(
                &session_id,
                &[ChatMessage {
                    role: Role::User,
                    content: "persist".into(),
                    tool_calls: None,
                    tool_call_id: None,
                    reasoning_content: None,
                }],
            )
            .unwrap();

        let store2 = SessionStore::open(&db).unwrap();
        let loaded = store2.load_all_messages(&session_id).unwrap();
        assert_eq!(loaded.len(), 1);
    }
