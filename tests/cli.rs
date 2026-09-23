use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

fn memo(cwd: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_memo"))
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("data"))
        .env("XDG_CONFIG_HOME", home.join("config"))
        .output()
        .unwrap()
}

fn stdout(output: Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn stderr(output: Output) -> String {
    assert!(!output.status.success());
    String::from_utf8(output.stderr).unwrap()
}

fn default_args<'a>(extra: &'a [&'a str]) -> Vec<&'a str> {
    let mut args = vec!["--store", "default"];
    args.extend_from_slice(extra);
    args
}

#[test]
fn where_is_read_only_and_init_is_idempotent() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let repo = fixture.path().join("repo");
    fs::create_dir_all(repo.join(".jj/repo")).unwrap();

    let report = stdout(memo(&repo, &home, &["where"]));
    assert!(report.contains("kind: project"));
    assert!(report.contains("initialized: false"));
    assert!(!home.join("data").exists());

    let initialized = stdout(memo(&repo, &home, &["init"]));
    let path = initialized
        .lines()
        .find_map(|line| line.strip_prefix("path: "))
        .unwrap();
    assert_eq!(
        fs::read_to_string(Path::new(path).join("FORMAT_VERSION")).unwrap(),
        "1\n"
    );
    stdout(memo(&repo, &home, &["init"]));
    assert_eq!(
        fs::read_to_string(Path::new(path).join("FORMAT_VERSION")).unwrap(),
        "1\n"
    );
    assert!(!home.join("data/memo/stores/default").exists());
}

#[test]
fn malformed_format_markers_fail_where_and_init() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let cwd = fixture.path().join("cwd");
    let store = home.join("data/memo/stores/default");
    fs::create_dir_all(&cwd).unwrap();
    fs::create_dir_all(&store).unwrap();

    for contents in ["2\n", "1", "garbage\n"] {
        fs::write(store.join("FORMAT_VERSION"), contents).unwrap();
        for command in ["where", "init"] {
            let error = stderr(memo(&cwd, &home, &["--store", "default", command]));
            assert!(error.contains("unsupported or malformed format marker"));
        }
    }

    fs::remove_file(store.join("FORMAT_VERSION")).unwrap();
    fs::create_dir(store.join("FORMAT_VERSION")).unwrap();
    for command in ["where", "init"] {
        let error = stderr(memo(&cwd, &home, &["--store", "default", command]));
        assert!(error.contains("format marker is not a regular file"));
    }
}

#[test]
fn unsafe_environment_roots_do_not_resolve_in_cwd() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let cwd = fixture.path().join("cwd");
    fs::create_dir_all(&cwd).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_memo"))
        .args(["--store", "default", "init"])
        .current_dir(&cwd)
        .env_clear()
        .env("HOME", &home)
        .env("XDG_DATA_HOME", "relative-data")
        .env("XDG_CONFIG_HOME", "")
        .output()
        .unwrap();
    stdout(output);
    assert!(
        home.join(".local/share/memo/stores/default/FORMAT_VERSION")
            .is_file()
    );
    assert!(!cwd.join("relative-data").exists());

    fs::create_dir(cwd.join(".git")).unwrap();
    fs::create_dir_all(cwd.join("relative-config/memo")).unwrap();
    fs::write(
        cwd.join("relative-config/memo/config.toml"),
        "auto_project = false\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_memo"))
        .arg("where")
        .current_dir(&cwd)
        .env_clear()
        .env("HOME", &home)
        .env("XDG_DATA_HOME", "")
        .env("XDG_CONFIG_HOME", "relative-config")
        .output()
        .unwrap();
    let report = stdout(output);
    assert!(report.contains("kind: project"));
    assert!(report.contains(&format!("path: {}/.local/share/", home.display())));

    let output = Command::new(env!("CARGO_BIN_EXE_memo"))
        .args(["--store", "default", "init"])
        .current_dir(&cwd)
        .env_clear()
        .env("HOME", &home)
        .env("MEMO_DATA_DIR", "")
        .output()
        .unwrap();
    let error = stderr(output);
    assert!(error.contains("MEMO_DATA_DIR"));
    assert!(!cwd.join("default").exists());
}

#[test]
fn config_and_flags_control_auto_selection() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let repo = fixture.path().join("repo");
    fs::create_dir_all(repo.join(".git")).unwrap();
    fs::create_dir_all(home.join("config/memo")).unwrap();
    fs::write(
        home.join("config/memo/config.toml"),
        "auto_project = false\n",
    )
    .unwrap();

    assert!(stdout(memo(&repo, &home, &[])).contains("kind: default"));
    assert!(stdout(memo(&repo, &home, &["--auto-project"])).contains("kind: project"));
    fs::write(
        home.join("config/memo/config.toml"),
        "auto_project = true\n",
    )
    .unwrap();
    assert!(stdout(memo(&repo, &home, &["--no-auto-project"])).contains("kind: default"));
    assert!(stdout(memo(&repo, &home, &["--store", "default"])).contains("kind: default"));
}

#[test]
fn overrides_data_root_and_initializes_only_explicit_default() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let repo = fixture.path().join("repo");
    let custom = fixture.path().join("custom");
    fs::create_dir_all(repo.join(".git")).unwrap();
    let report = stdout(memo(
        &repo,
        &home,
        &[
            "--data-dir",
            custom.to_str().unwrap(),
            "--store",
            "default",
            "init",
        ],
    ));
    assert!(report.contains(&format!("path: {}/default", custom.display())));
    assert!(custom.join("default/FORMAT_VERSION").is_file());
    assert!(!custom.join("projects").exists());
}

