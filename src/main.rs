use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{ArgAction, Parser, Subcommand};
use serde::Deserialize;
use sha2::{Digest, Sha256};

const FORMAT_MARKER: &str = "FORMAT_VERSION";

#[derive(Debug, Parser)]
#[command(name = "memo", version, about)]
struct Cli {
    /// Root directory containing memo stores.
    #[arg(long, env = "MEMO_DATA_DIR", global = true, value_parser = parse_data_dir)]
    data_dir: Option<PathBuf>,
    /// Select the default, project, or a named store.
    #[arg(long, global = true)]
    store: Option<String>,
    /// Enable automatic project-store selection.
    #[arg(long, global = true, action = ArgAction::SetTrue, conflicts_with = "no_auto_project")]
    auto_project: bool,
    /// Disable automatic project-store selection.
    #[arg(long, global = true, action = ArgAction::SetTrue)]
    no_auto_project: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Show the selected store without creating it.
    Where,
    /// Initialize the selected store.
    Init,
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Config {
    auto_project: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { auto_project: true }
    }
}

#[derive(Debug, PartialEq)]
enum Kind {
    Default,
    Project,
    Named(String),
}

#[derive(Debug)]
struct Selection {
    kind: Kind,
    path: PathBuf,
}

fn main() -> Result<()> {
    run(Cli::parse(), &env::current_dir()?, &mut io::stdout())
}

fn run(cli: Cli, cwd: &Path, out: &mut dyn Write) -> Result<()> {
    let config = read_config()?;
    let auto_project = if cli.auto_project {
        true
    } else if cli.no_auto_project {
        false
    } else {
        config.auto_project
    };
    let root = absolute_data_root(cli.data_dir, cwd)?;
    let selection = select(&root, cwd, cli.store.as_deref(), auto_project)?;
    let initialized = marker_initialized(&selection.path)?;

    match cli.command.unwrap_or(Command::Where) {
        Command::Where => print_selection(out, &selection, initialized),
        Command::Init => {
            fs::create_dir_all(&selection.path)
                .with_context(|| format!("create store {}", selection.path.display()))?;
            let marker = selection.path.join(FORMAT_MARKER);
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&marker)
            {
                Ok(mut file) => file.write_all(b"1\n")?,
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    marker_initialized(&selection.path)?;
                }
                Err(error) => {
                    return Err(error).with_context(|| format!("create {}", marker.display()));
                }
            }
            print_selection(out, &selection, true)
        }
    }
}

fn print_selection(out: &mut dyn Write, selection: &Selection, initialized: bool) -> Result<()> {
    let kind = match &selection.kind {
        Kind::Default => "default".to_owned(),
        Kind::Project => "project".to_owned(),
        Kind::Named(name) => format!("named:{name}"),
    };
    writeln!(out, "kind: {kind}")?;
    writeln!(out, "path: {}", selection.path.display())?;
    writeln!(out, "initialized: {initialized}")?;
    Ok(())
}

fn marker_initialized(store: &Path) -> Result<bool> {
    let marker = store.join(FORMAT_MARKER);
    let metadata = match fs::metadata(&marker) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error).with_context(|| format!("inspect {}", marker.display())),
    };
    if !metadata.is_file() {
        bail!("format marker is not a regular file: {}", marker.display())
    }
    let contents = fs::read(&marker).with_context(|| format!("read {}", marker.display()))?;
    if contents != b"1\n" {
        bail!(
            "unsupported or malformed format marker {}: expected exactly '1\\n'",
            marker.display()
        )
    }
    Ok(true)
}

fn parse_data_dir(value: &str) -> std::result::Result<PathBuf, String> {
    if value.is_empty() {
        Err("data directory cannot be empty (including MEMO_DATA_DIR)".to_owned())
    } else {
        Ok(PathBuf::from(value))
    }
}

fn absolute_env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
}

fn read_config() -> Result<Config> {
    let home = absolute_env_path("HOME");
    let path = absolute_env_path("XDG_CONFIG_HOME")
        .or_else(|| home.map(|p| p.join(".config")))
        .map(|p| p.join("memo/config.toml"));
    let Some(path) = path else {
        return Ok(Config::default());
    };
    match fs::read_to_string(&path) {
        Ok(text) => toml::from_str(&text).with_context(|| format!("parse {}", path.display())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Config::default()),
        Err(error) => Err(error).with_context(|| format!("read {}", path.display())),
    }
}

fn absolute_data_root(override_path: Option<PathBuf>, cwd: &Path) -> Result<PathBuf> {
    let path = override_path
        .or_else(|| {
            absolute_env_path("XDG_DATA_HOME")
                .or_else(|| absolute_env_path("HOME").map(|h| h.join(".local/share")))
                .map(|p| p.join("memo/stores"))
        })
        .context("HOME or XDG_DATA_HOME must be set (or pass --data-dir)")?;
    Ok(if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    })
}

fn select(root: &Path, cwd: &Path, requested: Option<&str>, auto: bool) -> Result<Selection> {
    let repository = || discover_repository(cwd);
    let kind = match requested {
        Some("default") => Kind::Default,
        Some("project") => {
            if repository()?.is_none() {
                bail!("--store project requires a Git or jj repository")
            }
            Kind::Project
        }
        Some(name) => {
            validate_name(name)?;
            Kind::Named(name.to_owned())
        }
        None if auto && repository()?.is_some() => Kind::Project,
        None => Kind::Default,
    };
    let path = match &kind {
        Kind::Default => root.join("default"),
        Kind::Named(name) => root.join("named").join(name),
        Kind::Project => {
            let identity = repository()?.expect("project identity was already found");
            root.join("projects").join(project_id(&identity))
        }
    };
    Ok(Selection { kind, path })
}

