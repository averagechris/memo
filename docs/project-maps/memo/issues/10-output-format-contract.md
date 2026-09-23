# Output format contract

Type: grilling
Status: resolved
Blocked by: none

## Question

What global output-format contract should the CLI expose, including defaults,
successful output, errors, incomplete wake results, raw text commands, and
help/version output?

## Answer

Use the global `-o, --output-format` option with `text|json` values and a
default of `text`. In JSON mode, a successful command writes one typed object
to stdout. A runtime or parser error writes one JSON error object to stderr and
nothing to stdout. An incomplete wake exits nonzero and reports its pending
source records and command. Raw skill and completion text remains raw unless
JSON mode is explicitly selected. Help and version output is always text.

The rationale is to keep the existing human-readable CLI as the default while
making structured mode a predictable, stream-safe contract for automation;
diagnostic output must not be mixed into a successful JSON stdout stream.

Tests and implementation evidence: `tests/cli.rs` and `src/main.rs`.

## Notes

The current implementation provides the structured mode described above.

## Delivery links

- [First-slice delivery plan](../delivery-plan.md)
