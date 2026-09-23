# Package name and release path

Type: grilling
Status: resolved
Blocked by: none

## Question

The repository currently declares package and binary name `memo`, while the
working assumption is that `memo-cli` may be needed because `memo` is occupied on
crates.io. Before any publication, should the Cargo package keep `memo` or use a
distinct package name such as `memo-cli`, and what verification and release path
should establish that choice without publishing during discovery? Verify the
registry state, package/binary naming relationship, release artifact expectations,
and the existing repository release workflow before choosing.

## Answer

The Cargo package is `memo-cli`, because `memo` is occupied on crates.io. The
binary, repository, and fleet identities remain `memo`; Nix's Cargo lock-package
selection uses `memo-cli`. The normal repository checks and release-artifact
workflow verify that relationship. This decision does not authorize or perform
publication.

## Delivery links

- [First-slice delivery plan](../delivery-plan.md)
