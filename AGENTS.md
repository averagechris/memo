# memo project guidance

Use `jj` for version-control actions in this repository.

## Hosting status

- GitHub is the canonical code host: https://github.com/averagechris/memo
- GitHub pushes currently have no CI workflow. Run `jj lint` and the relevant
  local Nix checks before handing off changes.
- The SourceHut CI, release, and Pages manifests remain as dormant scaffolding.
  They are not active for memo until someone deliberately provisions the
  required SourceHut services and mirror.
- Do not run release, tag, or build-submission apps, and do not describe the
  SourceHut manifests as active, until that provisioning is complete.

## Development

- Enter the toolchain with `direnv allow` or `nix develop`.
- Nix formatting uses wrapped `alejandra -q`; run `nix fmt` or `nix fmt -- --check .`.
- Prefer local checks: `nix run .#static-checks` (fmt + clippy), `nix run .#ci-test`, `nix run .#ci-machete`, `nix run .#ci-sort`, `nix run .#ci-deny`, `nix run .#ci-audit`.
- Before handoff, run `jj lint` plus the smallest relevant checks below.

## sccache

The host sets `RUSTC_WRAPPER=sccache` globally, and it must never be unset to "fix"
build failures. If builds fail with sccache connection or compiler errors, run
`sccache --stop-server` and retry; the supervised launchd agent restarts a healthy
server.

## Release workflow

The standard averagechris fleet interface remains available but dormant:

```sh
nix run .#prepare-release -- --version X.Y.Z
nix run .#release-tag
nix build .#release-artifact
nix run .#static-checks
nix run .#release -- --version X.Y.Z --submit-linux-build
```

Do not execute these release commands until the release backend is deliberately
provisioned. Keep `builds/release-linux-x86_64.yml` outside `.builds/`; it is
dormant explicit-submit scaffolding, not an active release path.

## Issue tracking

Tracker selection and provisioning are undecided. Do not advertise a tracker or
label until one has been created for memo.
