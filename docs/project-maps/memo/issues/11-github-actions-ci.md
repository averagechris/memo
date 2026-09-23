# GitHub Actions CI

Type: implementation
Status: resolved
Blocked by: none

## Question

Which hosted CI service owns repository checks after the external backend is
retired?

## Answer

GitHub Actions is the only hosted CI for `memo`. The workflow runs on pushes to
`main` and pull requests targeting `main`. Its three jobs are `fmt`,
`clippy`, and `test`, each on `ubuntu-24.04`:

- `fmt` runs `cargo fmt --all -- --check` after installing `rustfmt`.
- `clippy` runs `cargo clippy --locked --all-targets --all-features -- -D warnings`
  after installing `clippy`.
- `test` runs `cargo test --locked --all-targets --all-features`.

The workflow uses only the hosted runner's Rust toolchain and the pinned
official checkout action. It has no caches, artifacts, secrets, path filters,
or release automation. Nix packaging and the broader local checks remain local
verification through `jj lint` and the flake. The retired external backend is
not configured. Branch protection required checks are intentionally deferred
until the first workflow run is green.

## Delivery links

- [Project map](../map.md)
- [GitHub Actions workflow](../../../../.github/workflows/ci.yml)
