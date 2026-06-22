    use super::*;
    use crate::memory::KnotKind;
    use crate::store::SessionStore;

    fn temp_store(name: &str) -> SessionStore {
        let dir = std::env::temp_dir().join(format!("hi-merge-{name}-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let db = dir.join("sessions.db");
        let _ = std::fs::remove_file(&db);
        SessionStore::open(&db).unwrap()
    }

    #[test]
    fn merge_adds_and_dedups() {
        let store = temp_store("dedup");
        let owner = OwnerId("local".into());
        store.ensure_memory_owner(&owner).unwrap();

        let extracted = vec![ExtractedKnot {
            kind: KnotKind::Fact,
            content: "代号 gz".into(),
            confidence: KnotConfidence::Confirmed,
            task_status: None,
            supersedes_content_hash: None,
        }];
        let prov = KnotProvenance::default();
        let a = merge_extracted(&store, &owner, &extracted, &prov).unwrap();
        assert_eq!(a.added, 1);

        let b = merge_extracted(&store, &owner, &extracted, &prov).unwrap();
        assert_eq!(b.skipped, 1);
        assert_eq!(store.list_knots(&owner).unwrap().len(), 1);
    }
