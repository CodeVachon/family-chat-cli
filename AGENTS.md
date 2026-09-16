# Agent instructions

<!-- motte:start -->

## Tracking work with motte

This project's work lives in `.motte/issues/` as Markdown files, committed alongside the code. Track
work there rather than in an ad-hoc TODO list, a commit message, or a pull request description.

Start by asking what to do:

```
motte next --why
```

That is not `motte list`, and not quite `motte ready` either. **Ready** means unsettled with every
blocker settled; `next` orders that set by what a piece of work would unblock, how close it is to a leaf,
and how long it has waited — and it leaves out anything somebody else holds. `motte ready --blocked` shows
what is waiting and on what.

The loop for an issue you pick up:

1. Claim it — `motte claim <ref>`. If that fails, somebody else is on it: ask `motte next` again.
2. Read it — `motte show <ref>`
3. Refine its **Plan** if the plan on the file is not what you are actually going to do
4. Add notes as you go, especially for decisions and dead ends — `motte note <ref> "..."`
5. Move it to Done when the verification for that issue passes, or `motte release <ref>` if you stop

A `<ref>` is an issue number or a fragment of the title, so `motte show parser` works as well as
`motte show 12`.

If you discover a prerequisite mid-task, record it with `motte block <ref> <blocker>` rather than
describing it in prose. Prose is not queryable, and `motte ready` is what the next agent reads.

Every read command accepts `--json`. The MCP server exposes the same operations, and notes written
through it are attributed to the agent rather than to the repository's git user — which is the point:
one shared record of who decided what.

<!-- motte:end -->

## Subagent workflow

Project-scoped Codex roles live in `.codex/agents/`:

- `project_manager` maintains Motte scope, sequencing, blockers, and delivery status.
- `developer` claims and implements one bounded Motte task with verification.
- `code_reviewer` performs a read-only review after implementation.

For implementation work that needs the full workflow:

1. Ask `project_manager` to validate the selected task and its prerequisites when planning is needed.
2. Ask `developer` to claim and implement the task. It leaves the issue In Progress for review.
3. Ask `code_reviewer` to review the resulting diff against the Motte issue.
4. Route findings back to `developer`; once verification passes and findings are resolved, record the review outcome in Motte and move the issue to Done.

Parallelize independent research, review, and test analysis. Do not run multiple write-enabled agents against overlapping files. Use `/agent` in Codex CLI to inspect or switch among active agent threads.
