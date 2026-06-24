use hi_core::llm::{ChatMessage, Role, ToolCall};
use hi_core::StoredMessage;

use crate::session::{format_message_line, truncate_preview};

fn stored(role: Role, content: &str) -> StoredMessage {
    StoredMessage {
        id: 42,
        in_context: true,
        message: ChatMessage {
            role,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        },
    }
}

#[test]
fn truncate_preview_respects_utf8_char_boundaries() {
    let text = "可以。我刚才实际探了一轮东方财富接口，以 **国电南瑞 600406.SH** 为样例。结论是：东方财富能抓到的数据相当多，足够写一个实时行情加技术指标加资金流加财务估值加新闻公告的股票分析预测脚本。";
    let preview = truncate_preview(text, 40);
    assert!(preview.ends_with('…'));
    assert_eq!(preview.chars().count(), 41);
    assert!(std::str::from_utf8(preview.as_bytes()).is_ok());
}

#[test]
fn truncate_preview_leaves_short_text_unchanged() {
    assert_eq!(truncate_preview("hi", 120), "hi");
}

#[test]
fn preview_mode_truncates_to_first_line() {
    let row = stored(Role::User, "line1\nline2");
    let out = format_message_line(&row, false);
    assert!(out.starts_with("#42    user: line1"));
    assert!(!out.contains("line2"));
}

#[test]
fn verbose_mode_shows_think_tool_calls_and_output() {
    let mut row = stored(Role::Assistant, "final answer");
    row.message.reasoning_content = Some("deep thought".into());
    row.message.tool_calls = Some(vec![ToolCall {
        id: "call_1".into(),
        name: "read".into(),
        arguments: r#"{"path":"a.txt"}"#.into(),
    }]);

    let out = format_message_line(&row, true);
    assert!(out.contains("--- think ---\ndeep thought\n"));
    assert!(out.contains("--- tool_calls ---\n· read {\"path\":\"a.txt\"}\n"));
    assert!(out.contains("--- content ---\nfinal answer\n"));
}

#[test]
fn verbose_mode_shows_full_tool_output_with_call_id() {
    let mut row = stored(Role::Tool, "line1\nline2\nline3");
    row.message.tool_call_id = Some("call_9".into());

    let out = format_message_line(&row, true);
    assert!(out.contains("#42    tool (call_id=call_9)\n"));
    assert!(out.contains("--- output ---\nline1\nline2\nline3\n"));
}
