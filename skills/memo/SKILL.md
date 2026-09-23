---
name: memo
description: Maintain durable, verified workflow context across agent sessions.
---
# memo

Use memory for durable, verified, nonredundant user or project workflow context—not a task log or source of truth.

- Run `memo wake` before relying on remembered context.
- Follow a store-pinned `memo nap` prompt with a faithful summary, then rerun `memo wake`.
- Use `memo note "…"` only for lasting insights.
- Use `memo where` to check the selected store.
- Never store secrets.
- Subagents should not add unsolicited or duplicate notes.
