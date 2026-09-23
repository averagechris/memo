# memo project guidance

Use `jj` for version-control actions in this repository.

## Hosting status

- GitHub is the canonical code host: https://github.com/averagechris/memo
- GitHub Actions CI is active on pushes to `main` and pull requests targeting
  `main`. The workflow checks are `fmt`, `clippy`, and `test`; configure branch
  protection required checks only after the first green workflow run.
- Run `jj lint` and the relevant local Nix checks before handing off changes.

## Development

- Enter the toolchain with `direnv allow` or `nix develop`.
- Nix formatting uses wrapped `alejandra -q`; run `nix fmt` or `nix fmt -- --check .`.
- Prefer local checks: `nix run .#static-checks` (fmt + clippy), `nix run .#ci-test`, `nix run .#ci-machete`, `nix run .#ci-sort`, `nix run .#ci-deny`, `nix run .#ci-audit`.
- Before handoff, run `jj lint`, `cargo test -q`, and the smallest relevant local
  Nix checks below. The GitHub workflow does not package the Nix derivation.

## sccache

The host sets `RUSTC_WRAPPER=sccache` globally, and it must never be unset to "fix"
build failures. If builds fail with sccache connection or compiler errors, run
`sccache --stop-server` and retry; the supervised launchd agent restarts a healthy
server.

## Release workflow

No release automation or publication process is currently defined. The GitHub
workflow only runs repository checks. Do not tag or publish a release without an
approved process.

## Issue tracking

Tracker selection and provisioning are undecided. Do not advertise a tracker or
label until one has been created for memo.
