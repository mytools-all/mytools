use super::*;

fn fixture() -> (tempfile::TempDir, Roots) {
    let dir = tempfile::tempdir().unwrap();
    let roots = Roots {
        config: dir.path().join("config"),
        data: dir.path().join("data"),
        plugins: dir.path().join("programs"),
    };
    fs::create_dir_all(&roots.config).unwrap();
    fs::create_dir_all(roots.data.join("plugin-files/one")).unwrap();
    fs::write(
        roots.config.join("settings.json"),
        br#"{"theme":"dark","pluginRoot":"machine-only"}"#,
    )
    .unwrap();
    fs::write(
        roots.config.join("plugin-store.json"),
        br#"{"one:todo":[1],"two:todo":[2]}"#,
    )
    .unwrap();
    fs::write(roots.config.join("plugin-state.json"), br#"["one"]"#).unwrap();
    fs::write(roots.data.join("plugin-files/one/note.txt"), "before").unwrap();
    (dir, roots)
}

#[test]
fn selected_restore_preserves_other_plugins_and_machine_settings() {
    let (dir, roots) = fixture();
    let archive = dir.path().join("selected.zip");
    let manifest = create(
        &roots,
        "0.2.0",
        &Selection {
            mode: "selected".into(),
            plugins: vec!["one".into()],
            include_programs: false,
        },
        &archive,
    )
    .unwrap();
    assert_eq!(manifest.plugins.len(), 1);
    fs::write(
        roots.config.join("plugin-store.json"),
        br#"{"one:todo":[8],"two:todo":[9]}"#,
    )
    .unwrap();
    fs::write(roots.data.join("plugin-files/one/note.txt"), "after").unwrap();
    let report = restore(
        &archive,
        &roots,
        "0.2.0",
        &RestoreOptions {
            host: false,
            clipboard: false,
            plugins: vec!["one".into()],
            programs: false,
        },
        &|_| Ok(()),
    )
    .unwrap();
    assert!(Path::new(&report.recovery_backup).exists());
    let values = object(&roots.config.join("plugin-store.json")).unwrap();
    assert_eq!(values["one:todo"], serde_json::json!([1]));
    assert_eq!(values["two:todo"], serde_json::json!([9]));
    assert_eq!(
        fs::read_to_string(roots.data.join("plugin-files/one/note.txt")).unwrap(),
        "before"
    );
    assert_eq!(
        object(&roots.config.join("settings.json")).unwrap()["pluginRoot"],
        "machine-only"
    );
    assert!(!roots.data.join("restore-journal.json").exists());
}

#[test]
fn host_restore_validation_failure_does_not_change_live_data() {
    let (dir, roots) = fixture();
    let archive = dir.path().join("host.zip");
    create(
        &roots,
        "0.2.0",
        &Selection {
            mode: "host".into(),
            plugins: vec![],
            include_programs: false,
        },
        &archive,
    )
    .unwrap();
    let before = fs::read(roots.config.join("settings.json")).unwrap();
    assert!(restore(
        &archive,
        &roots,
        "0.2.0",
        &RestoreOptions {
            host: true,
            clipboard: false,
            plugins: vec![],
            programs: false
        },
        &|_| Err("invalid".into())
    )
    .is_err());
    assert_eq!(
        fs::read(roots.config.join("settings.json")).unwrap(),
        before
    );
    assert!(inspect(&archive, "0.1.0").is_err());
}

#[test]
fn recovery_rolls_back_interrupted_replacement() {
    let (_dir, roots) = fixture();
    let workspace = roots.config.join(".mytools-restore-test");
    fs::create_dir(&workspace).unwrap();
    let destination = roots.config.join("settings.json");
    let before = fs::read(&destination).unwrap();
    fs::rename(&destination, workspace.join("previous")).unwrap();
    fs::write(&destination, "replacement").unwrap();
    write_json(
        &roots.data.join("restore-journal.json"),
        &Journal {
            version: 1,
            committed: false,
            plans: vec![Plan {
                target: "config/settings.json".into(),
                workspace: ".mytools-restore-test".into(),
                existed: true,
            }],
        },
    )
    .unwrap();
    recover(&roots).unwrap();
    assert_eq!(fs::read(destination).unwrap(), before);
    assert!(!workspace.exists());
}

#[test]
fn invalid_archive_paths_and_tampering_are_rejected() {
    let (dir, roots) = fixture();
    for path in [
        "../escape",
        "programs/one/CON.txt",
        "plugins/files/one/a:stream",
        "plugins/files/one/foo.",
        "programs/one/a\\b",
    ] {
        assert!(allowed(path).is_err());
    }
    let archive = dir.path().join("backup.zip");
    create(
        &roots,
        "0.2.0",
        &Selection {
            mode: "host".into(),
            plugins: vec![],
            include_programs: false,
        },
        &archive,
    )
    .unwrap();
    let mut source = zip::ZipArchive::new(fs::File::open(&archive).unwrap()).unwrap();
    let bad = dir.path().join("tampered.zip");
    let mut output = zip::ZipWriter::new(fs::File::create(&bad).unwrap());
    for i in 0..source.len() {
        let mut file = source.by_index(i).unwrap();
        output
            .start_file(file.name(), zip::write::SimpleFileOptions::default())
            .unwrap();
        if file.name() == "host/settings.json" {
            output.write_all(b"{}").unwrap();
        } else {
            std::io::copy(&mut file, &mut output).unwrap();
        }
    }
    output.finish().unwrap();
    assert!(inspect(&bad, "0.2.0").is_err());
}

#[test]
fn full_backup_restores_a_consistent_clipboard_snapshot() {
    let (dir, roots) = fixture();
    let database = roots.data.join("mytools.db");
    let connection = crate::clipboard::ClipboardStore::open(&database).unwrap();
    connection.push("fixture text").unwrap();
    let archive = dir.path().join("all.zip");
    create(
        &roots,
        "0.2.0",
        &Selection {
            mode: "all".into(),
            plugins: vec![],
            include_programs: false,
        },
        &archive,
    )
    .unwrap();
    drop(connection);
    fs::write(
        roots.config.join("settings.json"),
        br#"{"theme":"light","pluginRoot":"local-path"}"#,
    )
    .unwrap();
    restore(
        &archive,
        &roots,
        "0.2.0",
        &RestoreOptions {
            host: true,
            clipboard: true,
            plugins: vec![],
            programs: false,
        },
        &|_| Ok(()),
    )
    .unwrap();
    let settings = object(&roots.config.join("settings.json")).unwrap();
    assert_eq!(settings["theme"], "dark");
    assert_eq!(settings["pluginRoot"], "local-path");
    let connection = rusqlite::Connection::open(database).unwrap();
    let text: String = connection
        .query_row("SELECT content FROM clipboard LIMIT 1", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(text, "fixture text");
}

#[cfg(windows)]
#[test]
fn replacement_failure_rolls_back_already_replaced_files() {
    use std::os::windows::fs::OpenOptionsExt;
    let (dir, roots) = fixture();
    let archive = dir.path().join("selected.zip");
    create(
        &roots,
        "0.2.0",
        &Selection {
            mode: "selected".into(),
            plugins: vec!["one".into()],
            include_programs: false,
        },
        &archive,
    )
    .unwrap();
    fs::write(
        roots.config.join("plugin-store.json"),
        br#"{"one:todo":[99],"two:todo":[88]}"#,
    )
    .unwrap();
    let before = fs::read(roots.config.join("plugin-store.json")).unwrap();
    let lock = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(roots.config.join("plugin-state.json"))
        .unwrap();
    let result = restore(
        &archive,
        &roots,
        "0.2.0",
        &RestoreOptions {
            host: false,
            clipboard: false,
            plugins: vec!["one".into()],
            programs: false,
        },
        &|_| Ok(()),
    );
    assert!(result.unwrap_err().contains("已回滚"));
    assert_eq!(
        fs::read(roots.config.join("plugin-store.json")).unwrap(),
        before
    );
    assert!(!roots.data.join("restore-journal.json").exists());
    drop(lock);
}
