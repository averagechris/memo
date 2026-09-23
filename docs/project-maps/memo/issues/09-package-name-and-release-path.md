# Package name and release path

Type: grilling
Status: resolved
Blocked by: none

## Question

The repository uses the package and binary name `memo`. The name `memo` is
already occupied on crates.io, so this work must not publish a crate there. The
local package name remains `memo`; verify the package/binary naming relationship,
release artifact expectations, and existing repository release workflow without
publishing during discovery.

## Answer

The Cargo package, binary, and repository are all named `memo`.
The name `memo` is already occupied on crates.io, so do not publish this package
there; that does not prevent using `memo` as the local Cargo package name. Nix's
Cargo lock-package selection uses `memo`. The normal repository checks verify
that relationship. This decision does not authorize or perform publication.

## Delivery links

- [First-slice delivery plan](../delivery-plan.md)
