    use super::*;

    #[test]
    fn parse_extract_json_array() {
        let raw = r#"[
          {"kind":"preference","content":"使用简体中文","confidence":"confirmed"},
          {"kind":"task","content":"写测试","confidence":"inferred","task_status":"open"}
        ]"#;
        let knots = parse_extract_json(raw).unwrap();
        assert_eq!(knots.len(), 2);
        assert_eq!(knots[0].kind, KnotKind::Preference);
        assert_eq!(knots[1].task_status, Some(TaskStatus::Open));
    }

    #[test]
    fn parse_strips_code_fence() {
        let raw = "```json\n[]\n```";
        assert!(parse_extract_json(raw).unwrap().is_empty());
    }

    #[test]
    fn cue_detects_explicit_and_self_statement() {
        let msg = |content: &str| ChatMessage {
            role: Role::User,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        };
        assert!(turn_has_memory_cue(&[msg("记住我的偏好是简体中文")]));
        assert!(turn_has_memory_cue(&[msg("我叫 gz")]));
        assert!(turn_has_memory_cue(&[msg("Please remember my email")]));
        assert!(!turn_has_memory_cue(&[msg("帮我看下这个报错")]));
        assert!(!turn_has_memory_cue(&[msg("1+1 等于几")]));
    }

    #[test]
    fn cue_ignores_assistant_messages() {
        let assistant = ChatMessage {
            role: Role::Assistant,
            content: "我记住了你的偏好".into(),
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        };
        assert!(!turn_has_memory_cue(&[assistant]));
    }

    #[test]
    fn format_excerpt_skips_system() {
        let msgs = vec![
            ChatMessage {
                role: Role::System,
                content: "sys".into(),
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            },
            ChatMessage {
                role: Role::User,
                content: "记住：我叫 gz".into(),
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            },
        ];
        let excerpt = format_excerpt(&msgs);
        assert!(excerpt.contains("gz"));
        assert!(!excerpt.contains("sys"));
    }
