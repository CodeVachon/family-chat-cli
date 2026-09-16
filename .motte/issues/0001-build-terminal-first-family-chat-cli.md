---
id: 1
title: Build terminal-first family chat CLI
state: Todo
labels: [epic, product-plan, tui]
created: 2026-08-25T21:37:05Z
updated: 2026-08-25T21:47:22Z
---

## Description

Create a native terminal interface for a REST-backed chat application. The CLI should focus on a full-screen TUI, authenticate against Better Auth without installing a local web app, and expose channels, users, and messages in a single native binary.

## Plan

Implement a Rust MVP with Ratatui and Crossterm, Tokio, Reqwest and Serde, a Better Auth OAuth device grant, and secure credential storage. MVP means login/logout/refresh, list and select channels, show users when supported, load and update messages, compose and send, recover from common errors, and ship as one native TUI binary. Search, unread indicators, and broad package-manager distribution are post-MVP. Resolve the deployed auth and chat API contracts first, then architecture, vertical TUI/API slices, resilience, and packaging. Before any task is completed, its plan must state a reproducible verification result.

## Notes

### 2026-08-25T21:39:41Z — codex-mcp-client (agent)

Planning decision: target a native terminal-first Rust application using Ratatui/Crossterm. Authentication should use Better Auth OAuth device authorization so users approve in their existing browser against the remote auth service; the CLI must not install or run a companion local web app.

### 2026-08-25T21:47:22Z — Christopher Vachon (user)

Plan validation (2026-08-25): architecture recommendation is sound and Better Auth device authorization is supported, conditional on the remote server/plugin/client prerequisites captured in task 14. The backlog is now risk-first: establish auth and chat API contracts plus the TUI interaction spec before fixing dependencies or implementing panes. MVP is login, channels, users, message history/updates, send, resilience, and one native TUI binary; search, unread indicators, and broad packaging are post-MVP. Execution rule: refine each claimed task with reproducible verification before marking it Done.
