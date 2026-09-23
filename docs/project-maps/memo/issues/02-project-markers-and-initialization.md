# Project markers and initialization

Type: grilling
Status: resolved
Blocked by: [Store boundaries and selection](01-store-boundaries-and-selection.md)

## Question

What should happen when `memo` recognizes a project marker but the corresponding
project store has not been initialized?

## Answer

Recognition and creation are separate operations. If a project marker is present
but its store is uninitialized, commands must return an error except for the
read-only diagnostic `where`, which reports the selected path and
`initialized: false`. Automatic
selection must not create files or a store as a side effect. Only `init` creates a
project store.

The marker's stable identity and the durable marker-to-store mapping are still
research questions in [project identity](05-project-identity-across-workspaces.md)
and [layout and repair](06-layout-and-crash-repair.md).

## Delivery links

- [First-slice delivery plan](../delivery-plan.md)
