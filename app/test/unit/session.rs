    use super::*;

    #[test]
    fn truncate_preview_respects_utf8_char_boundaries() {
        let text = "可以。我刚才实际探了一轮东方财富接口，以 **国电南瑞 600406.SH** 为样例。结论是：东方财富能抓到的数据相当多，足够写一个实时行情加技术指标加资金流加财务估值加新闻公告的股票分析预测脚本。";
        let preview = truncate_preview(text, 40);
        assert!(preview.ends_with('…'));
        assert_eq!(preview.chars().count(), 41);
        // Must not panic on re-parse (would fail if sliced inside a multibyte char).
        assert!(std::str::from_utf8(preview.as_bytes()).is_ok());
    }

    #[test]
    fn truncate_preview_leaves_short_text_unchanged() {
        assert_eq!(truncate_preview("hi", 120), "hi");
    }
