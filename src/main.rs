use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{ArgAction, CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use fs2::FileExt;
use serde::Deserialize;
use sha2::{Digest, Sha256};

const FORMAT_MARKER: &str = "FORMAT_VERSION";
const RECORD_BYTES: u64 = 320;
const MAX_TEXT_BYTES: usize = 280;
const BUNDLED_SKILL: &str = include_str!("../skills/memo/SKILL.md");

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
    /// Show or install the bundled OpenCode skill.
    Skills {
        #[command(subcommand)]
        command: Option<SkillsCommand>,
    },
    /// Generate or install shell completions.
    Completions {
        #[command(subcommand)]
        command: CompletionsCommand,
    },
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

#[derive(Debug, Subcommand)]
enum SkillsCommand {
    /// Print the bundled skill.
    Show,
    /// Install the bundled skill.
    Install {
        /// OpenCode skills directory (the memo subdirectory is added).
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Replace an existing skill file.
        #[arg(long)]
        force: bool,
    },
}

#[derive(Debug, Subcommand)]
enum CompletionsCommand {
    /// Print a completion script.
    #[command(external_subcommand)]
    Generate(Vec<String>),
    /// Install a completion script.
    Install {
        #[arg(value_enum)]
        shell: Shell,
        /// Destination directory (the shell-specific filename is added).
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Replace an existing completion file.
        #[arg(long)]
        force: bool,
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
    data_root: PathBuf,
    path: PathBuf,
    selector: String,
}

fn main() -> Result<()> {
    run(Cli::parse(), &env::current_dir()?, &mut io::stdout())
}

fn run(cli: Cli, cwd: &Path, out: &mut dyn Write) -> Result<()> {
    match &cli.command {
        Some(Command::Skills { command }) => {
            return run_skills(command.as_ref(), cwd, out);
        }
        Some(Command::Completions { command }) => {
            return run_completions(command, cwd, out);
        }
        _ => {}
    }
    let executable = env::current_exe().context("resolve current executable")?;
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
        Command::Skills { .. } | Command::Completions { .. } => unreachable!(),
        Command::Where => print_selection(out, &selection, initialized),
        Command::Init => {
            create_dir_all_synced(&selection.path)
                .with_context(|| format!("create store {}", selection.path.display()))?;
            let marker = selection.path.join(FORMAT_MARKER);
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&marker)
            {
                Ok(mut file) => {
                    file.write_all(b"1\n")?;
                    file.sync_all()?;
                    sync_directory(&selection.path)?;
                }
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
            require_initialized(&selection, &executable)?;
            append_note(&selection, &executable, &text, out)
        }
        Command::Wake { lines } => {
            require_initialized(&selection, &executable)?;
            wake(&selection, &executable, lines, out)
        }
        Command::Nap { span, summary } => {
            require_initialized(&selection, &executable)?;
            nap(
                &selection,
                &executable,
                span.as_deref(),
                summary.as_deref(),
                out,
            )
        }
    }
}

fn run_skills(command: Option<&SkillsCommand>, cwd: &Path, out: &mut dyn Write) -> Result<()> {
    match command {
        None | Some(SkillsCommand::Show) => {
            out.write_all(BUNDLED_SKILL.as_bytes()).map_err(Into::into)
        }
        Some(SkillsCommand::Install { dir, force }) => {
            let base = match dir {
                Some(path) => absolute_destination(path, cwd),
                None => absolute_env_path("XDG_CONFIG_HOME")
                    .or_else(|| absolute_env_path("HOME").map(|p| p.join(".config")))
                    .context("HOME or XDG_CONFIG_HOME must be set (or pass --dir)")?
                    .join("opencode/skills"),
            };
            let path = base.join("memo/SKILL.md");
            safe_write(&path, BUNDLED_SKILL.as_bytes(), *force)?;
            writeln!(out, "installed {}", path.display())?;
            Ok(())
        }
    }
}

fn run_completions(command: &CompletionsCommand, cwd: &Path, out: &mut dyn Write) -> Result<()> {
    match command {
        CompletionsCommand::Generate(values) => {
            if values.len() != 1 {
                bail!("completions requires a shell")
            }
            let shell: Shell = values[0]
                .parse()
                .map_err(|_| anyhow::anyhow!("unsupported shell {}", values[0]))?;
            generate_completion(shell, out)
        }
        CompletionsCommand::Install { shell, dir, force } => {
            let (default_dir, filename) = completion_destination(*shell)?;
            let base = dir
                .as_ref()
                .map(|p| absolute_destination(p, cwd))
                .or(default_dir)
                .context("Elvish and PowerShell completion installation requires --dir")?;
            let mut bytes = Vec::new();
            generate_completion(*shell, &mut bytes)?;
            let path = base.join(filename);
            safe_write(&path, &bytes, *force)?;
            writeln!(out, "installed {}", path.display())?;
            if *shell == Shell::Zsh {
                writeln!(out, "ensure {} is in your zsh fpath", base.display())?;
            }
            Ok(())
        }
    }
}

fn generate_completion(shell: Shell, out: &mut dyn Write) -> Result<()> {
    let mut command = Cli::command();
    clap_complete::generate(shell, &mut command, "memo", out);
    Ok(())
}

fn completion_destination(shell: Shell) -> Result<(Option<PathBuf>, &'static str)> {
    let home = absolute_env_path("HOME");
    let config =
        absolute_env_path("XDG_CONFIG_HOME").or_else(|| home.clone().map(|p| p.join(".config")));
    let data = absolute_env_path("XDG_DATA_HOME").or_else(|| home.map(|p| p.join(".local/share")));
    Ok(match shell {
        Shell::Bash => (data.map(|p| p.join("bash-completion/completions")), "memo"),
        Shell::Fish => (config.map(|p| p.join("fish/completions")), "memo.fish"),
        Shell::Zsh => (data.map(|p| p.join("zsh/site-functions")), "_memo"),
        Shell::Elvish => (None, "memo.elv"),
        Shell::PowerShell => (None, "_memo.ps1"),
        _ => bail!("unsupported shell"),
    })
}

fn absolute_destination(path: &Path, cwd: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}

fn safe_write(path: &Path, contents: &[u8], force: bool) -> Result<()> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if !force {
            bail!("refusing to overwrite {}; pass --force", path.display())
        }
        if metadata.is_dir() {
            bail!("refusing to replace directory {}", path.display())
        }
        fs::remove_file(path).with_context(|| format!("remove {}", path.display()))?;
    }
    let parent = path.parent().context("destination has no parent")?;
    fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    let temporary = parent.join(format!(".memo-install-{}", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .with_context(|| format!("create {}", temporary.display()))?;
    let result = (|| -> Result<()> {
        file.write_all(contents)?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        sync_directory(parent)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn require_initialized(selection: &Selection, executable: &Path) -> Result<()> {
    if !marker_initialized(&selection.path)? {
        let command = runnable_command(executable, selection, "init")?;
        bail!(
            "selected store is not initialized: {} ({})\nrun: {command}",
            selection.path.display(),
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

fn normalized_text(text: &str) -> &str {
    text.trim_end_matches(' ')
}

fn sync_directory(path: &Path) -> Result<()> {
    File::open(path)
        .with_context(|| format!("open directory {} for sync", path.display()))?
        .sync_all()
        .with_context(|| format!("sync directory {}", path.display()))
}

fn create_dir_all_synced(path: &Path) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    let parent = path.parent().context("directory has no parent")?;
    create_dir_all_synced(parent)?;
    match fs::create_dir(path) {
        Ok(()) => sync_directory(parent),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error).with_context(|| format!("create directory {}", path.display())),
    }
}

fn locked_store(selection: &Selection) -> Result<File> {
    let path = selection.path.join("store.lock");
    let existed = path.exists();
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .with_context(|| format!("open {}", path.display()))?;
    if !existed {
        file.sync_all()?;
        sync_directory(&selection.path)?;
    }
    file.lock_exclusive()
        .with_context(|| format!("lock {}", path.display()))?;
    Ok(file)
}

fn open_repaired(
    path: &Path,
    validate_last: impl FnOnce(&mut File, u64) -> Result<()>,
) -> Result<(File, bool)> {
    let existed = path.exists();
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .with_context(|| format!("open {}", path.display()))?;
    let len = file.metadata()?.len();
    let complete_slots = len / RECORD_BYTES;
    let mut repair_at = complete_slots;
    if complete_slots > 0 {
        let final_index = complete_slots - 1;
        if is_zero_slot(&read_slot(&mut file, final_index)?) {
            // A crash can leave one or more unwritten full slots. Scan the
            // contiguous zero suffix once, then validate its predecessor
            // before changing the file. The scan is bounded by the file.
            while repair_at > 0 && is_zero_slot(&read_slot(&mut file, repair_at - 1)?) {
                repair_at -= 1;
            }
            if repair_at > 0 {
                validate_last(&mut file, repair_at - 1)?;
            }
        } else {
            validate_last(&mut file, final_index)?;
        }
    }
    let repaired_len = repair_at * RECORD_BYTES;
    if len != repaired_len {
        file.set_len(repaired_len)
            .with_context(|| format!("repair torn suffix in {}", path.display()))?;
        file.sync_all()?;
    }
    file.seek(SeekFrom::Start(repaired_len))?;
    Ok((file, !existed))
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

fn append_note(
    selection: &Selection,
    executable: &Path,
    text: &str,
    out: &mut dyn Write,
) -> Result<()> {
    let text = normalized_text(text);
    validate_text(text, "note")?;
    let _lock = locked_store(selection)?;
    let path = selection.path.join("notes.log");
    let (mut file, created) = open_repaired(&path, |file, id| {
        parse_note(&read_slot(file, id)?, id)
            .with_context(|| format!("malformed complete note record {id}"))?;
        Ok(())
    })?;
    let id = file.metadata()?.len() / RECORD_BYTES;
    if id > 9_999_999_999 {
        bail!("note ID exceeds the version-1 ten-digit limit")
    }
    let prefix = format!("N {id:010} {} ", chrono::Local::now().format("%Y-%m-%d"));
    file.seek(SeekFrom::End(0))?;
    file.write_all(&record(&prefix, text)?)?;
    file.sync_all()?;
    if created {
        sync_directory(&selection.path)?;
    }
    writeln!(
        out,
        "noted {id} in {} ({})",
        selection.path.display(),
        selection.selector
    )?;
    drop(file);
    drop(_lock);
    if let Err(error) = next_nap(selection, executable, out) {
        writeln!(out, "pending maintenance unavailable: {error:#}")?;
    }
    Ok(())
}

fn read_slot(file: &mut File, index: u64) -> Result<[u8; RECORD_BYTES as usize]> {
    file.seek(SeekFrom::Start(index * RECORD_BYTES))?;
    let mut slot = [0; RECORD_BYTES as usize];
    file.read_exact(&mut slot)?;
    Ok(slot)
}

fn is_zero_slot(slot: &[u8; RECORD_BYTES as usize]) -> bool {
    slot.iter().all(|byte| *byte == 0)
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

fn request(
    out: &mut dyn Write,
    selection: &Selection,
    executable: &Path,
    lo: u64,
    span: u64,
) -> Result<()> {
    writeln!(
        out,
        "summary pending for {} ({})",
        selection.path.display(),
        selection.selector
    )?;
    if span == 2 {
        let mut notes = OpenOptions::new()
            .read(true)
            .open(selection.path.join("notes.log"))?;
        for id in [lo, lo + 1] {
            let text = parse_note(&read_slot(&mut notes, id)?, id)
                .with_context(|| format!("malformed complete note record {id}"))?;
            writeln!(out, "source {id}: {text}")?;
        }
    } else {
        let half = span / 2;
        for child in [lo, lo + half] {
            let text = read_summary(selection, child, half)?
                .context("internal error: requested summary has an unsettled child")?;
            writeln!(out, "source {child}-{}: {text}", child + half - 1)?;
        }
    }
    let command = runnable_command(
        executable,
        selection,
        &format!("nap {lo}-{} \"summary\"", lo + span - 1),
    )?;
    writeln!(out, "next: {command}")?;
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
    executable: &Path,
    span_arg: Option<&str>,
    summary: Option<&str>,
    out: &mut dyn Write,
) -> Result<()> {
    if span_arg.is_none() {
        if summary.is_some() {
            bail!("summary requires LO-HI")
        };
        return next_nap(selection, executable, out);
    }
    let summary = normalized_text(summary.context("nap LO-HI requires a summary")?);
    validate_text(summary, "summary")?;
    let (lo, hi, span) = parse_span(span_arg.unwrap())?;
    if hi > 9_999_999_999 {
        bail!("summary range exceeds the version-1 ten-digit limit")
    }
    let _lock = locked_store(selection)?;
    let (notes, _) = open_repaired(&selection.path.join("notes.log"), |file, id| {
        parse_note(&read_slot(file, id)?, id)
            .with_context(|| format!("malformed complete note record {id}"))?;
        Ok(())
    })?;
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
    let summaries = selection.path.join("summaries");
    create_dir_all_synced(&summaries)?;
    let path = summary_path(selection, span);
    let (mut file, created) = open_repaired(&path, |file, index| {
        let old_lo = index * span;
        parse_summary(&read_slot(file, index)?, old_lo, old_lo + span - 1)
            .with_context(|| format!("malformed complete summary record in {}", path.display()))?;
        Ok(())
    })?;
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
    if created {
        sync_directory(&summaries)?;
    }
    writeln!(
        out,
        "settled {lo}-{hi} in {} ({})",
        selection.path.display(),
        selection.selector
    )?;
    Ok(())
}

fn next_nap(selection: &Selection, executable: &Path, out: &mut dyn Write) -> Result<()> {
    let _lock = locked_store(selection)?;
    let notes_path = selection.path.join("notes.log");
    let total = if notes_path.exists() {
        let (notes, _) = open_repaired(&notes_path, |file, id| {
            parse_note(&read_slot(file, id)?, id)
                .with_context(|| format!("malformed complete note record {id}"))?;
            Ok(())
        })?;
        notes.metadata()?.len() / RECORD_BYTES
    } else {
        0
    };
    let mut span = 2;
    while span <= total {
        let path = summary_path(selection, span);
        let count = if path.exists() {
            let (file, _) = open_repaired(&path, |file, index| {
                let lo = index * span;
                parse_summary(&read_slot(file, index)?, lo, lo + span - 1).with_context(|| {
                    format!("malformed complete summary record in {}", path.display())
                })?;
                Ok(())
            })?;
            file.metadata()?.len() / RECORD_BYTES
        } else {
            0
        };
        let available = total / span;
        let children_ready = if span == 2 {
            available
        } else {
            summary_dense_count(selection, span / 2)? / 2
        };
        if count < available && count < children_ready {
            return request(out, selection, executable, count * span, span);
        }
        span = span.checked_mul(2).context("summary span overflow")?;
    }
    writeln!(
        out,
        "no eligible summary pending in {} ({})",
        selection.path.display(),
        selection.selector
    )?;
    Ok(())
}

fn summary_dense_count(selection: &Selection, span: u64) -> Result<u64> {
    let path = summary_path(selection, span);
    let mut file = match OpenOptions::new().read(true).open(&path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error.into()),
    };
    let len = file.metadata()?.len();
    if len % RECORD_BYTES != 0 {
        bail!("summary log has a torn suffix: {}", path.display())
    }
    let count = len / RECORD_BYTES;
    if count > 0 {
        let index = count - 1;
        let lo = index * span;
        parse_summary(&read_slot(&mut file, index)?, lo, lo + span - 1)
            .with_context(|| format!("malformed complete summary record in {}", path.display()))?;
    }
    Ok(count)
}

fn request_prerequisite(
    out: &mut dyn Write,
    selection: &Selection,
    executable: &Path,
    lo: u64,
    span: u64,
) -> Result<()> {
    if span == 2 {
        return request(out, selection, executable, lo, span);
    }
    let half = span / 2;
    for child in [lo, lo + half] {
        if read_summary(selection, child, half)?.is_none() {
            return request_prerequisite(out, selection, executable, child, half);
        }
    }
    request(out, selection, executable, lo, span)
}

fn wake(
    selection: &Selection,
    executable: &Path,
    budget: usize,
    out: &mut dyn Write,
) -> Result<()> {
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
            request_prerequisite(out, selection, executable, l, s)?;
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

fn runnable_command(executable: &Path, selection: &Selection, tail: &str) -> Result<String> {
    Ok(format!(
        "{} --data-dir {} --store {} {tail}",
        shell_quote_path(executable)?,
        shell_quote_path(&selection.data_root)?,
        shell_quote(&selection.selector),
    ))
}

fn shell_quote_path(path: &Path) -> Result<String> {
    let value = path
        .to_str()
        .with_context(|| format!("path is not valid UTF-8: {}", path.display()))?;
    Ok(shell_quote(value))
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
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
            data_root: root.to_path_buf(),
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
        data_root: root.to_path_buf(),
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
