    use super::*;
    use crate::memory::{KnotStatus, OwnerId};

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
    fn baseline_only_preference_and_fact() {
        let knots = vec![
            sample_knot(1, KnotKind::Preference, "简体中文"),
            sample_knot(2, KnotKind::Task, "写测试"),
        ];
        let config = MemoryConfig::default();
        let selected = select_baseline_knots(&knots, &config, 0);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].kind, KnotKind::Preference);
    }

    #[test]
    fn search_matches_keywords() {
        let knots = vec![
            sample_knot(1, KnotKind::Decision, "M7 采用结绳记忆"),
            sample_knot(2, KnotKind::Procedure, "部署脚本"),
        ];
        let config = MemoryConfig::default();
        let hits = search_knots(&knots, &config, "结绳 M7", None, 10, 0);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, 1);
    }
