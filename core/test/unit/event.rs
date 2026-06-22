    use super::*;

    #[test]
    fn channel_reply_omits_tool_traces() {
        let events = vec![
            AgentEvent::ToolCallStarted {
                name: "read".into(),
                arguments: "{}".into(),
            },
            AgentEvent::ToolCallFinished {
                name: "read".into(),
                success: true,
                output: "file body".into(),
            },
            AgentEvent::AssistantDelta {
                text: "文件内容是 hello".into(),
            },
            AgentEvent::TurnCompleted,
        ];
        assert_eq!(channel_reply_text(&events), "文件内容是 hello");
    }

    #[test]
    fn channel_reply_falls_back_to_reasoning() {
        let events = vec![
            AgentEvent::ReasoningDelta {
                text: "only reasoning".into(),
            },
            AgentEvent::TurnCompleted,
        ];
        assert_eq!(channel_reply_text(&events), "only reasoning");
    }

    #[test]
    fn split_channel_message_prefers_newlines() {
        let text = "line one\nline two\nline three";
        let chunks = split_channel_message(text, 12);
        assert_eq!(chunks, vec!["line one\n", "line two\n", "line three"]);
    }

    #[test]
    fn split_channel_message_preserves_utf8() {
        let text = "你好".repeat(5000);
        let chunks = split_channel_message(&text, 100);
        assert!(chunks.len() > 1);
        assert!(chunks.iter().all(|c| c.len() <= 100));
        assert_eq!(chunks.concat(), text);
    }

    #[test]
    fn channel_reply_chunks_never_truncates() {
        let long = "x".repeat(5000);
        let events = vec![
            AgentEvent::AssistantDelta {
                text: long.clone(),
            },
            AgentEvent::TurnCompleted,
        ];
        let chunks = channel_reply_chunks(&events, 1000);
        assert_eq!(chunks.concat(), long);
    }
