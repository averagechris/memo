use std::fs;
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
