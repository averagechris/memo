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
locked, append, and `fsync` both data and newly created directory entries before
acknowledging. The append path normally validates the last complete record in
the target dense prefix in O(1), then truncates only an incomplete trailing
slot. A final complete slot containing exactly 320 NUL bytes is also treated as an
unacknowledged torn suffix: the contiguous zero suffix is discarded, after
validating its preceding final complete record (if any), and the file is synced.
Other malformed complete slots are hard errors and leave the file byte-identical;
earlier corruption is reported when that slot is read (and belongs in a future
explicit verification command), rather than making every later append
scan-bound or unavailable. Summary levels are aligned, dense, and write-once.
Two-note summaries are the leaf level; larger parents require both children.
Pending maintenance uses dense-prefix counts and examines one next slot per
level, so it is O(log N). Identical normalized retries are idempotent and
conflicting retries fail.

The executable integration checks in `tests/cli.rs` cover fixed-width concurrent
IDs, torn-suffix and zero-slot repair, malformed complete records, child ordering,
text limits, scoped pending prompts, store validation, and project identity across
a Bay-style jj workspace.

## Delivery links

- [First-slice delivery plan](../delivery-plan.md)
