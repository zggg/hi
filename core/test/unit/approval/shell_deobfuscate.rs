    use super::*;

    #[test]
    fn deobfuscates_backslash_and_empty_quotes() {
        let (out, ok) = deobfuscate(r"r\m -rf /tmp/x");
        assert!(ok);
        assert_eq!(out, "rm -rf /tmp/x");
        let (out, ok) = deobfuscate("r''m -rf /tmp/x");
        assert!(ok);
        assert_eq!(out, "rm -rf /tmp/x");
    }

    #[test]
    fn strips_eval_wrapper() {
        let (out, ok) = deobfuscate("eval sudo apt install");
        assert!(ok);
        assert_eq!(out, "sudo apt install");
    }

    #[test]
    fn expands_ifs() {
        let (out, ok) = deobfuscate("curl${IFS}-s${IFS}http://x");
        assert!(ok);
        assert!(out.contains("curl -s http://x"));
    }

    #[test]
    fn decodes_ansi_c_quote() {
        let (out, ok) = deobfuscate("$'curl'");
        assert!(ok);
        assert_eq!(out, "curl");
    }

    #[test]
    fn preserves_utf8_inside_quotes() {
        let s = "echo \"# curl 权限弹窗 Bug\" > /tmp/bugs.md";
        let (out, ok) = normalize_with_status(s);
        assert!(ok);
        assert_eq!(out, s);
    }

    #[test]
    fn unwraps_command_substitution_inline() {
        let (out, ok) = deobfuscate("$(echo rm) -rf /tmp/x");
        assert!(ok);
        assert_eq!(out, "rm -rf /tmp/x");
    }
