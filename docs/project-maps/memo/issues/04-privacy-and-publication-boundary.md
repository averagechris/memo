# Privacy and publication boundary

Type: grilling
Status: resolved
Blocked by: none

## Question

Which data-handling and publication behaviors are allowed in the first slice?

## Answer

Personal data stays outside project VCS. `memo` does not automatically run Git
operations or publish to a code host. It also does not copy source code or prompts
from an upstream project whose license is not declared.

These constraints apply during development and packaging. A future explicit
release process may publish the `memo` project, but that is separate from runtime
behavior and is not authorized by this map.

## Delivery links

- [First-slice delivery plan](../delivery-plan.md)
