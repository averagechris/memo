# memo

`memo` is a local, personal-memory CLI for AI-agent workflows. The Cargo package
is `memo-cli`; the installed executable is `memo`.

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
```

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

Work is tracked at <https://todo.sr.ht/~averagechris/projects> with the
`repo:memo` label.
