    use super::*;
    use hi_core::AgentEvent;

    fn thinking_count(turn: &Turn) -> usize {
        turn.blocks
            .iter()
            .filter(|b| matches!(b, Block::Thinking(_)))
            .count()
    }

    fn reply_count(turn: &Turn) -> usize {
        turn.blocks
            .iter()
            .filter(|b| matches!(b, Block::Reply(_)))
            .count()
    }

    /// 模拟双 forwarder 乱序：正文先到、尾部 reasoning 后到 → 截图里的碎裂形态。
    #[test]
    fn reordered_events_split_reply_and_thinking() {
        let mut turn = Turn::default();
        for _ in 0..5 {
            turn.apply(AgentEvent::ReasoningDelta {
                text: "思".into(),
            });
        }
        turn.apply(AgentEvent::AssistantDelta {
            text: "我是".into(),
        });
        turn.apply(AgentEvent::ReasoningDelta {
            text: "尾思".into(),
        });
        turn.apply(AgentEvent::AssistantDelta {
            text: "，完整自我介绍".into(),
        });
        turn.finalize();

        assert_eq!(thinking_count(&turn), 2, "expect collapsed think + late think");
        assert_eq!(reply_count(&turn), 2, "expect early reply fragment + main reply");
        assert_eq!(turn.blocks[1].as_reply(), "我是");
        assert_eq!(turn.blocks[3].as_reply(), "，完整自我介绍");
    }

    #[test]
    fn tool_output_delta_appends_to_running_tool() {
        let mut turn = Turn::default();
        turn.apply(AgentEvent::ToolCallStarted {
            name: "bash".into(),
            arguments: r#"{"command":"ls"}"#.into(),
        });
        turn.apply(AgentEvent::ToolOutputDelta {
            name: "bash".into(),
            text: "file.txt\n".into(),
        });
        turn.apply(AgentEvent::ToolOutputDelta {
            name: "bash".into(),
            text: "dir/".into(),
        });
        let Block::Tool(t) = &turn.blocks[0] else {
            panic!("expected tool block");
        };
        assert_eq!(t.output, "file.txt\ndir/");
    }

    #[test]
    fn context_compressed_notice_deduped_in_turn() {
        let mut turn = Turn::default();
        let summary = "x".repeat(67);
        for _ in 0..5 {
            turn.apply(AgentEvent::ContextCompressed {
                summary: summary.clone(),
            });
        }
        assert_eq!(turn.notices.len(), 1);
    }

    impl Block {
        fn as_reply(&self) -> &str {
            match self {
                Block::Reply(r) => &r.content,
                _ => panic!("expected reply block"),
            }
        }
    }
