---
id: 53
title: Configure project subagents
state: Done
parent: 8
assignee: Christopher Vachon
labels: [task, tooling, agents]
created: 2026-08-25T22:06:56Z
updated: 2026-08-25T22:11:07Z
---

## Description

Add project-scoped Codex custom agents for implementation, code review, and Motte-based project management.

## Plan

Create supported project agent TOML files, set a bounded concurrent-agent limit, document the handoff workflow in AGENTS.md, validate TOML parsing and required fields, and record usage guidance.

## Notes

### 2026-08-25T22:10:05Z — codex-mcp-client (agent)

Configured project-scoped Codex agents under .codex/agents: developer (workspace-write, one claimed Motte task, implementation plus verification), code_reviewer (read-only, findings-first review), and project_manager (workspace-write limited by instructions to Motte planning and coordination). Added a three-thread project limit and documented the plan/develop/review/fix handoff in AGENTS.md. Verification: tomlq parsed all four TOML files and asserted required fields; Codex CLI 0.149.1 accepted the project config during help invocation; motte doctor passed with 53 issues. A strict-config feature-list probe was not usable because that Codex subcommand does not support --strict-config; this was a command limitation, not a configuration parse failure.
