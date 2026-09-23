use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{ArgAction, Parser, Subcommand};
use fs2::FileExt;
use serde::Deserialize;
use sha2::{Digest, Sha256};

const FORMAT_MARKER: &str = "FORMAT_VERSION";
const RECORD_BYTES: u64 = 320;
const MAX_TEXT_BYTES: usize = 280;

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
    /// Append one memory.
    Note { text: String },
    /// Read the selected store, using summaries to fit the line budget.
    Wake {
        #[arg(long, default_value_t = 96)]
        lines: usize,
    },
    /// Show the next summary request, or settle one requested range.
    Nap {
        span: Option<String>,
        summary: Option<String>,
    },
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
    selector: String,
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
        Command::Note { text } => {
            require_initialized(&selection)?;
            append_note(&selection, &text, out)
        }
        Command::Wake { lines } => {
            require_initialized(&selection)?;
            wake(&selection, lines, out)
        }
        Command::Nap { span, summary } => {
            require_initialized(&selection)?;
            nap(&selection, span.as_deref(), summary.as_deref(), out)
        }
    }
}

fn require_initialized(selection: &Selection) -> Result<()> {
    if !marker_initialized(&selection.path)? {
        bail!(
            "selected store is not initialized: {} ({})\nrun: memo --store {} init",
            selection.path.display(),
            selection.selector,
            selection.selector
        )
    }
    Ok(())
}

fn validate_text(text: &str, what: &str) -> Result<()> {
    if text.is_empty() || text.contains(['\n', '\r']) || text.len() > MAX_TEXT_BYTES {
        bail!("{what} must be one nonempty line of at most {MAX_TEXT_BYTES} UTF-8 bytes")
    }
    Ok(())
}

fn locked_store(selection: &Selection) -> Result<File> {
    let path = selection.path.join("store.lock");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .with_context(|| format!("open {}", path.display()))?;
    file.lock_exclusive()
        .with_context(|| format!("lock {}", path.display()))?;
    Ok(file)
}

fn open_repaired(path: &Path) -> Result<File> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .with_context(|| format!("open {}", path.display()))?;
    let len = file.metadata()?.len();
    let complete = len / RECORD_BYTES * RECORD_BYTES;
    if len != complete {
        file.set_len(complete)
            .with_context(|| format!("repair torn suffix in {}", path.display()))?;
        file.sync_all()?;
    }
    file.seek(SeekFrom::Start(complete))?;
    Ok(file)
}

fn record(prefix: &str, text: &str) -> Result<[u8; RECORD_BYTES as usize]> {
    validate_text(text, "text")?;
    let body = format!("{prefix}{text}");
    if body.len() >= RECORD_BYTES as usize {
        bail!("record metadata is too large")
    }
    let mut slot = [b' '; RECORD_BYTES as usize];
    slot[..body.len()].copy_from_slice(body.as_bytes());
    slot[RECORD_BYTES as usize - 1] = b'\n';
    Ok(slot)
}

fn append_note(selection: &Selection, text: &str, out: &mut dyn Write) -> Result<()> {
    validate_text(text, "note")?;
    let _lock = locked_store(selection)?;
    let path = selection.path.join("notes.log");
    let mut file = open_repaired(&path)?;
    validate_notes(&mut file)?;
    let id = file.metadata()?.len() / RECORD_BYTES;
    if id > 9_999_999_999 {
        bail!("note ID exceeds the version-1 ten-digit limit")
    }
    let prefix = format!("N {id:010} {} ", chrono::Local::now().format("%Y-%m-%d"));
    file.seek(SeekFrom::End(0))?;
    file.write_all(&record(&prefix, text)?)?;
    file.sync_all()?;
    writeln!(
        out,
        "noted {id} in {} ({})",
        selection.path.display(),
        selection.selector
    )?;
    Ok(())
}

fn read_slot(file: &mut File, index: u64) -> Result<[u8; RECORD_BYTES as usize]> {
    file.seek(SeekFrom::Start(index * RECORD_BYTES))?;
    let mut slot = [0; RECORD_BYTES as usize];
    file.read_exact(&mut slot)?;
    Ok(slot)
}

fn slot_body(slot: &[u8; RECORD_BYTES as usize]) -> Result<&str> {
    if slot[RECORD_BYTES as usize - 1] != b'\n' {
        bail!("complete record has malformed terminator")
    }
    let body = &slot[..RECORD_BYTES as usize - 1];
    let end = body.iter().rposition(|b| *b != b' ').map_or(0, |i| i + 1);
    std::str::from_utf8(&body[..end]).context("record is not valid UTF-8")
}

