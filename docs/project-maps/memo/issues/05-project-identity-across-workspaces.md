# Project identity across workspaces

Type: research
Status: resolved
Blocked by: [Store boundaries and selection](01-store-boundaries-and-selection.md)

## Question

What stable project-key strategy lets one logical project resolve to one project
store across Git worktrees, jj workspaces, ordinary clones, and repeated command
runs, while avoiding collisions and preserving the outside-repository fallback?

## Answer

Use the canonical shared repository metadata directory, prefixed by its VCS
namespace, and SHA-256 hash that string. For jj this is `.jj/repo` when it is a
directory, or the target of a Bay-style relative `.jj/repo` pointer file. For
Git it is the `.git` directory, or a linked worktree's gitdir resolved through
its `commondir`. This makes workspaces/worktrees sharing repository metadata
select the same key while separate clones remain distinct.

The identity is location-based: moving the shared metadata directory changes
the project ID and therefore its selected store. A colocated jj/Git checkout
currently follows the required same-directory marker precedence (`.jj` wins);
it does not inspect jj's internal Git target to unify identity with Git
worktrees. Both limitations are explicit rather than relying on brittle VCS
subprocesses or undocumented metadata parsing.

## Delivery links

- [First-slice delivery plan](../delivery-plan.md)
