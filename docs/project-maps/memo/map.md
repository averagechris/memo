## Destination

Produce a delivery-ready first-slice specification for `memo`: its store-selection
rules, project identity, durable storage and repair contract, review boundaries,
and staged implementation plan. This is discovery work only. It does not authorize
implementation, publication, or release.

## Notes

The repository now provides the bounded single-store `where`, `init`, `note`,
`wake`, and `nap` loop, project-pinned selectors, and the verified version-1
fixed-record layout. Review/undo and combined wake remain future work.

The XDG data root lives under the tool name, with an override. Personal data stays
outside project VCS. The project must not automate Git, publish to SourceHut, or
copy source or prompts from an upstream project that has no declared license.

The Cargo package is `memo-cli` because `memo` is occupied on crates.io; the
binary and repository remain `memo`. No publication is authorized.

Decision frontier: [issues/](issues/)

Agents scan this directory for open, unblocked child decisions.

The staged route is in the [first-slice delivery plan](delivery-plan.md).

## Decisions so far

- [Store boundaries and selection](issues/01-store-boundaries-and-selection.md): Keep default-user, automatically selected project, and named stores physically separate; explicit store selection wins.
- [Project markers and initialization](issues/02-project-markers-and-initialization.md): A recognized project marker with no initialized store is an error; only `init` creates a project store.
- [Read and summary boundaries](issues/03-read-and-summary-boundaries.md): Ordinary reads use one selected store; a later labeled combined wake is opt-in and never persists a mixed summary.
- [Privacy and publication boundary](issues/04-privacy-and-publication-boundary.md): Keep personal data outside VCS and avoid automatic Git, SourceHut, upstream-source, and upstream-prompt actions.
- [Project identity](issues/05-project-identity-across-workspaces.md): Hash the namespaced canonical shared VCS metadata path; shared workspaces match, while moving a repository changes identity.
- [Package name and release path](issues/09-package-name-and-release-path.md): Use Cargo package `memo-cli` with binary/fleet identity `memo`; do not publish as part of this work.
- [Durable layout and crash repair](issues/06-layout-and-crash-repair.md): Use locked, fsynced 320-byte append-only records, truncate only torn suffixes, and reject malformed complete records.

## Not yet specified

- The exact command output, persistence schema, and verification matrix depend on the open decisions in `issues/`.
- The package and release workflow must be settled before packaging work starts. Discovery does not publish anything.

## Out of scope

- Automatic Git operations or SourceHut publication. These are excluded from the first slice and from the tool's normal behavior.
- Copying source code or prompts from an upstream project without a declared license.
- Mixing summaries from multiple stores into a persisted record. A combined view may be read later, but its merged result is not stored.
- Treating a project marker as permission to create storage. Store creation belongs to `init`.