fn parse_note(slot: &[u8; RECORD_BYTES as usize], expected: u64) -> Result<String> {
    let body = slot_body(slot)?;
    let prefix = format!("N {expected:010} ");
    let rest = body
        .strip_prefix(&prefix)
        .context("note record ID/type does not match its slot")?;
    if rest.len() < 12 || &rest.as_bytes()[10..11] != b" " || !valid_date(&rest[..10]) {
        bail!("note record has malformed date metadata")
    }
    let text = &rest[11..];
    validate_text(text, "stored note")?;
    Ok(text.to_owned())
}

fn valid_date(date: &str) -> bool {
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_ok()
}

fn validate_notes(file: &mut File) -> Result<()> {
    let count = file.metadata()?.len() / RECORD_BYTES;
    for id in 0..count {
        parse_note(&read_slot(file, id)?, id)
            .with_context(|| format!("malformed complete note record {id}"))?;
    }
    Ok(())
}

fn note_count(selection: &Selection) -> Result<u64> {
    let path = selection.path.join("notes.log");
    if !path.exists() {
        return Ok(0);
    }
    let file = OpenOptions::new().read(true).open(&path)?;
    let len = file.metadata()?.len();
    if len % RECORD_BYTES != 0 {
        bail!("notes log has a torn suffix; run note or nap to repair it under lock")
    }
    Ok(len / RECORD_BYTES)
}

fn summary_path(selection: &Selection, span: u64) -> PathBuf {
    selection.path.join("summaries").join(format!("{span}.log"))
}

fn parse_summary(slot: &[u8; RECORD_BYTES as usize], lo: u64, hi: u64) -> Result<String> {
    let body = slot_body(slot)?;
    let prefix = format!("S {lo:010}-{hi:010} ");
    let rest = body
        .strip_prefix(&prefix)
        .context("summary range/type does not match its slot")?;
    if rest.len() < 12 || &rest.as_bytes()[10..11] != b" " || !valid_date(&rest[..10]) {
        bail!("summary record has malformed date metadata")
    }
    let text = &rest[11..];
    validate_text(text, "stored summary")?;
    Ok(text.to_owned())
}

fn read_summary(selection: &Selection, lo: u64, span: u64) -> Result<Option<String>> {
    let path = summary_path(selection, span);
    let mut file = match OpenOptions::new().read(true).open(&path) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let len = file.metadata()?.len();
    if len % RECORD_BYTES != 0 {
        bail!("summary log has torn suffix: {}", path.display())
    }
    let index = lo / span;
    if index >= len / RECORD_BYTES {
        return Ok(None);
    }
    parse_summary(&read_slot(&mut file, index)?, lo, lo + span - 1)
        .map(Some)
        .with_context(|| format!("malformed complete summary record in {}", path.display()))
}

fn request(out: &mut dyn Write, selection: &Selection, lo: u64, span: u64) -> Result<()> {
    writeln!(
        out,
        "summary pending for {} ({})",
        selection.path.display(),
        selection.selector
    )?;
    writeln!(
        out,
        "next: memo --store {} nap {}-{} \"summary\"",
        selection.selector,
        lo,
        lo + span - 1
    )?;
    Ok(())
}

fn parse_span(value: &str) -> Result<(u64, u64, u64)> {
    let (a, b) = value.split_once('-').context("span must be LO-HI")?;
    let lo: u64 = a.parse()?;
    let hi: u64 = b.parse()?;
    let span = hi
        .checked_sub(lo)
        .and_then(|v| v.checked_add(1))
        .context("invalid span")?;
    if span < 2 || !span.is_power_of_two() || !lo.is_multiple_of(span) {
        bail!("span must be an aligned power-of-two range of at least two notes")
    }
    Ok((lo, hi, span))
}

