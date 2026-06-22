    use super::*;
    use crate::llm::{ChatMessage, Role};

    #[test]
    fn pressure_at_seventy_and_ninety_percent() {
        assert!(budget_pressure_notice(Locale::Zh, 6, 10).is_none());
        let at70 = budget_pressure_notice(Locale::Zh, 7, 10).unwrap();
        assert!(at70.contains("7/10"));
        let at90 = budget_pressure_notice(Locale::Zh, 9, 10).unwrap();
        assert!(at90.contains("9/10"));
        assert!(budget_pressure_notice(Locale::Zh, 10, 10).unwrap().contains("10/10"));
    }

    #[test]
    fn apply_pressure_appends_to_last_tool_message() {
        let mut history = vec![
            ChatMessage {
                role: Role::Assistant,
                content: String::new(),
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            },
            ChatMessage {
                role: Role::Tool,
                content: "ok".into(),
                tool_calls: None,
                tool_call_id: Some("t1".into()),
                reasoning_content: None,
            },
        ];
        apply_budget_pressure(Locale::Zh, &mut history, 9, 10);
        assert!(history.last().unwrap().content.contains("预算提醒"));
        assert!(history.last().unwrap().content.starts_with("ok\n\n"));
    }