#[test]
fn note_nap_wake_roundtrip_and_scoped_pending_prompt() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let cwd = fixture.path();
    stdout(memo(cwd, &home, &default_args(&["init"])));
    stdout(memo(cwd, &home, &default_args(&["note", "first"])));
    stdout(memo(cwd, &home, &default_args(&["note", "second"])));

    let pending = stderr(memo(cwd, &home, &default_args(&["wake", "--lines", "1"])));
    assert!(pending.contains("wake incomplete"));
    let project_independent =
        String::from_utf8(memo(cwd, &home, &default_args(&["nap"])).stdout).unwrap();
    assert!(project_independent.contains("memo --store default nap 0-1"));

    stdout(memo(
        cwd,
        &home,
        &default_args(&["nap", "0-1", "both memories"]),
    ));
    let wake = stdout(memo(cwd, &home, &default_args(&["wake", "--lines", "1"])));
    assert!(wake.contains("0-1: both memories"));
    assert!(wake.contains("wake complete:"));
}

#[test]
fn text_limits_and_uninitialized_store_are_strict() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let cwd = fixture.path();
    let store = home.join("data/memo/stores/default");
    assert!(stderr(memo(cwd, &home, &default_args(&["note", "x"]))).contains("not initialized"));
    assert!(!store.exists());
    stdout(memo(cwd, &home, &default_args(&["init"])));
    for bad in ["", "two\nlines", "two\rlines"] {
        assert!(
            stderr(memo(cwd, &home, &default_args(&["note", bad]))).contains("one nonempty line")
        );
    }
    let exact = "é".repeat(140);
    stdout(memo(cwd, &home, &default_args(&["note", &exact])));
    stdout(memo(cwd, &home, &default_args(&["note", "second"])));
    stdout(memo(cwd, &home, &default_args(&["nap", "0-1", &exact])));
    let too_long = format!("{exact}x");
    assert!(
        stderr(memo(cwd, &home, &default_args(&["note", &too_long]))).contains("280 UTF-8 bytes")
    );
    assert!(
        stderr(memo(cwd, &home, &default_args(&["nap", "0-1", &too_long])))
            .contains("280 UTF-8 bytes")
    );
}

#[test]
fn concurrent_notes_are_fixed_width_contiguous_and_torn_suffix_repairs() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let cwd = fixture.path();
    stdout(memo(cwd, &home, &default_args(&["init"])));
    let children: Vec<_> = (0..12)
        .map(|i| {
            Command::new(env!("CARGO_BIN_EXE_memo"))
                .args(["--store", "default", "note", &format!("note {i}")])
                .current_dir(cwd)
                .env_clear()
                .env("HOME", &home)
                .env("XDG_DATA_HOME", home.join("data"))
                .env("XDG_CONFIG_HOME", home.join("config"))
                .spawn()
                .unwrap()
        })
        .collect();
    for mut child in children {
        assert!(child.wait().unwrap().success());
    }
    let log = home.join("data/memo/stores/default/notes.log");
    let bytes = fs::read(&log).unwrap();
    assert_eq!(bytes.len(), 12 * 320);
    for (id, slot) in bytes.as_chunks::<320>().0.iter().enumerate() {
        assert!(
            std::str::from_utf8(slot)
                .unwrap()
                .starts_with(&format!("N {id:010} "))
        );
    }
    fs::OpenOptions::new()
        .append(true)
        .open(&log)
        .unwrap()
        .write_all(b"torn")
        .unwrap();
    stdout(memo(cwd, &home, &default_args(&["note", "after repair"])));
    let bytes = fs::read(&log).unwrap();
    assert_eq!(bytes.len(), 13 * 320);
    assert!(
        std::str::from_utf8(&bytes[12 * 320..])
            .unwrap()
            .starts_with("N 0000000012 ")
    );
}

#[test]
fn malformed_complete_note_and_missing_summary_children_are_errors() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let cwd = fixture.path();
    stdout(memo(cwd, &home, &default_args(&["init"])));
    for text in ["a", "b", "c", "d"] {
        stdout(memo(cwd, &home, &default_args(&["note", text])));
    }
    assert!(
        stderr(memo(cwd, &home, &default_args(&["nap", "0-3", "parent"])))
            .contains("requires child")
    );
    let log = home.join("data/memo/stores/default/notes.log");
    let mut bytes = fs::read(&log).unwrap();
    bytes[2] = b'9';
    fs::write(&log, bytes).unwrap();
    assert!(stderr(memo(cwd, &home, &default_args(&["wake"]))).contains("malformed complete note"));
}

#[test]
fn project_pin_maps_workspace_and_survives_other_cwd() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let main = fixture.path().join("main");
    let workspace = fixture.path().join("bay");
    fs::create_dir_all(main.join(".jj/repo")).unwrap();
    fs::create_dir_all(workspace.join(".jj")).unwrap();
    fs::write(workspace.join(".jj/repo"), "../../main/.jj/repo\n").unwrap();
    let initialized = stdout(memo(&main, &home, &["init"]));
    let selector = initialized
        .lines()
        .find_map(|l| l.strip_prefix("selector: "))
        .unwrap()
        .to_owned();
    stdout(memo(&workspace, &home, &["note", "from bay"]));
    stdout(memo(&main, &home, &["note", "from main"]));
    let pending =
        String::from_utf8(memo(&workspace, &home, &["wake", "--lines", "1"]).stdout).unwrap();
    assert!(pending.contains(&format!("memo --store {selector} nap 0-1")));
    stdout(memo(
        fixture.path(),
        &home,
        &["--store", &selector, "nap", "0-1", "shared"],
    ));
    assert!(
        stdout(memo(
            fixture.path(),
            &home,
            &["--store", &selector, "wake", "--lines", "1"]
        ))
        .contains("shared")
    );
}