fn nap(
    selection: &Selection,
    span_arg: Option<&str>,
    summary: Option<&str>,
    out: &mut dyn Write,
) -> Result<()> {
    if span_arg.is_none() {
        if summary.is_some() {
            bail!("summary requires LO-HI")
        };
        return next_nap(selection, out);
    }
    let summary = summary.context("nap LO-HI requires a summary")?;
    validate_text(summary, "summary")?;
    let (lo, hi, span) = parse_span(span_arg.unwrap())?;
    if hi > 9_999_999_999 {
        bail!("summary range exceeds the version-1 ten-digit limit")
    }
    let _lock = locked_store(selection)?;
    let mut notes = open_repaired(&selection.path.join("notes.log"))?;
    validate_notes(&mut notes)?;
    let total = notes.metadata()?.len() / RECORD_BYTES;
    if hi >= total {
        bail!("summary range ends beyond the last note ({total} notes)")
    }
    if span > 2 {
        let half = span / 2;
        for child in [lo, lo + half] {
            if read_summary(selection, child, half)?.is_none() {
                bail!(
                    "parent summary requires child {}-{} first",
                    child,
                    child + half - 1
                )
            }
        }
    }
    fs::create_dir_all(selection.path.join("summaries"))?;
    let path = summary_path(selection, span);
    let mut file = open_repaired(&path)?;
    let index = lo / span;
    let count = file.metadata()?.len() / RECORD_BYTES;
    if index < count {
        let old = parse_summary(&read_slot(&mut file, index)?, lo, hi)?;
        if old == summary {
            writeln!(
                out,
                "already settled {lo}-{hi} in {}",
                selection.path.display()
            )?;
            return Ok(());
        }
        bail!("summary {lo}-{hi} is already settled with different text")
    }
    if index != count {
        bail!(
            "summary level must be written as a dense prefix; settle range {}-{} first",
            count * span,
            count * span + span - 1
        )
    }
    let prefix = format!(
        "S {lo:010}-{hi:010} {} ",
        chrono::Local::now().format("%Y-%m-%d")
    );
    file.seek(SeekFrom::End(0))?;
    file.write_all(&record(&prefix, summary)?)?;
    file.sync_all()?;
    writeln!(
        out,
        "settled {lo}-{hi} in {} ({})",
        selection.path.display(),
        selection.selector
    )?;
    Ok(())
}

fn next_nap(selection: &Selection, out: &mut dyn Write) -> Result<()> {
    let total = note_count(selection)?;
    let mut span = 2;
    while span <= total.next_power_of_two() {
        for lo in (0..total).step_by(span as usize) {
            if lo + span <= total
                && read_summary(selection, lo, span)?.is_none()
                && (span == 2
                    || (read_summary(selection, lo, span / 2)?.is_some()
                        && read_summary(selection, lo + span / 2, span / 2)?.is_some()))
            {
                return request(out, selection, lo, span);
            }
        }
        span *= 2;
    }
    writeln!(
        out,
        "no eligible summary pending in {} ({})",
        selection.path.display(),
        selection.selector
    )?;
    Ok(())
}

fn wake(selection: &Selection, budget: usize, out: &mut dyn Write) -> Result<()> {
    if budget == 0 {
        bail!("--lines must be at least 1")
    }
    let total = note_count(selection)?;
    let mut blocks = Vec::new();
    let mut lo = 0;
    while lo < total {
        let remain = total - lo;
        let span = if lo == 0 {
            1u64 << (63 - remain.leading_zeros())
        } else {
            (1u64 << lo.trailing_zeros()).min(1u64 << (63 - remain.leading_zeros()))
        };
        blocks.push((lo, span));
        lo += span;
    }
    if blocks.len() > budget {
        bail!(
            "--lines {budget} cannot represent {total} notes without omission (minimum {})",
            blocks.len()
        )
    }
    while blocks.len() < budget {
        let Some(i) = blocks.iter().rposition(|(_, s)| *s > 1) else {
            break;
        };
        let (l, s) = blocks.remove(i);
        blocks.insert(i, (l, s / 2));
        blocks.insert(i + 1, (l + s / 2, s / 2));
    }
    let mut notes = if total > 0 {
        Some(
            OpenOptions::new()
                .read(true)
                .open(selection.path.join("notes.log"))?,
        )
    } else {
        None
    };
    for (l, s) in blocks {
        if s == 1 {
            let text = parse_note(&read_slot(notes.as_mut().unwrap(), l)?, l)
                .with_context(|| format!("malformed complete note record {l}"))?;
            writeln!(out, "{l}: {text}")?
        } else if let Some(text) = read_summary(selection, l, s)? {
            writeln!(out, "{l}-{}: {text}", l + s - 1)?
        } else {
            request(out, selection, l, s)?;
            bail!("wake incomplete: required summary is pending")
        }
    }
    writeln!(
        out,
        "wake complete: {} ({})",
        selection.path.display(),
        selection.selector
    )?;
    Ok(())
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
    writeln!(out, "selector: {}", selection.selector)?;
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
    let pinned_project = requested.and_then(|r| r.strip_prefix("project:"));
    if let Some(id) = pinned_project {
        if id.len() != 64
            || !id.bytes().all(|b| {
                b.is_ascii_hexdigit() && (!b.is_ascii_alphabetic() || b.is_ascii_lowercase())
            })
        {
            bail!("invalid pinned project ID")
        }
        return Ok(Selection {
            kind: Kind::Project,
            path: root.join("projects").join(id),
            selector: format!("project:{id}"),
        });
    }
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
    let selector = match &kind {
        Kind::Default => "default".into(),
        Kind::Named(n) => n.clone(),
        Kind::Project => format!("project:{}", path.file_name().unwrap().to_string_lossy()),
    };
    Ok(Selection {
        kind,
        path,
        selector,
    })
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
