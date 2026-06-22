    use super::*;

    #[test]
    fn extracts_text_from_message() {
        let msg = WeixinMessage {
            message_id: None,
            from_user_id: None,
            to_user_id: None,
            group_id: None,
            message_type: Some(1),
            message_state: None,
            item_list: Some(vec![MessageItem {
                r#type: Some(1),
                text_item: Some(TextItem {
                    text: Some("hello".into()),
                }),
            }]),
            context_token: None,
        };
        assert_eq!(extract_text(&msg).as_deref(), Some("hello"));
    }
