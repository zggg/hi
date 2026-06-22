    use super::*;

    #[test]
    fn normalize_strips_carriage_returns() {
        assert_eq!(normalize_text("a\r\nb\rc"), "a\nbc");
        assert_eq!(normalize_text("line1\r\nline2"), "line1\nline2");
    }

    #[test]
    fn reply_only_falls_back_to_reasoning_deltas() {
        let events = vec![
            AgentEvent::ReasoningDelta {
                text: "think".into(),
            },
            AgentEvent::TurnCompleted,
        ];
        let mut assistant = String::new();
        let mut reasoning = String::new();
        for event in &events {
            match event {
                AgentEvent::ReasoningDelta { text } => reasoning.push_str(text),
                AgentEvent::AssistantDelta { text } => assistant.push_str(text),
                _ => {}
            }
        }
        if assistant.is_empty() && !reasoning.is_empty() {
            assistant = reasoning;
        }
        assert_eq!(assistant, "think");
    }

    #[test]
    fn verbose_mode_shows_tools_not_in_reply_only() {
        let events = vec![
            AgentEvent::ToolCallStarted {
                name: "read".into(),
                arguments: r#"{"path":"a.txt"}"#.into(),
            },
            AgentEvent::ToolCallFinished {
                name: "read".into(),
                success: true,
                output: "ok".into(),
            },
            AgentEvent::AssistantDelta {
                text: "done".into(),
            },
            AgentEvent::TurnCompleted,
        ];
        let mut out = Vec::new();
        {
            let mut assistant = String::new();
            let mut had_tools = false;
            for event in &events {
                match event {
                    AgentEvent::ToolCallStarted { name, .. } => {
                        had_tools = true;
                        let _ = writeln!(out, "[tool] {name} …");
                    }
                    AgentEvent::AssistantDelta { text } => assistant.push_str(text),
                    _ => {}
                }
            }
            assert!(had_tools);
            assert_eq!(assistant, "done");
        }
    }
