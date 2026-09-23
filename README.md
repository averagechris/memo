# memo

`memo` keeps short, local context between agent sessions and human work. Use
`note` to save a decision, environment quirk, or workflow lesson, `wake` to
retrieve a bounded context window, and `nap` to record the summary an agent
supplies when `note` or `wake` asks for one. `memo` makes no model calls.

In a repository, `memo` selects a private project store in its data directory.
That keeps project facts separate from cross-project facts. Use
`--store default` for a preference or other fact that should follow you across
projects. The Cargo package and installed executable are both `memo`. The
package is not intended for crates.io publication because `memo` is already
occupied there.

Source code: [github.com/averagechris/memo](https://github.com/averagechris/memo)

## CI

GitHub Actions runs the `fmt`, `clippy`, and `test` checks on pushes to `main`
and pull requests targeting `main`. Local Nix checks remain part of development
verification and are not release automation.

## Current commands

```sh
memo where                         # show selection, path, and initialization state
memo init                          # initialize the selected store (format version 1)
memo --store default init
memo --store project where
memo --store research where        # named store
memo --no-auto-project where
memo --data-dir /private/path where
memo note "one line"
memo wake --lines 96
memo nap                           # print the next eligible, store-pinned request
memo nap 0-1 "summary"
memo skills                       # list bundled OpenCode skills
memo skills show memo             # print the embedded skill Markdown
memo skills install memo          # install one skill globally for OpenCode
memo completions zsh              # print shell completions
memo completions install fish     # install shell completions
```

All subcommands accept the global `-o, --output-format text|json` option before
or after the subcommand. Text is the default. Bare `memo`, help, and version
output remain human-readable; embedded skill Markdown and printed completion
scripts remain raw bytes in text mode.

## Machine-readable output

JSON mode emits one compact JSON object followed by LF. Success uses stdout,
leaves stderr empty, and exits 0; runtime failures leave stdout empty, emit one
error object on stderr, and exit 1. Usage errors selected with `-o json` use the
same atomic stderr rule and exit 2. Every successful object includes `command`
and `ok: true`; command-specific fields carry typed stores, ranges, sources,
items, and install paths rather than wrapping rendered prose.

```sh
memo -o json --store default where
# {"command":"where","ok":true,"store":{"initialized":false,"kind":"default",...}}

memo --store default note "Prefer focused tests" --output-format json
# {"command":"note","ok":true,"id":0,"text":"Prefer focused tests",...}
```

An incomplete JSON `wake` is atomic: prior items and the typed pending request
are included in the `wake_incomplete` error on stderr, while stdout stays empty.
The pending request's `command` is executable- and store-pinned.

## Project map

The [memo project map](docs/project-maps/memo/map.md) records the storage and
command boundaries behind this first slice.

The output-format boundary is resolved in the CLI child: storage operations
return typed outcomes, and only the command edge chooses text rendering or
structured JSON, preserving the version-1 record and locking implementation.

`memo skills install [NAME] [--dir DIR] [--force]` installs one named skill, or
all bundled skills when NAME is omitted. Currently it writes only
`DIR/memo/SKILL.md`; its default root is
`${XDG_CONFIG_HOME:-$HOME/.config}/opencode/skills`. The directory name `memo`
is the OpenCode skill ID.

Completion installs use `${XDG_DATA_HOME:-$HOME/.local/share}` for Bash
(`bash-completion/completions/memo`) and Zsh (`zsh/site-functions/_memo`), and
`${XDG_CONFIG_HOME:-$HOME/.config}` for Fish (`fish/completions/memo.fish`). Zsh
users may need to add that site-functions directory to `fpath`. Elvish and
PowerShell require `--dir`; memo never edits shell configuration. In all cases,
`--dir` names the destination directory and existing files require `--force`.

`where` never creates storage and reports an absent selected store as
`initialized: false`; this diagnostic command is the exception to the normal
uninitialized-store error. `init` is idempotent. An existing `FORMAT_VERSION`
must be a regular file containing exactly `1\n`; malformed or unsupported
markers fail all commands. `note` appends one memory, `wake` renders an aligned
cover within its line budget, and `nap` records a requested power-of-two summary.
`note` and no-argument `nap` eagerly show the next eligible maintenance request,
including its two source texts. `wake` does not require all eager maintenance;
it only requires the summaries in the cover selected for its line budget.
When a required summary is absent, `wake` exits unsuccessfully and prints a
store-pinned `nap` command. `--store project:<64-hex-id>` pins a project store
independently of the current directory.

Version 1 stores local-calendar dates (`YYYY-MM-DD`). `notes.log` and each
`summaries/<span>.log` are readable, append-only logs of 320-byte records
(including LF), with ten-digit decimal IDs/ranges and space padding. Text is one
nonempty line of at most 280 UTF-8 bytes; trailing ASCII spaces are normalized
before storage and retry comparison. A per-store advisory lock serializes
writes; acknowledgements follow `fsync`. A torn final slot is truncated under
that lock after validating the last complete slot, while malformed complete
records are errors when accessed. Summary levels are dense,
aligned, write-once prefixes: two-note summaries read raw notes and every larger
summary requires its two child summaries. Identical retries are idempotent.

## Data layout and selection

The default root is `${XDG_DATA_HOME:-$HOME/.local/share}/memo/stores`. Empty or
relative XDG base-directory values are treated as unset. Override it with a
nonempty `MEMO_DATA_DIR` or, with higher precedence, `--data-dir`. Stores live at:

- `default/` for the user store;
- `projects/<sha256-id>/` for a repository store; and
- `named/<name>/` for an explicitly named store.

Project selection is enabled by default. Configure `auto_project = false` in
`${XDG_CONFIG_HOME:-$HOME/.config}/memo/config.toml`, or override it with
`--auto-project` / `--no-auto-project`. An explicit `--store` always wins.
Project IDs derive from canonical shared jj or Git metadata paths, so linked
worktrees/workspaces share a store, while moving a repository changes its ID.

This executable has the same name as OptMem's executable. It does **not** read
OptMem's `MEMORY_DIR`, and its storage is not compatible with OptMem storage.

Not yet implemented: summary review/undo, forget/recall/zoom/import, combined
multi-store wake, model integration, daemons, background services, or networking.

## Development

```sh
direnv allow   # or: nix develop
cargo test -q
nix run .#static-checks
```
