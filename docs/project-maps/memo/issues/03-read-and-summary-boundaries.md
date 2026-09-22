# Read and summary boundaries

Type: grilling
Status: resolved
Blocked by: [Store boundaries and selection](01-store-boundaries-and-selection.md)

## Question

How should ordinary reads and future combined wakes treat summaries when more than
one store exists?

## Answer

Every summary belongs to exactly one store. The first slice reads one selected
store at a time. It does not silently combine the default, project, and named
stores.

A later feature may provide an explicitly labeled combined wake. That view must
identify its combined nature, and its merged result must not be persisted as a
summary in any store. Its included stores, budget, and default behavior remain open
in [combined wake budget and default](08-combined-wake-budget-and-default.md).

The review and undo contract for individual summaries remains open in
[review and summary undo](07-review-and-summary-undo.md).

## Delivery links

- [First-slice delivery plan](../delivery-plan.md)
