# Durable layout and crash repair

Type: research
Status: resolved
Blocked by: [Project markers and initialization](02-project-markers-and-initialization.md), [Project identity across workspaces](05-project-identity-across-workspaces.md)

## Question

What versioned on-disk file layout, atomic write protocol, locking or concurrency
rule, and crash-repair procedure should protect stores under the overridable XDG
data root without creating an uninitialized store during selection?

## Answer

Version 1 uses an exact `FORMAT_VERSION` value of `1\n`, `notes.log`, and dense
per-span `summaries/<span>.log` files. Every slot is exactly 320 bytes including
LF, space padded, with a distinct `N` or `S` prefix, ten-digit decimal IDs or
ranges, and a local-calendar ISO date. Text is one nonempty line of at most 280
UTF-8 bytes. This keeps records inspectable while allowing constant-offset seeks.

One advisory lock file scopes concurrency to one store. Writers validate the
initialized marker before creating data files, assign IDs while exclusively
locked, append, and `fsync` before acknowledging. Under that lock only an
incomplete trailing slot is truncated; a complete malformed slot is a hard
error. Summary levels are aligned, dense, and write-once. Two-note summaries are
the leaf level; larger parents require both children. Identical retries are
idempotent and conflicting retries fail.

The executable integration checks in `tests/cli.rs` cover fixed-width concurrent
IDs, torn-suffix repair, malformed complete records, child ordering, text limits,
scoped pending prompts, store validation, and project identity across a Bay-style
jj workspace.

## Delivery links

- [First-slice delivery plan](../delivery-plan.md)
