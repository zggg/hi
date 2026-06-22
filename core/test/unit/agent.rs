    use async_trait::async_trait;
    use std::sync::Mutex;

    use super::*;
    use crate::approval::{shared_approval_policy, ApprovalHandler};
    use crate::llm::LlmResponse;
    use crate::memory::OwnerId;
    use crate::ToolCall;

    /// Author: gz
    struct MockClient {
        steps: Mutex<Vec<LlmResponse>>,
    }

    /// Author: gz
    struct AlwaysApprove;

    #[async_trait]
    impl ApprovalHandler for AlwaysApprove {
        async fn approve_bash(&self, _command: &str) -> Result<bool> {
            Ok(true)
        }
    }

    #[async_trait]
    impl LlmClient for MockClient {
        async fn complete(
            &self,
            _request: LlmRequest,
            _on_stream_delta: Option<UnboundedSender<StreamChunk>>,
        ) -> Result<LlmResponse> {
            let mut steps = self.steps.lock().unwrap();
            Ok(steps.pop().unwrap_or(LlmResponse {
                content: Some("done".into()),
                tool_calls: vec![],
                reasoning_content: None,
            }))
        }
    }

    #[tokio::test]
    async fn run_turn_executes_tool_then_replies() {
        let tmp = std::env::temp_dir().join(format!("hi-agent-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        let file = tmp.join("hello.txt");
        std::fs::write(&file, "hello").unwrap();

        let client = MockClient {
            steps: Mutex::new(vec![
                LlmResponse {
                    content: Some("file says hello".into()),
                    tool_calls: vec![],
                    reasoning_content: None,
                },
                LlmResponse {
                    content: None,
                    tool_calls: vec![ToolCall {
                        id: "c1".into(),
                        name: "read".into(),
                        arguments: r#"{"path":"hello.txt"}"#.into(),
                    }],
                    reasoning_content: None,
                },
            ]),
        };

        let mut loop_ = AgentLoop::new(client, "test".into(), tmp.clone());
        let events = loop_
            .run_turn("read file", &AlwaysApprove, None)
            .await
            .unwrap();
        assert!(events.iter().any(|e| {
            matches!(e, AgentEvent::ToolCallStarted { name, .. } if name == "read")
        }));
        assert!(events.iter().any(|e| {
            matches!(e, AgentEvent::AssistantDelta { text } if text == "file says hello")
        }));
    }

    #[tokio::test]
    async fn with_persistence_roundtrip() {
        let dir = std::env::temp_dir().join(format!("hi-agent-persist-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let db = dir.join("sessions.db");
        let _ = std::fs::remove_file(&db);

        let store = Arc::new(SessionStore::open(&db).unwrap());
        let session_a = SessionId("chat:main".into());
        let session_b = SessionId("tui:main".into());
        let workdir = std::env::temp_dir();

        let client = MockClient {
            steps: Mutex::new(vec![LlmResponse {
                content: Some("ok".into()),
                tool_calls: vec![],
                reasoning_content: None,
            }]),
        };

        let policy = shared_approval_policy(&ApprovalsConfig::default(), crate::messages::Locale::Zh);
        let memory = MemoryConfig::default();
        let mut loop1 = AgentLoop::with_persistence(
            client,
            "m".into(),
            crate::messages::Locale::Zh,
            workdir.clone(),
            Arc::clone(&store),
            session_a.clone(),
            ContextConfig {
                enabled: false,
                ..ContextConfig::default()
            },
            memory.clone(),
            Arc::clone(&policy),
        )
        .unwrap();
        loop1
            .run_turn("hello", &AlwaysApprove, None)
            .await
            .unwrap();

        let client2 = MockClient {
            steps: Mutex::new(vec![LlmResponse {
                content: Some("again".into()),
                tool_calls: vec![],
                reasoning_content: None,
            }]),
        };
        let mut loop2 = AgentLoop::with_persistence(
            client2,
            "m".into(),
            crate::messages::Locale::Zh,
            workdir.clone(),
            Arc::clone(&store),
            session_a.clone(),
            ContextConfig {
                enabled: false,
                ..ContextConfig::default()
            },
            memory.clone(),
            Arc::clone(&policy),
        )
        .unwrap();
        assert!(loop2.history.len() >= 3);

        let client3 = MockClient {
            steps: Mutex::new(vec![LlmResponse {
                content: Some("again".into()),
                tool_calls: vec![],
                reasoning_content: None,
            }]),
        };
        let loop3 = AgentLoop::with_persistence(
            client3,
            "m".into(),
            crate::messages::Locale::Zh,
            workdir,
            store,
            session_b,
            ContextConfig {
                enabled: false,
                ..ContextConfig::default()
            },
            memory,
            policy,
        )
        .unwrap();
        assert_eq!(loop3.history.len(), 1);

        let events = loop2
            .run_turn("more", &AlwaysApprove, None)
            .await
            .unwrap();
        assert!(events.iter().any(|e| {
            matches!(e, AgentEvent::AssistantDelta { text } if text == "again")
        }));
    }

    #[tokio::test]
    async fn knot_memory_shared_across_sessions() {
        let dir = std::env::temp_dir().join(format!("hi-knot-xsess-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let db = dir.join("sessions.db");
        let _ = std::fs::remove_file(&db);

        let store = Arc::new(SessionStore::open(&db).unwrap());
        store
            .add_knot(&crate::memory::NewKnot {
                owner_id: crate::memory::OwnerId("local".into()),
                kind: crate::memory::KnotKind::Fact,
                content: "用户代号 gz".into(),
                confidence: crate::memory::KnotConfidence::Confirmed,
                clarity: 1.0,
                permanent: true,
                visibility: crate::memory::KnotVisibility::Inject,
                task_status: None,
            })
            .unwrap();

        let client = MockClient {
            steps: Mutex::new(vec![LlmResponse {
                content: Some("ok".into()),
                tool_calls: vec![],
                reasoning_content: None,
            }]),
        };
        let loop_ = AgentLoop::with_persistence(
            client,
            "m".into(),
            crate::messages::Locale::Zh,
            std::env::temp_dir(),
            store,
            SessionId("tui:main".into()),
            ContextConfig {
                enabled: false,
                ..ContextConfig::default()
            },
            MemoryConfig::default(),
            shared_approval_policy(&ApprovalsConfig::default(), crate::messages::Locale::Zh),
        )
        .unwrap();

        assert!(loop_.history[0].content.contains("用户代号 gz"));
        assert!(loop_.history[0].content.contains("Long-term memory"));
    }

    #[tokio::test]
    async fn extract_after_turn_persists_knot() {
        let dir = std::env::temp_dir().join(format!("hi-extract-turn-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let db = dir.join("sessions.db");
        let _ = std::fs::remove_file(&db);

        let store = Arc::new(SessionStore::open(&db).unwrap());
        let client = MockClient {
            steps: Mutex::new(vec![
                LlmResponse {
                    content: Some(
                        r#"[{"kind":"fact","content":"用户代号 gz","confidence":"confirmed"}]"#
                            .into(),
                    ),
                    tool_calls: vec![],
                    reasoning_content: None,
                },
                LlmResponse {
                    content: Some("好的".into()),
                    tool_calls: vec![],
                    reasoning_content: None,
                },
            ]),
        };
        let mut loop_ = AgentLoop::with_persistence(
            client,
            "m".into(),
            crate::messages::Locale::Zh,
            std::env::temp_dir(),
            store.clone(),
            SessionId("chat:main".into()),
            ContextConfig {
                enabled: false,
                ..ContextConfig::default()
            },
            MemoryConfig {
                enabled: true,
                extract_after_turn: true,
                ..MemoryConfig::default()
            },
            shared_approval_policy(&ApprovalsConfig::default(), crate::messages::Locale::Zh),
        )
        .unwrap();

        let events = loop_
            .run_turn("请记住我的代号是 gz", &AlwaysApprove, None)
            .await
            .unwrap();
        assert!(events.iter().any(|e| {
            matches!(e, AgentEvent::KnotsExtracted { count } if *count == 1)
        }));

        let knots = store.list_knots(&OwnerId("local".into())).unwrap();
        assert_eq!(knots.len(), 1);
        assert!(knots[0].content.contains("gz"));
    }

    #[tokio::test]
    async fn failed_turn_rolls_back_uncommitted_messages() {
        use crate::error::Error;

        struct FailClient;

        #[async_trait]
        impl LlmClient for FailClient {
            async fn complete(
                &self,
                _request: LlmRequest,
                _on_stream_delta: Option<UnboundedSender<StreamChunk>>,
            ) -> Result<LlmResponse> {
                Err(Error::Message("timeout".into()))
            }
        }

        let dir = std::env::temp_dir().join(format!("hi-rollback-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let db = dir.join("sessions.db");
        let _ = std::fs::remove_file(&db);

        let store = Arc::new(SessionStore::open(&db).unwrap());
        let mut loop_ = AgentLoop::with_persistence(
            FailClient,
            "m".into(),
            crate::messages::Locale::Zh,
            dir.clone(),
            Arc::clone(&store),
            SessionId("chat:main".into()),
            ContextConfig::default(),
            MemoryConfig {
                enabled: false,
                ..MemoryConfig::default()
            },
            shared_approval_policy(&ApprovalsConfig::default(), crate::messages::Locale::Zh),
        )
        .unwrap();

        let before = loop_.history.len();
        let err = loop_
            .run_turn("analyze huge project", &AlwaysApprove, None)
            .await;
        assert!(err.is_err());
        assert_eq!(loop_.history.len(), before);

        let ctx = store.load_context_messages(&SessionId("chat:main".into())).unwrap();
        assert_eq!(ctx.len(), before);
    }

    #[test]
    fn rebuild_system_prompt_updates_workdir() {
        let a = std::env::temp_dir().join(format!("hi-wd-a-{}", std::process::id()));
        let b = std::env::temp_dir().join(format!("hi-wd-b-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&a);
        let _ = std::fs::create_dir_all(&b);
        let mut history = vec![ChatMessage {
            role: Role::User,
            content: "hi".into(),
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        }];
        rebuild_system_message(
            &mut history,
            &b,
            None,
            None,
            &MemoryConfig {
                enabled: false,
                ..MemoryConfig::default()
            },
            None,
        )
        .unwrap();
        assert!(history[0].content.contains(&b.display().to_string()));
    }

    /// 模拟官方两阶段 SSE（先 reasoning、后 content），经 agent 单 forwarder 到 live 通道。
    struct TwoPhaseStreamClient;

    #[async_trait]
    impl LlmClient for TwoPhaseStreamClient {
        async fn complete(
            &self,
            _request: LlmRequest,
            on_stream_delta: Option<UnboundedSender<StreamChunk>>,
        ) -> Result<LlmResponse> {
            let on_stream = on_stream_delta.expect("stream forwarder");
            for _ in 0..80 {
                on_stream
                    .send(StreamChunk::Reasoning("思".into()))
                    .unwrap();
            }
            for chunk in ["我是", "，", "DeepSeek", " 助手"] {
                on_stream
                    .send(StreamChunk::Content(chunk.into()))
                    .unwrap();
            }
            Ok(LlmResponse {
                content: Some("我是，DeepSeek 助手".into()),
                tool_calls: vec![],
                reasoning_content: Some("思".repeat(80)),
            })
        }
    }

    #[tokio::test]
    async fn live_channel_preserves_reasoning_before_content() {
        let tmp = std::env::temp_dir().join(format!("hi-stream-order-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);

        let (live_tx, mut live_rx) = tokio::sync::mpsc::unbounded_channel();
        let collector = tokio::spawn(async move {
            let mut events = Vec::new();
            while let Some(ev) = live_rx.recv().await {
                events.push(ev);
            }
            events
        });

        let mut loop_ = AgentLoop::new(TwoPhaseStreamClient, "test".into(), tmp);
        loop_
            .run_turn("你是什么", &AlwaysApprove, Some(live_tx))
            .await
            .unwrap();
        let live_events = collector.await.unwrap();

        let first_content = live_events.iter().position(|e| {
            matches!(e, AgentEvent::AssistantDelta { .. })
        });
        let last_reasoning = live_events.iter().rposition(|e| {
            matches!(e, AgentEvent::ReasoningDelta { .. })
        });
        assert!(first_content.is_some());
        assert!(last_reasoning.is_some());
        assert!(
            first_content.unwrap() > last_reasoning.unwrap(),
            "content must follow all reasoning on live channel"
        );
    }

    /// Author: gz
    struct ToolOnlyLoopClient;

    #[async_trait]
    impl LlmClient for ToolOnlyLoopClient {
        async fn complete(
            &self,
            request: LlmRequest,
            _on_stream_delta: Option<UnboundedSender<StreamChunk>>,
        ) -> Result<LlmResponse> {
            if request.tools.is_empty() {
                return Ok(LlmResponse {
                    content: Some("已完成阶段性探测。".into()),
                    tool_calls: vec![],
                    reasoning_content: None,
                });
            }
            Ok(LlmResponse {
                content: None,
                tool_calls: vec![ToolCall {
                    id: "c1".into(),
                    name: "read".into(),
                    arguments: r#"{"path":"x.txt"}"#.into(),
                }],
                reasoning_content: None,
            })
        }
    }

    #[tokio::test]
    async fn tool_budget_exhausted_emits_summary_not_loop_error() {
        let tmp = std::env::temp_dir().join(format!("hi-budget-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        std::fs::write(tmp.join("x.txt"), "x").unwrap();

        let mut loop_ = AgentLoop::new_with_context(
            ToolOnlyLoopClient,
            "test".into(),
            tmp.clone(),
            crate::messages::Locale::Zh,
            ContextConfig {
                enabled: false,
                max_tool_iterations: 2,
                ..ContextConfig::default()
            },
            MemoryConfig {
                enabled: false,
                ..MemoryConfig::default()
            },
            shared_approval_policy(&ApprovalsConfig::default(), crate::messages::Locale::Zh),
        );

        let events = loop_
            .run_turn("probe many times", &AlwaysApprove, None)
            .await
            .unwrap();

        assert!(!events.iter().any(|e| {
            matches!(
                e,
                AgentEvent::Error { message } if message.contains("tool loop exceeded")
            )
        }));
        assert!(events.iter().any(|e| {
            matches!(
                e,
                AgentEvent::AssistantDelta { text }
                    if text.contains("阶段性总结") && text.contains("已完成阶段性探测")
            )
        }));
        assert!(loop_.history.iter().any(|m| {
            m.role == Role::Tool && m.content.contains("预算提醒")
        }));
    }
