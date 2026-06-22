    use super::*;

    #[test]
    fn parses_tool_call_response() {
        let json = r#"{
            "choices": [{
                "message": {
                    "content": null,
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": { "name": "read", "arguments": "{}" }
                    }]
                }
            }]
        }"#;
        let parsed: ApiResponse = serde_json::from_str(json).unwrap();
        let msg = &parsed.choices[0].message;
        let calls = from_api_tool_calls(msg.tool_calls.clone());
        assert_eq!(calls[0].name, "read");
    }

    #[test]
    fn serializes_reasoning_content_in_assistant_history() {
        let msg = to_api_message(ChatMessage {
            role: Role::Assistant,
            content: "answer".into(),
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: Some("thinking".into()),
        });
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["reasoning_content"], "thinking");
        assert_eq!(json["content"], "answer");
    }