fn validate_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name == "default"
        || name == "project"
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        bail!(
            "invalid store name {name:?}; use letters, digits, '-' or '_' (default and project are reserved)"
        )
    }
    Ok(())
}

/// Returns a namespace plus the canonical shared repository metadata path.
fn discover_repository(cwd: &Path) -> Result<Option<String>> {
    for directory in cwd.ancestors() {
        let jj = directory.join(".jj");
        let git = directory.join(".git");
        if jj.exists() {
            return jj_identity(&jj).map(Some);
        }
        if git.exists() {
            return git_identity(&git).map(Some);
        }
    }
    Ok(None)
}

fn jj_identity(marker: &Path) -> Result<String> {
    if !marker.is_dir() {
        bail!(
            "malformed jj marker: {} is not a directory",
            marker.display()
        )
    }
    let repo = marker.join("repo");
    let target = if repo.is_dir() {
        repo
    } else if repo.is_file() {
        let pointer =
            fs::read_to_string(&repo).with_context(|| format!("read {}", repo.display()))?;
        let pointer = pointer.trim();
        if pointer.is_empty() {
            bail!("malformed jj repository pointer: {}", repo.display())
        }
        marker.join(pointer)
    } else {
        bail!("malformed jj marker: missing {}", repo.display())
    };
    let canonical = target
        .canonicalize()
        .with_context(|| format!("resolve jj repository {}", target.display()))?;
    if !canonical.is_dir() {
        bail!("jj repository is not a directory: {}", canonical.display())
    }
    Ok(format!("jj:{}", canonical.display()))
}

fn git_identity(marker: &Path) -> Result<String> {
    let admin = if marker.is_dir() {
        marker.to_path_buf()
    } else if marker.is_file() {
        let text =
            fs::read_to_string(marker).with_context(|| format!("read {}", marker.display()))?;
        let target = text
            .strip_prefix("gitdir:")
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .context("malformed .git file: expected 'gitdir: PATH'")?;
        marker.parent().expect("marker has parent").join(target)
    } else {
        bail!("malformed Git marker: {}", marker.display())
    };
    let common_file = admin.join("commondir");
    let common = if common_file.exists() {
        let text = fs::read_to_string(&common_file)
            .with_context(|| format!("read {}", common_file.display()))?;
        let value = text.trim();
        if value.is_empty() {
            bail!("malformed Git commondir: {}", common_file.display())
        }
        admin.join(value)
    } else {
        admin
    };
    let canonical = common
        .canonicalize()
        .with_context(|| format!("resolve Git common directory {}", common.display()))?;
    if !canonical.is_dir() {
        bail!(
            "Git common directory is not a directory: {}",
            canonical.display()
        )
    }
    Ok(format!("git:{}", canonical.display()))
}

fn project_id(identity: &str) -> String {
    let digest = Sha256::digest(identity.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn outside_defaults_and_where_does_not_create() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("stores");
        let selected = select(&root, temp.path(), None, true).unwrap();
        assert_eq!(selected.kind, Kind::Default);
        assert!(!selected.path.exists());
    }

    #[test]
    fn nested_jj_and_workspace_share_identity() {
        let temp = TempDir::new().unwrap();
        let main = temp.path().join("main");
        let workspace = temp.path().join("workspace");
        fs::create_dir_all(main.join(".jj/repo")).unwrap();
        fs::create_dir_all(workspace.join(".jj")).unwrap();
        fs::write(workspace.join(".jj/repo"), "../../main/.jj/repo\n").unwrap();
        fs::create_dir_all(main.join("deep")).unwrap();
        assert_eq!(
            discover_repository(&main.join("deep")).unwrap(),
            discover_repository(&workspace).unwrap()
        );
    }

    #[test]
    fn git_worktree_uses_common_dir() {
        let temp = TempDir::new().unwrap();
        let main = temp.path().join("main");
        let worktree = temp.path().join("worktree");
        fs::create_dir_all(main.join(".git/worktrees/w")).unwrap();
        fs::create_dir_all(&worktree).unwrap();
        fs::write(
            worktree.join(".git"),
            format!("gitdir: {}\n", main.join(".git/worktrees/w").display()),
        )
        .unwrap();
        fs::write(main.join(".git/worktrees/w/commondir"), "../..\n").unwrap();
        assert_eq!(
            discover_repository(&main).unwrap(),
            discover_repository(&worktree).unwrap()
        );
    }

    #[test]
    fn malformed_marker_is_an_error() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join(".jj")).unwrap();
        assert!(discover_repository(temp.path()).is_err());
    }

    #[test]
    fn explicit_selection_and_names() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("stores");
        assert_eq!(
            select(&root, temp.path(), Some("default"), true)
                .unwrap()
                .kind,
            Kind::Default
        );
        assert_eq!(
            select(&root, temp.path(), Some("notes_2"), true)
                .unwrap()
                .kind,
            Kind::Named("notes_2".into())
        );
        assert!(select(&root, temp.path(), Some("../bad"), true).is_err());
        assert!(select(&root, temp.path(), Some("project"), true).is_err());
    }
}
