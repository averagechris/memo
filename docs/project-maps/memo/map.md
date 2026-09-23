## Destination

Produce a delivery-ready first-slice specification for `memo`: its store-selection
rules, project identity, durable storage and repair contract, review boundaries,
and staged implementation plan. This is discovery work only. It does not authorize
implementation, publication, or release.

## Notes

The repository provides the bounded single-store `where`, `init`, `note`, `wake`,
and `nap` loop, project-pinned selectors, and the verified version-1 fixed-record
layout. Review/undo and combined wake remain future work.

The XDG data root lives under the tool name, with an override. Personal data stays
outside project VCS. The project must not automate Git, publish to a code host, or
copy source or prompts from an upstream project that has no declared license.

The Cargo package, binary, and repository are all `memo`. The name `memo` is
already occupied on crates.io, so do not publish this package there; the local
Cargo package name remains `memo`. No publication is authorized.

Decision frontier: [issues/](issues/)

Agents scan this directory for open, unblocked child decisions.

The staged route is in the [first-slice delivery plan](delivery-plan.md).

## Decisions so far

- [Store boundaries and selection](issues/01-store-boundaries-and-selection.md): Keep default-user, automatically selected project, and named stores physically separate; explicit store selection wins.
- [Project markers and initialization](issues/02-project-markers-and-initialization.md): A recognized project marker with no initialized store is an error; only `init` creates a project store. The read-only diagnostic `where` is the exception and reports the selected path and `initialized: false`.
- [Read and summary boundaries](issues/03-read-and-summary-boundaries.md): Ordinary reads use one selected store; a later labeled combined wake is opt-in and never persists a mixed summary.
- [Privacy and publication boundary](issues/04-privacy-and-publication-boundary.md): Keep personal data outside VCS and avoid automatic Git, code-host publication, upstream-source, and upstream-prompt actions.
- [Project identity](issues/05-project-identity-across-workspaces.md): Hash the namespaced canonical shared VCS metadata path; shared workspaces match, while moving a repository changes identity.
- [Package name and release path](issues/09-package-name-and-release-path.md): Use `memo` for the local Cargo package and binary; do not publish to crates.io as part of this work because `memo` is occupied there.
- [Output format contract](issues/10-output-format-contract.md): Use global `-o, --output-format text|json`, defaulting to text, with typed JSON success/error and incomplete-wake contracts while help/version remain text.
- [Durable layout and crash repair](issues/06-layout-and-crash-repair.md): Use locked, fsynced 320-byte append-only records, truncate only torn suffixes, and reject malformed complete records.

## Not yet specified

- [Review and summary undo](issues/07-review-and-summary-undo.md) remains open.
- [Combined wake budget and default](issues/08-combined-wake-budget-and-default.md) remains open.
- Any release or publishing plan remains subject to approval; this work does not authorize publication.

## Out of scope

- Automatic Git operations or code-host publication. These are excluded from the first slice and from the tool's normal behavior.
- Copying source code or prompts from an upstream project without a declared license.
- Mixing summaries from multiple stores into a persisted record. A combined view may be read later, but its merged result is not stored.
- Treating a project marker as permission to create storage. Store creation belongs to `init`.
