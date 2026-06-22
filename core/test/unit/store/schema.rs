    use super::*;
    use rusqlite::Connection;

    #[test]
    fn fresh_db_gets_current_schema() {
        let conn = Connection::open_in_memory().unwrap();
        ensure_compatible(&conn, ":memory:").unwrap();
        let version: i32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        assert!(has_current_shape(&conn).unwrap());
    }

    #[test]
    fn legacy_db_without_knots_is_rejected() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE messages (id INTEGER PRIMARY KEY, session_id TEXT, role TEXT, content TEXT);",
        )
        .unwrap();
        let err = ensure_compatible(&conn, "/tmp/old.db").unwrap_err();
        assert!(err.to_string().contains("不兼容"));
    }
