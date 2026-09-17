---
id: 5
title: "Epic: Core TUI shell"
state: Done
parent: 1
labels: [epic, tui]
created: 2026-08-25T21:37:48Z
updated: 2026-09-17T15:52:35Z
---

## Description

Build the native full-screen terminal interface, navigation model, rendering, and event loop.

## Plan

Create a Ratatui application shell with panes for channels, messages, users, and compose input. Implement keyboard navigation, focus management, loading/error states, resize handling, and clean terminal restore on exit.

## Notes

### 2026-09-17T15:52:35Z — Christopher Vachon (user)

All child issues (24/25/26/27/28/49/58) are Done. The epic's plan also mentions a 'users' pane, but that's tracked separately as #50 (parented under #6, Chat workflows) rather than as a child here — not blocking this epic's own completion.
