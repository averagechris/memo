# Durable layout and crash repair

Type: research
Status: open
Blocked by: [Project markers and initialization](02-project-markers-and-initialization.md), [Project identity across workspaces](05-project-identity-across-workspaces.md)

## Question

What versioned on-disk file layout, atomic write protocol, locking or concurrency
rule, and crash-repair procedure should protect stores under the overridable XDG
data root without creating an uninitialized store during selection?

## Answer

The first implementation slice reserves an idempotently created
`FORMAT_VERSION` file containing `1` as the store initialization marker. No raw
log or durable record schema is chosen yet; those layout and repair decisions
remain open.

## Delivery links

- [First-slice delivery plan](../delivery-plan.md)
