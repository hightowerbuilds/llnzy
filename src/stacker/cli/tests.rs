use super::*;
    use crate::stacker::storage;

    fn temp_root(name: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("llnzy-cli-{name}-{nonce}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn paths_in(root: &Path) -> (PathBuf, PathBuf) {
        (root.join("inbox"), root.join(".tmp"))
    }

    fn invoke(args: &[&str], stdin: &[u8], inbox: &Path, tmp: &Path) -> (i32, String, String) {
        let owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let mut stdin_cursor = stdin;
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let root = inbox.parent().expect("inbox should live under test root");
        let saved_dir = root.join("saved");
        let archive_dir = root.join("archive");
        let legacy_path = root.join("stacker.json");
        let queue_path = root.join("stacker_queue.json");
        let code = run(
            &owned,
            &mut stdin_cursor,
            &mut stdout,
            &mut stderr,
            CliPaths {
                inbox_dir: inbox,
                saved_dir: &saved_dir,
                archive_dir: &archive_dir,
                tmp_dir: tmp,
                legacy_path: &legacy_path,
                queue_path: &queue_path,
            },
        );
        (
            code,
            String::from_utf8_lossy(&stdout).into_owned(),
            String::from_utf8_lossy(&stderr).into_owned(),
        )
    }

    #[test]
    fn missing_label_exits_usage() {
        let root = temp_root("missing-label");
        let (inbox, tmp) = paths_in(&root);
        let (code, _out, err) = invoke(&["prompt", "add"], b"hello", &inbox, &tmp);
        assert_eq!(code, EXIT_USAGE);
        assert!(err.contains("--label"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn unknown_flag_exits_usage() {
        let root = temp_root("unknown-flag");
        let (inbox, tmp) = paths_in(&root);
        let (code, _out, err) = invoke(
            &["prompt", "add", "--label", "x", "--bogus", "y"],
            b"hello",
            &inbox,
            &tmp,
        );
        assert_eq!(code, EXIT_USAGE);
        assert!(err.contains("--bogus"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn empty_body_exits_input_error() {
        let root = temp_root("empty-body");
        let (inbox, tmp) = paths_in(&root);
        let (code, _out, err) = invoke(&["prompt", "add", "--label", "x"], b"   \n", &inbox, &tmp);
        assert_eq!(code, EXIT_INPUT);
        assert!(err.contains("empty"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn oversize_stdin_body_exits_input_error() {
        let root = temp_root("big-body");
        let (inbox, tmp) = paths_in(&root);
        let big = vec![b'a'; BODY_MAX_BYTES + 1];
        let (code, _out, err) = invoke(&["prompt", "add", "--label", "big"], &big, &inbox, &tmp);
        assert_eq!(code, EXIT_INPUT);
        assert!(err.contains("byte limit"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn non_utf8_stdin_body_exits_input_error() {
        let root = temp_root("non-utf8");
        let (inbox, tmp) = paths_in(&root);
        let invalid: Vec<u8> = vec![0xff, 0xfe, 0xfd];
        let (code, _out, err) = invoke(&["prompt", "add", "--label", "x"], &invalid, &inbox, &tmp);
        assert_eq!(code, EXIT_INPUT);
        assert!(err.contains("UTF-8"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn happy_path_writes_pending_prompt_to_inbox() {
        let root = temp_root("happy");
        let (inbox, tmp) = paths_in(&root);
        let (code, out, err) = invoke(
            &[
                "prompt",
                "add",
                "--label",
                "Refactor LSP transport",
                "--category",
                "lsp",
                "--source-agent",
                "claude-code",
                "--session",
                "abc",
                "--workspace",
                "llnzy",
            ],
            b"Body of the prompt.\n",
            &inbox,
            &tmp,
        );
        assert_eq!(code, EXIT_OK, "stderr: {err}");
        let trimmed = out.trim();
        assert!(trimmed.ends_with(".md"), "unexpected stdout: {out}");

        let records = storage::list(&inbox).unwrap();
        assert_eq!(records.len(), 1);
        let record = &records[0];
        assert_eq!(record.frontmatter.state, PromptState::Pending);
        assert_eq!(record.frontmatter.label, "Refactor LSP transport");
        assert_eq!(record.frontmatter.category, "lsp");
        assert_eq!(
            record.frontmatter.source_agent.as_deref(),
            Some("claude-code")
        );
        assert_eq!(record.frontmatter.session_id.as_deref(), Some("abc"));
        assert_eq!(record.frontmatter.workspace.as_deref(), Some("llnzy"));
        assert_eq!(record.body, "Body of the prompt.\n");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn file_source_reads_body_from_disk() {
        let root = temp_root("file-source");
        let (inbox, tmp) = paths_in(&root);
        let body_path = root.join("body.md");
        fs::write(&body_path, "from a file").unwrap();
        let (code, _out, err) = invoke(
            &[
                "prompt",
                "add",
                "--label",
                "from-file",
                "--file",
                body_path.to_str().unwrap(),
            ],
            &[],
            &inbox,
            &tmp,
        );
        assert_eq!(code, EXIT_OK, "stderr: {err}");
        let records = storage::list(&inbox).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].body, "from a file");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn file_source_rejects_directory() {
        let root = temp_root("file-dir");
        let (inbox, tmp) = paths_in(&root);
        let dir = root.join("body-dir");
        fs::create_dir_all(&dir).unwrap();
        let (code, _out, err) = invoke(
            &[
                "prompt",
                "add",
                "--label",
                "from-dir",
                "--file",
                dir.to_str().unwrap(),
            ],
            &[],
            &inbox,
            &tmp,
        );
        assert_eq!(code, EXIT_INPUT);
        assert!(err.contains("regular file"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn file_source_rejects_missing_path() {
        let root = temp_root("file-missing");
        let (inbox, tmp) = paths_in(&root);
        let missing = root.join("nope.md");
        let (code, _out, err) = invoke(
            &[
                "prompt",
                "add",
                "--label",
                "x",
                "--file",
                missing.to_str().unwrap(),
            ],
            &[],
            &inbox,
            &tmp,
        );
        assert_eq!(code, EXIT_INPUT);
        assert!(err.contains("cannot resolve"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn file_source_rejects_oversize_payload() {
        let root = temp_root("file-big");
        let (inbox, tmp) = paths_in(&root);
        let big_path = root.join("big.md");
        fs::write(&big_path, vec![b'a'; BODY_MAX_BYTES + 1]).unwrap();
        let (code, _out, err) = invoke(
            &[
                "prompt",
                "add",
                "--label",
                "x",
                "--file",
                big_path.to_str().unwrap(),
            ],
            &[],
            &inbox,
            &tmp,
        );
        assert_eq!(code, EXIT_INPUT);
        assert!(err.contains("byte limit"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn long_label_is_truncated_and_control_chars_stripped() {
        let root = temp_root("label-sanitize");
        let (inbox, tmp) = paths_in(&root);
        let label = format!("\nbeg\t{}\rend", "x".repeat(LABEL_MAX_CHARS + 50));
        let (code, _out, _err) =
            invoke(&["prompt", "add", "--label", &label], b"body", &inbox, &tmp);
        assert_eq!(code, EXIT_OK);
        let records = storage::list(&inbox).unwrap();
        assert_eq!(records.len(), 1);
        let stored = &records[0].frontmatter.label;
        assert_eq!(stored.chars().count(), LABEL_MAX_CHARS);
        assert!(!stored.contains('\n'));
        assert!(!stored.contains('\t'));
        assert!(!stored.contains('\r'));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn label_of_only_control_chars_exits_input_error() {
        let root = temp_root("label-empty");
        let (inbox, tmp) = paths_in(&root);
        let (code, _out, err) = invoke(
            &["prompt", "add", "--label", "\n\t\r"],
            b"body",
            &inbox,
            &tmp,
        );
        assert_eq!(code, EXIT_INPUT);
        assert!(err.contains("--label"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn stacker_add_saves_prompt_to_saved_library() {
        let root = temp_root("save");
        let (inbox, tmp) = paths_in(&root);
        let saved = root.join("saved");
        let (code, out, err) = invoke(
            &[
                "stacker",
                "add",
                "--label",
                "Release Checklist",
                "--category",
                "ship",
                "--workspace",
                "llnzy",
            ],
            b"Run the release checklist.\n",
            &inbox,
            &tmp,
        );
        assert_eq!(code, EXIT_OK, "stderr: {err}");
        assert!(out.trim().ends_with(".md"));

        let saved_records = storage::list(&saved).unwrap();
        assert_eq!(saved_records.len(), 1);
        let record = &saved_records[0];
        assert_eq!(record.frontmatter.state, PromptState::Saved);
        assert_eq!(record.frontmatter.label, "Release Checklist");
        assert_eq!(record.frontmatter.category, "ship");
        assert_eq!(record.frontmatter.workspace.as_deref(), Some("llnzy"));
        assert_eq!(record.frontmatter.source_agent, None);
        assert_eq!(record.frontmatter.session_id, None);
        assert_eq!(record.body, "Run the release checklist.\n");
        assert!(storage::list(&inbox).unwrap().is_empty());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn list_saved_prompts_outputs_json() {
        let root = temp_root("list-json");
        let (inbox, tmp) = paths_in(&root);
        let (code, _out, err) = invoke(
            &["stacker", "save", "--label", "One", "--body", "First body"],
            &[],
            &inbox,
            &tmp,
        );
        assert_eq!(code, EXIT_OK, "stderr: {err}");

        let (code, out, err) = invoke(&["stacker", "list", "--format", "json"], &[], &inbox, &tmp);
        assert_eq!(code, EXIT_OK, "stderr: {err}");
        let value: serde_json::Value = serde_json::from_str(&out).unwrap();
        let records = value.as_array().expect("list should be a JSON array");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["state"], "saved");
        assert_eq!(records[0]["label"], "One");
        assert_eq!(records[0]["body"], "First body");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn edit_saved_prompt_updates_record_and_queue_entry() {
        let root = temp_root("edit-saved");
        let (inbox, tmp) = paths_in(&root);
        let saved = root.join("saved");
        let queue_path = root.join("stacker_queue.json");
        let (code, _out, err) = invoke(
            &[
                "stacker",
                "save",
                "--label",
                "Original",
                "--body",
                "Original body",
            ],
            &[],
            &inbox,
            &tmp,
        );
        assert_eq!(code, EXIT_OK, "stderr: {err}");
        let id = storage::list(&saved).unwrap()[0].frontmatter.id.clone();
        save_queue_to_path(
            &[crate::stacker::queue::QueuedPrompt {
                text: "Original body".to_string(),
                label: "Original".to_string(),
            }],
            &queue_path,
        )
        .unwrap();

        let (code, _out, err) = invoke(
            &[
                "stacker",
                "edit",
                &id,
                "--label",
                "Edited",
                "--category",
                "ops",
                "--body",
                "Edited body",
            ],
            &[],
            &inbox,
            &tmp,
        );
        assert_eq!(code, EXIT_OK, "stderr: {err}");
        let record = storage::read(&saved.join(format!("{id}.md"))).unwrap();
        assert_eq!(record.frontmatter.state, PromptState::Saved);
        assert_eq!(record.frontmatter.label, "Edited");
        assert_eq!(record.frontmatter.category, "ops");
        assert_eq!(record.frontmatter.body_hash, body_hash("Edited body"));
        assert_eq!(record.body, "Edited body");

        let queued = load_queue_from_path(&queue_path).unwrap();
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0].text, "Edited body");
        assert_eq!(queued[0].label, "Edited");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn delete_saved_prompt_archives_record_and_removes_queue_entry() {
        let root = temp_root("delete-saved");
        let (inbox, tmp) = paths_in(&root);
        let saved = root.join("saved");
        let archive = root.join("archive");
        let queue_path = root.join("stacker_queue.json");
        let (code, _out, err) = invoke(
            &[
                "stacker",
                "save",
                "--label",
                "Remove Me",
                "--body",
                "Queued body",
            ],
            &[],
            &inbox,
            &tmp,
        );
        assert_eq!(code, EXIT_OK, "stderr: {err}");
        let id = storage::list(&saved).unwrap()[0].frontmatter.id.clone();
        save_queue_to_path(
            &[crate::stacker::queue::QueuedPrompt {
                text: "Queued body".to_string(),
                label: "Remove Me".to_string(),
            }],
            &queue_path,
        )
        .unwrap();

        let (code, out, err) = invoke(&["stacker", "delete", &id], &[], &inbox, &tmp);
        assert_eq!(code, EXIT_OK, "stderr: {err}");
        assert!(out.trim().ends_with(".md"));
        assert!(!saved.join(format!("{id}.md")).exists());
        let archived = storage::read(&archive.join(format!("{id}.md"))).unwrap();
        assert_eq!(archived.frontmatter.state, PromptState::Archived);
        assert_eq!(archived.body, "Queued body");
        assert!(load_queue_from_path(&queue_path).unwrap().is_empty());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn invalid_prompt_id_exits_usage_before_path_lookup() {
        let root = temp_root("invalid-id");
        let (inbox, tmp) = paths_in(&root);
        let (code, _out, err) = invoke(&["stacker", "delete", "../../x"], &[], &inbox, &tmp);
        assert_eq!(code, EXIT_USAGE);
        assert!(err.contains("invalid prompt id"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn quota_blocks_when_inbox_at_file_limit() {
        let root = temp_root("quota-files");
        let (inbox, tmp) = paths_in(&root);
        fs::create_dir_all(&inbox).unwrap();
        // Stuff the inbox with placeholder .md files up to the file quota.
        for _ in 0..INBOX_QUOTA_FILES {
            let id = storage::new_id();
            fs::write(inbox.join(format!("{id}.md")), b"").unwrap();
        }
        let (code, _out, err) = invoke(&["prompt", "add", "--label", "x"], b"body", &inbox, &tmp);
        assert_eq!(code, EXIT_QUOTA);
        assert!(err.contains("quota"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn parallel_calls_produce_distinct_ids() {
        // Sequential proxy for the v2 validation case: two adds in a row
        // must land as two distinct files (ULIDs are time-ordered + random).
        let root = temp_root("parallel");
        let (inbox, tmp) = paths_in(&root);
        for label in ["one", "two"] {
            let (code, _out, err) =
                invoke(&["prompt", "add", "--label", label], b"body", &inbox, &tmp);
            assert_eq!(code, EXIT_OK, "stderr: {err}");
        }
        let records = storage::list(&inbox).unwrap();
        assert_eq!(records.len(), 2);
        assert_ne!(records[0].frontmatter.id, records[1].frontmatter.id);
        fs::remove_dir_all(&root).unwrap();
    }
