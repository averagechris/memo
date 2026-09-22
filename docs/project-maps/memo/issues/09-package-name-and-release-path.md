# Package name and release path

Type: grilling
Status: open
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

## Delivery links

- [First-slice delivery plan](../delivery-plan.md)
