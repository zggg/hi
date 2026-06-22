    use super::*;
    use std::fs;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hi-path-{name}-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn resolves_relative_path() {
        let ws = temp("rel");
        let file = ws.join("notes.txt");
        fs::write(&file, "ok").unwrap();
        let access = FileAccess {
            workspace: ws.clone(),
        };
        let resolved = resolve_path(&access, "notes.txt").unwrap();
        assert_eq!(resolved.path, file.canonicalize().unwrap());
    }

    #[test]
    fn resolves_absolute_path() {
        let ws = temp("abs-ws");
        let file = temp("abs-file").join("x.txt");
        fs::write(&file, "ok").unwrap();
        let access = FileAccess { workspace: ws };
        let resolved = resolve_path(&access, file.to_str().unwrap()).unwrap();
        assert_eq!(resolved.path, file.canonicalize().unwrap());
    }

    #[test]
    fn write_resolves_new_file_under_workspace() {
        let ws = temp("write-new");
        let access = FileAccess {
            workspace: ws.clone(),
        };
        let resolved = resolve_path_for_write(&access, "bugs.md").unwrap();
        assert_eq!(resolved.path, ws.canonicalize().unwrap().join("bugs.md"));
    }

    #[test]
    fn write_resolves_nested_new_file() {
        let ws = temp("write-nested");
        fs::create_dir_all(ws.join("notes")).unwrap();
        let access = FileAccess {
            workspace: ws.clone(),
        };
        let resolved = resolve_path_for_write(&access, "notes/new.txt").unwrap();
        assert_eq!(
            resolved.path,
            ws.join("notes").canonicalize().unwrap().join("new.txt")
        );
    }
