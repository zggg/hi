    use super::*;
    use crate::turn::{Notice, ToolBlock, ToolPhase};

    fn text_of(lines: &[Line<'static>]) -> String {
        lines
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn user_and_pieces_render_with_marks() {
        let turn = Turn {
            user: "hi".into(),
            ..Turn::default()
        };
        let user = text_of(&render_user(&turn, 80));
        assert!(user.contains('>'));

        let tool = ToolBlock {
            name: "bash".into(),
            arguments: r#"{"command":"ls ~"}"#.into(),
            phase: ToolPhase::Done(true),
            output: String::new(),
            ..ToolBlock::default()
        };
        let tool = text_of(&[tool_line(&tool, 80)]);
        assert!(tool.contains('✓'));
        assert!(tool.contains("bash"));
        assert!(tool.contains("ls ~"));

        let reply = text_of(&reply_logical_line("done", 80, true));
        assert!(reply.contains('●'));
        assert!(reply.contains("done"));
    }

    #[test]
    fn thinking_summary_line() {
        let summary = thinking_summary("hello world");
        let text: String = summary
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert!(text.contains("think"));
    }

    #[test]
    fn thinking_preview_shows_tail() {
        let preview = thinking_preview("and the session runner.", 80);
        let text: String = preview
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert!(text.contains("session runner"));
    }

    #[test]
    fn tool_preview_shows_header_and_streaming_output() {
        let running = ToolBlock {
            name: "bash".into(),
            arguments: r#"{"command":"cargo test"}"#.into(),
            phase: ToolPhase::Running,
            output: String::new(),
            ..ToolBlock::default()
        };
        let waiting = tool_preview_lines(&running, 80, 2);
        assert_eq!(waiting.len(), 2);

        let mut running = running;
        running.output = "running 3 tests\nok".into();
        let streaming = tool_preview_lines(&running, 80, 2);
        assert_eq!(streaming.len(), 2);
        let text: String = streaming
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(text.contains("bash"));
        assert!(text.contains("ok"));
    }

    #[test]
    fn error_notice_wraps_on_narrow_terminal() {
        let msg = "大模型 API 请求失败 (HTTP 429): The usage limit has been reached请求过于频繁，请稍后再试。";
        let lines = render_notice(&Notice::Error(msg.into()), 40);
        assert!(
            lines.len() > 1,
            "expected wrap, got {} line(s)",
            lines.len()
        );
        let text = text_of(&lines);
        assert!(text.contains('⚠'));
        assert!(text.contains("HTTP 429"));
        assert!(text.contains("请稍后再试"));
    }

    #[test]
    fn verbose_thinking_body_keeps_full_text() {
        let lines = thinking_body_lines("第一行思考内容\n继续推理细节", 80);
        let text = text_of(&lines);
        assert!(text.contains("第一行思考内容"));
        assert!(text.contains("继续推理细节"));
    }

    #[test]
    fn verbose_tool_header_and_status() {
        let tool = ToolBlock {
            name: "bash".into(),
            arguments: r#"{"command":"cargo test"}"#.into(),
            phase: ToolPhase::Done(true),
            output: String::new(),
            ..ToolBlock::default()
        };
        let header = text_of(&[tool_header_line(&tool, 80)]);
        assert!(header.contains("bash"));
        assert!(header.contains("cargo test"));
        assert!(header.contains('·'));
        let ok = text_of(&[tool_status_line(true)]);
        assert!(ok.contains('✓'));
        let fail = text_of(&[tool_status_line(false)]);
        assert!(fail.contains('✗'));
    }

    #[test]
    fn verbose_tool_body_not_truncated() {
        let long = "x".repeat(500);
        let lines = tool_body_lines(&long, 40);
        let text: String = text_of(&lines).chars().filter(|c| *c == 'x').collect();
        assert_eq!(text.chars().count(), 500, "verbose 展开应保留全文，不截断");
    }

    #[test]
    fn tool_line_shows_name_and_command() {
        let running = ToolBlock {
            name: "bash".into(),
            arguments: r#"{"command":"cargo test"}"#.into(),
            phase: ToolPhase::Running,
            output: String::new(),
            ..ToolBlock::default()
        };
        let line = tool_line(&running, 80);
        let text: String = line
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert!(text.contains("bash"));
        assert!(text.contains("cargo test"));
    }
