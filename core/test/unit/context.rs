    use super::*;

    fn user_msg(content: &str) -> ChatMessage {
        ChatMessage {
            role: Role::User,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        }
    }

    fn tool_msg(content: &str) -> ChatMessage {
        ChatMessage {
            role: Role::Tool,
            content: content.into(),
            tool_calls: None,
            tool_call_id: Some("t1".into()),
            reasoning_content: None,
        }
    }

    #[test]
    fn estimate_tokens_counts_reasoning() {
        let msgs = vec![ChatMessage {
            role: Role::Assistant,
            content: String::new(),
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: Some("think".repeat(40)),
        }];
        assert!(estimate_tokens(&msgs) >= 10);
    }

    #[test]
    fn emergency_trim_single_turn_with_huge_tools() {
        let mut history = vec![
            ChatMessage {
                role: Role::System,
                content: "sys".into(),
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            },
            user_msg("analyze"),
            tool_msg(&"x".repeat(80_000)),
            tool_msg(&"y".repeat(80_000)),
        ];
        let config = ContextConfig {
            compress_at_k: 4,
            protect_tail_k: 2,
            ..ContextConfig::default()
        };
        let outcome = emergency_trim_history(&mut history, &config).expect("should trim");
        assert!(outcome.messages_trimmed > 0);
        assert!(estimate_tokens(&history) < outcome.tokens_before);
    }

    #[test]
    fn emergency_trim_stops_when_already_minimized() {
        // 回归：消息已截断到下限仍超预算时，不得反复空转（曾导致 heal_loaded_context 死循环）。
        let suffix = TRUNC_SUFFIX;
        let minimized = format!("{}{suffix}", "z".repeat(256));
        let mut history = vec![ChatMessage {
            role: Role::System,
            content: "sys".into(),
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        }];
        for _ in 0..40 {
            history.push(tool_msg(&minimized));
        }
        let config = ContextConfig {
            compress_at_k: 1,
            protect_tail_k: 1,
            trim_keep_chars: 256,
            ..ContextConfig::default()
        };
        assert!(over_context_budget(&history, &config));
        // 已无可缩短内容 → 返回 None，调用方据此停止循环。
        assert!(emergency_trim_history(&mut history, &config).is_none());
        assert!(longest_truncatable_message(&history, 256).is_none());
    }

    #[test]
    fn compression_split_index_isolates_last_user_turn() {
        let history = vec![
            ChatMessage {
                role: Role::System,
                content: "sys".into(),
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            },
            user_msg("old"),
            ChatMessage {
                role: Role::Assistant,
                content: "a".into(),
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            },
            user_msg("recent"),
        ];
        assert_eq!(compression_split_index(&history, &ContextConfig::default()), 3);
    }

    #[test]
    fn tail_split_respects_token_budget() {
        let history = vec![
            ChatMessage {
                role: Role::System,
                content: "sys".into(),
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            },
            user_msg(&"o".repeat(4000)),
            user_msg(&"r".repeat(4000)),
        ];
        // Each ~1000 tokens; protect 1500 → keep only last message
        assert_eq!(tail_split_index(&history, 1500), 2);
        // Protect 2500 → keep both user messages
        assert_eq!(tail_split_index(&history, 2500), 1);
    }

    #[test]
    fn apply_compression_trim_drops_middle() {
        let mut history = vec![
            ChatMessage {
                role: Role::System,
                content: "sys".into(),
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            },
            user_msg("old"),
            user_msg("recent"),
        ];
        apply_compression_trim(&mut history, 2);
        assert_eq!(history.len(), 2);
        assert_eq!(history[1].content, "recent");
    }
