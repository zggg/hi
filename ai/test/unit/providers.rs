    use super::*;

    #[test]
    fn ollama_normalizes_base_url() {
        let p = OllamaProvider::new(Some("http://localhost:11434".into()));
        assert_eq!(p.name(), "ollama");
    }

    #[test]
    fn anthropic_maps_tool_messages() {
        let msgs = vec![
            ChatMessage {
                role: Role::Assistant,
                content: String::new(),
                tool_calls: Some(vec![ToolCall {
                    id: "t1".into(),
                    name: "read".into(),
                    arguments: r#"{"path":"a.txt"}"#.into(),
                }]),
                tool_call_id: None,
                reasoning_content: None,
            },
            ChatMessage {
                role: Role::Tool,
                content: "data".into(),
                tool_calls: None,
                tool_call_id: Some("t1".into()),
                reasoning_content: None,
            },
        ];
        let out = to_anthropic_messages(msgs).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[1].role, "user");
    }
