# First-slice delivery plan

This is a short delivery sequence for the `memo` first slice. It is subordinate
to the [project map](map.md), and it is discovery output, not permission to
implement or publish.

## 1. Core commands and store selection

Implement and verify `init`, `where`, `note`, `wake`, and `nap` after the store
selection, project identity, and initialization decisions are resolved. Keep the
default user store, automatic project store, and named stores separate. Verify
`auto_project=true`, CLI true/false overrides, explicit `--store`, outside-repo
fallback, and the error for an uninitialized project marker.

Depends on [store boundaries](issues/01-store-boundaries-and-selection.md),
[project markers](issues/02-project-markers-and-initialization.md), and
[project identity](issues/05-project-identity-across-workspaces.md).

## 2. Read and repair

Add the durable layout, write protocol, crash repair, and first-slice review or
undo behavior. Verify that each summary remains owned by one store and that a
failed write cannot silently create or corrupt a store.

Depends on [layout and repair](issues/06-layout-and-crash-repair.md) and
[review and undo](issues/07-review-and-summary-undo.md).

## 3. Combined view

Add the later opt-in, explicitly labeled combined wake. Define its eligible stores
and budget first. Verify that it reads across stores without persisting a merged
summary or changing any store's ownership.

Depends on [read and summary boundaries](issues/03-read-and-summary-boundaries.md)
and [combined wake budget](issues/08-combined-wake-budget-and-default.md).

## 4. Packaging and verification

Resolve the package name and release path, then run the repository's formatting,
clippy, test, package-build, and release-artifact checks. Keep personal data out
of the project tree and do not publish as part of this discovery or implementation
sequence.

Depends on [package name and release path](issues/09-package-name-and-release-path.md).
