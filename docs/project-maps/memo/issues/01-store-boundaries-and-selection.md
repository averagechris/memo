# Store boundaries and selection

Type: grilling
Status: resolved
Blocked by: none

## Question

Which store classes must remain physically separate, and how does `memo` select
one store for a command when the caller is in or outside a project?

## Answer

`memo` has three physically separate store classes:

1. A default user store.
2. An automatically selected project store.
3. A named store selected explicitly by the caller.

The data root is under the tool name and has an override. One project maps to one
project store, including its Git task workspaces and jj workspaces. Outside a
recognized repository, selection falls back to the default user store.

The built-in configuration is `auto_project=true`. The CLI can override it with
explicit `true` or `false` values. An explicit `--store` selection chooses the
default, project, or named store and takes precedence over automatic selection.

The stable project key and the on-disk representation of these stores remain open
in [project identity](05-project-identity-across-workspaces.md) and
[layout and repair](06-layout-and-crash-repair.md).

## Delivery links

- [First-slice delivery plan](../delivery-plan.md)
