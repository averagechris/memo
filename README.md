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
```

`where` never creates storage. `init` is idempotent and does not replace an
existing format marker. `note`, `wake`, and `nap` are planned future commands;
the current release does not store memories.

## Data layout and selection

The default root is `${XDG_DATA_HOME:-$HOME/.local/share}/memo/stores`. Override
it with `MEMO_DATA_DIR` or, with higher precedence, `--data-dir`. Stores live at:

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

## Development

```sh
direnv allow   # or: nix develop
cargo test -q
nix run .#static-checks
```

Work is tracked at <https://todo.sr.ht/~averagechris/projects> with the
`repo:memo` label.
