use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::fs::symlink;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

fn memo(cwd: &Path, home: &Path, args: &[&str]) -> Output {
    memo_binary_data(
        Path::new(env!("CARGO_BIN_EXE_memo")),
        cwd,
        home,
        &home.join("data"),
        args,
    )
}

fn memo_binary_data(
    binary: &Path,
    cwd: &Path,
    home: &Path,
    data_home: &Path,
    args: &[&str],
) -> Output {
    memo_binary_data_dir(binary, cwd, home, data_home, None, args)
}

fn memo_binary_override(
    binary: &Path,
    cwd: &Path,
    home: &Path,
    data_dir: &Path,
    args: &[&str],
) -> Output {
    memo_binary_data_dir(binary, cwd, home, &home.join("data"), Some(data_dir), args)
}

fn memo_binary_data_dir(
    binary: &Path,
    cwd: &Path,
    home: &Path,
    data_home: &Path,
    data_dir: Option<&Path>,
    args: &[&str],
) -> Output {
    let mut command = Command::new(binary);
    command
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .env("HOME", home)
        .env("XDG_DATA_HOME", data_home)
        .env("XDG_CONFIG_HOME", home.join("config"));
    if let Some(data_dir) = data_dir {
        command.env("MEMO_DATA_DIR", data_dir);
    }
    command.output().unwrap()
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
fn skill_commands_are_embedded_installable_and_memory_independent() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let cwd = fixture.path().join("broken");
    fs::create_dir_all(cwd.join(".jj")).unwrap();
    fs::create_dir_all(home.join("config/memo")).unwrap();
    fs::write(home.join("config/memo/config.toml"), "not toml = [").unwrap();

    let bundled = include_str!("../skills/memo/SKILL.md");
    assert_eq!(stdout(memo(&cwd, &home, &["skills"])), bundled);
    assert_eq!(stdout(memo(&cwd, &home, &["skills", "show"])), bundled);
    stdout(memo(&cwd, &home, &["skills", "install"]));
    let installed = home.join("config/opencode/skills/memo/SKILL.md");
    assert_eq!(fs::read_to_string(&installed).unwrap(), bundled);
    assert!(stderr(memo(&cwd, &home, &["skills", "install"])).contains("refusing to overwrite"));

    let custom = fixture.path().join("custom");
    stdout(memo(
        &cwd,
        &home,
        &["skills", "install", "--dir", custom.to_str().unwrap()],
    ));
    assert_eq!(
        fs::read_to_string(custom.join("memo/SKILL.md")).unwrap(),
        bundled
    );

    let outside = fixture.path().join("outside");
    fs::write(&outside, "keep").unwrap();
    fs::remove_file(&installed).unwrap();
    symlink(&outside, &installed).unwrap();
    stdout(memo(&cwd, &home, &["skills", "install", "--force"]));
    assert_eq!(fs::read_to_string(outside).unwrap(), "keep");
    assert_eq!(fs::read_to_string(installed).unwrap(), bundled);
}

