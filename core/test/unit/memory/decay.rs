    use super::*;

    #[test]
    fn decay_halves_at_half_life() {
        let half = 30.0;
        let now = 30 * 86_400;
        let c = decay_clarity(1.0, 0, now, half);
        assert!((c - 0.5).abs() < 0.01);
    }

    #[test]
    fn permanent_skips_decay() {
        use crate::memory::{
            Knot, KnotConfidence, KnotKind, KnotStatus, KnotVisibility, OwnerId,
        };
        let knot = Knot {
            id: 1,
            owner_id: OwnerId("local".into()),
            kind: KnotKind::Fact,
            content: "x".into(),
            status: KnotStatus::Active,
            task_status: None,
            confidence: KnotConfidence::Confirmed,
            clarity: 0.8,
            permanent: true,
            visibility: KnotVisibility::Inject,
            content_hash: "h".into(),
            access_count: 0,
            last_accessed_at: None,
            created_at: 0,
            updated_at: 0,
        };
        let config = MemoryConfig {
            decay_enabled: true,
            ..MemoryConfig::default()
        };
        assert_eq!(effective_clarity(&knot, &config, 1_000_000), 0.8);
    }
