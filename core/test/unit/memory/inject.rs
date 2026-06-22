    use super::*;
    use crate::memory::{
        Knot, KnotConfidence, KnotKind, KnotStatus, KnotVisibility, OwnerId,
    };

    fn sample_knot(id: i64, kind: KnotKind, content: &str) -> Knot {
        Knot {
            id,
            owner_id: OwnerId("local".into()),
            kind,
            content: content.into(),
            status: KnotStatus::Active,
            task_status: None,
            confidence: KnotConfidence::Confirmed,
            clarity: 1.0,
            permanent: false,
            visibility: KnotVisibility::Inject,
            content_hash: format!("h{id}"),
            access_count: 0,
            last_accessed_at: None,
            created_at: 0,
            updated_at: id,
        }
    }

    #[test]
    fn keyword_filter_keeps_preference_without_match() {
        let knots = vec![
            sample_knot(1, KnotKind::Preference, "简体中文"),
            sample_knot(2, KnotKind::Procedure, "部署脚本"),
        ];
        let mut config = MemoryConfig::default();
        config.memory_search_enabled = false;
        let selected = select_knots(&knots, &config, Some("Rust 项目"), 0);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].kind, KnotKind::Preference);
    }

    #[test]
    fn baseline_mode_skips_procedure() {
        let knots = vec![
            sample_knot(1, KnotKind::Preference, "简体中文"),
            sample_knot(2, KnotKind::Procedure, "部署脚本"),
        ];
        let config = MemoryConfig::default();
        let result = build_injection(&knots, &config, None, 0);
        assert!(result.block.contains("简体中文"));
        assert!(!result.block.contains("部署脚本"));
        assert!(result.block.contains("memory_search"));
    }

    #[test]
    fn dream_not_injected_below_clarity() {
        let mut knot = sample_knot(1, KnotKind::Fact, "也许用 macOS");
        knot.confidence = KnotConfidence::Dream;
        knot.clarity = 0.4;
        let mut config = MemoryConfig::default();
        config.memory_search_enabled = false;
        let selected = select_knots(&[knot], &config, None, 0);
        assert!(selected.is_empty());
    }

    #[test]
    fn format_block_respects_max_chars() {
        let knots = vec![sample_knot(1, KnotKind::Fact, "a".repeat(500).as_str())];
        let block = format_baseline_block(&knots, 250);
        assert!(block.chars().count() <= 250);
    }