#[test]
fn completion_commands_generate_and_install_shell_filenames() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let cwd = fixture.path().join("broken");
    fs::create_dir_all(cwd.join(".jj")).unwrap();
    fs::create_dir_all(home.join("config/memo")).unwrap();
    fs::write(home.join("config/memo/config.toml"), "broken = [").unwrap();

    for shell in ["bash", "zsh", "fish", "elvish", "powershell"] {
        let script = stdout(memo(&cwd, &home, &["completions", shell]));
        assert!(script.contains("memo"), "empty {shell} completion");
    }
    for (shell, path) in [
        ("bash", "data/bash-completion/completions/memo"),
        ("fish", "config/fish/completions/memo.fish"),
        ("zsh", "data/zsh/site-functions/_memo"),
    ] {
        stdout(memo(&cwd, &home, &["completions", "install", shell]));
        assert!(home.join(path).is_file(), "missing {path}");
    }
    assert!(
        stderr(memo(&cwd, &home, &["completions", "install", "elvish"])).contains("requires --dir")
    );
    let custom = fixture.path().join("completions");
    stdout(memo(
        &cwd,
        &home,
        &[
            "completions",
            "install",
            "powershell",
            "--dir",
            custom.to_str().unwrap(),
        ],
    ));
    assert!(custom.join("_memo.ps1").is_file());
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
fn generated_commands_pin_executable_and_data_root() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let cwd = fixture.path().join("original cwd");
    let other_cwd = fixture.path().join("other cwd");
    let selected_root = fixture.path().join("selected root 'a'");
    let fallback_data = fixture.path().join("fallback data");
    let fallback_store = fallback_data.join("memo/stores/default");
    fs::create_dir_all(&cwd).unwrap();
    fs::create_dir_all(&other_cwd).unwrap();

    let binary = fixture.path().join("bin dir/it's/memo");
    fs::create_dir_all(binary.parent().unwrap()).unwrap();
    fs::copy(env!("CARGO_BIN_EXE_memo"), &binary).unwrap();

    stdout(memo_binary_override(
        &binary,
        &cwd,
        &home,
        &selected_root,
        &["--store", "default", "init"],
    ));
    stdout(memo_binary_data(
        &binary,
        &cwd,
        &home,
        &fallback_data,
        &["--store", "default", "init"],
    ));
    assert!(selected_root.join("default/FORMAT_VERSION").is_file());
    assert!(fallback_store.join("FORMAT_VERSION").is_file());
    stdout(memo_binary_override(
        &binary,
        &cwd,
        &home,
        &selected_root,
        &["--store", "default", "note", "first"],
    ));
    stdout(memo_binary_override(
        &binary,
        &cwd,
        &home,
        &selected_root,
        &["--store", "default", "note", "second"],
    ));

    let pending = stdout(memo_binary_override(
        &binary,
        &cwd,
        &home,
        &selected_root,
        &["--store", "default", "nap"],
    ));
    let command = pending
        .lines()
        .find_map(|line| line.strip_prefix("next: "))
        .unwrap();
    assert!(command.contains("--data-dir '"));
    assert!(command.contains("selected root '\\''a'"));
    assert!(command.contains("bin dir/it'\\''s/memo"));

    let fake_bin = fixture.path().join("fake bin");
    fs::create_dir_all(&fake_bin).unwrap();
    let fake_memo = fake_bin.join("memo");
    let fake_called = fixture.path().join("fake memo called");
    fs::write(
        &fake_memo,
        format!("#!/bin/sh\nprintf called > '{}'\n", fake_called.display()),
    )
    .unwrap();
    fs::set_permissions(&fake_memo, fs::Permissions::from_mode(0o755)).unwrap();

    let executed = Command::new("/bin/sh")
        .arg("-c")
        .arg(command)
        .current_dir(&other_cwd)
        .env_clear()
        .env("HOME", &home)
        .env("XDG_DATA_HOME", &fallback_data)
        .env("PATH", &fake_bin)
        .output()
        .unwrap();
    assert!(
        executed.status.success(),
        "{}",
        String::from_utf8_lossy(&executed.stderr)
    );
    let summary = fs::read(selected_root.join("default/summaries/2.log")).unwrap();
    assert!(
        summary
            .windows(b"summary".len())
            .any(|window| window == b"summary")
    );
    assert!(!fallback_store.join("summaries").exists());
    assert!(!fake_called.exists());

    let error = stderr(memo_binary_override(
        &binary,
        &cwd,
        &home,
        &selected_root,
        &["--store", "fresh", "note", "literal"],
    ));
    let init_command = error
        .lines()
        .find_map(|line| line.strip_prefix("run: "))
        .unwrap();
    let initialized = Command::new("/bin/sh")
        .arg("-c")
        .arg(init_command)
        .current_dir(&other_cwd)
        .env_clear()
        .env("HOME", &home)
        .env("XDG_DATA_HOME", &fallback_data)
        .env("PATH", &fake_bin)
        .output()
        .unwrap();
    assert!(
        initialized.status.success(),
        "{}",
        String::from_utf8_lossy(&initialized.stderr)
    );
    assert!(selected_root.join("named/fresh/FORMAT_VERSION").is_file());
    assert!(!fallback_store.join("named/fresh").exists());
    assert!(!fake_called.exists());
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
    assert!(project_independent.contains("--store 'default' nap 0-1 \"summary\""));

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
    for bad in ["", " ", "   ", "two\nlines", "two\rlines"] {
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
fn trailing_spaces_normalize_and_pending_requests_include_sources() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let cwd = fixture.path();
    stdout(memo(cwd, &home, &default_args(&["init"])));
    stdout(memo(cwd, &home, &default_args(&["note", "first   "])));
    let noted = stdout(memo(cwd, &home, &default_args(&["note", "second"])));
    assert!(noted.contains("source 0: first\nsource 1: second"));
    assert!(noted.contains("--store 'default' nap 0-1 \"summary\""));
    stdout(memo(cwd, &home, &default_args(&["nap", "0-1", "both   "])));
    assert!(
        stdout(memo(cwd, &home, &default_args(&["nap", "0-1", "both"])))
            .contains("already settled")
    );
}

