    use super::*;
    use crate::memory::{KnotKind, KnotVisibility};
    use crate::store::SessionStore;

    fn temp_store(name: &str) -> SessionStore {
        let dir = std::env::temp_dir().join(format!("hi-knot-{name}-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let db = dir.join("sessions.db");
        let _ = std::fs::remove_file(&db);
        SessionStore::open(&db).unwrap()
    }

    #[test]
    fn add_list_forget_knot() {
        let store = temp_store("crud");
        let owner = OwnerId("local".into());
        let id = store
            .add_knot(&NewKnot {
                owner_id: owner.clone(),
                kind: KnotKind::Fact,
                content: "我叫 gz".into(),
                confidence: KnotConfidence::Confirmed,
                clarity: 1.0,
                permanent: true,
                visibility: KnotVisibility::Inject,
                task_status: None,
            })
            .unwrap();
        assert!(id > 0);

        let active = store.list_knots(&owner).unwrap();
        assert_eq!(active.len(), 1);

        store.forget_knot(id).unwrap();
        assert!(store.list_knots(&owner).unwrap().is_empty());
        assert_eq!(store.list_all_knots(&owner).unwrap().len(), 1);
    }

    #[test]
    fn dedup_by_content_hash() {
        let store = temp_store("dedup");
        let owner = OwnerId("local".into());
        let a = store
            .add_knot(&NewKnot {
                owner_id: owner.clone(),
                kind: KnotKind::Preference,
                content: "偏好  简体中文".into(),
                confidence: KnotConfidence::Inferred,
                clarity: 0.7,
                permanent: false,
                visibility: KnotVisibility::Inject,
                task_status: None,
            })
            .unwrap();
        let b = store
            .add_knot(&NewKnot {
                owner_id: owner.clone(),
                kind: KnotKind::Preference,
                content: "偏好 简体中文".into(),
                confidence: KnotConfidence::Inferred,
                clarity: 0.7,
                permanent: false,
                visibility: KnotVisibility::Inject,
                task_status: None,
            })
            .unwrap();
        assert_eq!(a, b);
        assert_eq!(store.list_knots(&owner).unwrap().len(), 1);
    }
