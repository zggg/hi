    use super::*;
    use crate::provider::{ChatMessage, ToolDefinition};

    #[test]
    fn base64url_decodes_jwt_segment() {
        // {"exp":123} 的 base64url
        let decoded = base64url_decode("eyJleHAiOjEyM30").unwrap();
        let v: Value = serde_json::from_slice(&decoded).unwrap();
        assert_eq!(v["exp"], 123);
    }

    #[test]
    fn build_input_maps_roles() {
        let req = AiRequest {
            model: "gpt-5.5".into(),
            messages: vec![
                ChatMessage {
                    role: Role::System,
                    content: "be brief".into(),
                    tool_calls: None,
                    tool_call_id: None,
                    reasoning_content: None,
                },
                ChatMessage {
                    role: Role::Assistant,
                    content: String::new(),
                    tool_calls: Some(vec![ToolCall {
                        id: "call_1".into(),
                        name: "read".into(),
                        arguments: "{}".into(),
                    }]),
                    tool_call_id: None,
                    reasoning_content: None,
                },
                ChatMessage {
                    role: Role::Tool,
                    content: "data".into(),
                    tool_calls: None,
                    tool_call_id: Some("call_1".into()),
                    reasoning_content: None,
                },
            ],
            tools: vec![],
        };
        let (instructions, input) = build_input(&req);
        assert_eq!(instructions, "be brief");
        assert_eq!(input[0]["type"], "function_call");
        assert_eq!(input[0]["call_id"], "call_1");
        assert_eq!(input[1]["type"], "function_call_output");
        assert_eq!(input[1]["output"], "data");
    }

    #[test]
    fn build_tools_flattens_function() {
        let req = AiRequest {
            model: "gpt-5.5".into(),
            messages: vec![],
            tools: vec![ToolDefinition {
                name: "read".into(),
                description: "read file".into(),
                parameters: json!({"type":"object"}),
            }],
        };
        let tools = build_tools(&req);
        assert_eq!(tools[0]["type"], "function");
        assert_eq!(tools[0]["name"], "read");
    }

    #[test]
    fn function_call_done_collected() {
        let mut state = StreamState::default();
        let event = json!({
            "type": "response.output_item.done",
            "item": {"type":"function_call","call_id":"c1","name":"read","arguments":"{\"p\":1}"}
        });
        handle_event(&mut state, &event, &None);
        assert_eq!(state.tool_calls.len(), 1);
        assert_eq!(state.tool_calls[0].name, "read");
        assert_eq!(state.tool_calls[0].id, "c1");
    }