#[test]
fn four_note_wake_guides_runnable_prerequisites_until_covered() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let cwd = fixture.path();
    stdout(memo(cwd, &home, &default_args(&["init"])));
    for text in ["a", "b", "c", "d"] {
        stdout(memo(cwd, &home, &default_args(&["note", text])));
    }

    let first = memo(cwd, &home, &default_args(&["wake", "--lines", "1"]));
    assert!(!first.status.success());
    assert!(String::from_utf8(first.stdout).unwrap().contains("nap 0-1"));
    stdout(memo(cwd, &home, &default_args(&["nap", "0-1", "ab"])));
    let second = memo(cwd, &home, &default_args(&["wake", "--lines", "1"]));
    assert!(
        String::from_utf8(second.stdout)
            .unwrap()
            .contains("nap 2-3")
    );
    stdout(memo(cwd, &home, &default_args(&["nap", "2-3", "cd"])));
    let parent = memo(cwd, &home, &default_args(&["wake", "--lines", "1"]));
    let parent_out = String::from_utf8(parent.stdout).unwrap();
    assert!(parent_out.contains("source 0-1: ab\nsource 2-3: cd"));
    assert!(parent_out.contains("nap 0-3"));
    stdout(memo(cwd, &home, &default_args(&["nap", "0-3", "all"])));
    assert!(
        stdout(memo(cwd, &home, &default_args(&["wake", "--lines", "1"]))).contains("0-3: all")
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
fn zero_filled_final_note_slots_are_repaired() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let cwd = fixture.path();
    stdout(memo(cwd, &home, &default_args(&["init"])));
    stdout(memo(cwd, &home, &default_args(&["note", "before"])));
    let log = home.join("data/memo/stores/default/notes.log");
    let mut bytes = fs::read(&log).unwrap();
    bytes.extend_from_slice(&[0; 640]);
    bytes.extend_from_slice(b"torn");
    fs::write(&log, bytes).unwrap();

    stdout(memo(cwd, &home, &default_args(&["note", "after"])));
    let bytes = fs::read(&log).unwrap();
    assert_eq!(bytes.len(), 2 * 320);
    assert!(
        std::str::from_utf8(&bytes[320..])
            .unwrap()
            .starts_with("N 0000000001 ")
    );
}

#[test]
fn zero_filled_note_slot_requires_valid_predecessor_without_mutating_failure() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let cwd = fixture.path();
    stdout(memo(cwd, &home, &default_args(&["init"])));
    for text in ["a", "b"] {
        stdout(memo(cwd, &home, &default_args(&["note", text])));
    }
    let log = home.join("data/memo/stores/default/notes.log");
    let mut bytes = fs::read(&log).unwrap();
    bytes[320 + 2] = b'9';
    bytes.extend_from_slice(&[0; 320]);
    fs::write(&log, &bytes).unwrap();

    let error = stderr(memo(cwd, &home, &default_args(&["note", "c"])));
    assert!(error.contains("malformed complete note record 1"));
    assert_eq!(fs::read(&log).unwrap(), bytes);
}

#[test]
fn append_checks_only_the_final_complete_slot_before_tail_repair() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let cwd = fixture.path();
    stdout(memo(cwd, &home, &default_args(&["init"])));
    stdout(memo(cwd, &home, &default_args(&["note", "a"])));
    stdout(memo(cwd, &home, &default_args(&["note", "b"])));
    let log = home.join("data/memo/stores/default/notes.log");

    let mut bytes = fs::read(&log).unwrap();
    bytes[320 + 2] = b'9';
    bytes.extend_from_slice(b"torn");
    fs::write(&log, &bytes).unwrap();
    let error = stderr(memo(cwd, &home, &default_args(&["note", "c"])));
    assert!(error.contains("malformed complete note record 1"));
    assert_eq!(fs::read(&log).unwrap(), bytes);

    bytes.truncate(640);
    bytes[320 + 2] = b'0';
    bytes[2] = b'9';
    fs::write(&log, bytes).unwrap();
    stdout(memo(cwd, &home, &default_args(&["note", "c"])));
    assert!(stderr(memo(cwd, &home, &default_args(&["wake"]))).contains("record 0"));
}

#[test]
fn no_arg_nap_repairs_a_torn_summary_suffix_and_reprompts() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let cwd = fixture.path();
    stdout(memo(cwd, &home, &default_args(&["init"])));
    for text in ["a", "b", "c", "d"] {
        stdout(memo(cwd, &home, &default_args(&["note", text])));
    }
    stdout(memo(cwd, &home, &default_args(&["nap", "0-1", "ab"])));
    let summaries = home.join("data/memo/stores/default/summaries/2.log");
    fs::OpenOptions::new()
        .append(true)
        .open(&summaries)
        .unwrap()
        .write_all(b"torn")
        .unwrap();
    let pending = stdout(memo(cwd, &home, &default_args(&["nap"])));
    assert!(pending.contains("nap 2-3"));
    assert_eq!(fs::metadata(summaries).unwrap().len(), 320);
}

#[test]
fn zero_filled_final_summary_slot_is_repaired_and_reprompts() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    let cwd = fixture.path();
    stdout(memo(cwd, &home, &default_args(&["init"])));
    for text in ["a", "b", "c", "d"] {
        stdout(memo(cwd, &home, &default_args(&["note", text])));
    }
    stdout(memo(cwd, &home, &default_args(&["nap", "0-1", "ab"])));
    let summaries = home.join("data/memo/stores/default/summaries/2.log");
    let mut bytes = fs::read(&summaries).unwrap();
    bytes.extend_from_slice(&[0; 320]);
    fs::write(&summaries, bytes).unwrap();

    let pending = stdout(memo(cwd, &home, &default_args(&["nap"])));
    assert!(pending.contains("nap 2-3"));
    assert_eq!(fs::metadata(summaries).unwrap().len(), 320);
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
    assert!(pending.contains(&format!("--store '{selector}' nap 0-1 \"summary\"")));
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
