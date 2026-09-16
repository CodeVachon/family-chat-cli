---
id: 24
title: Create Ratatui layout skeleton
state: Done
parent: 5
assignee: Christopher Vachon
labels: [task, tui]
blockedBy: [12, 13, 49]
created: 2026-08-25T21:38:35Z
updated: 2026-09-16T19:56:59Z
---

## Description

Build the initial full-screen terminal layout for channels, messages, users, and compose input.

## Plan

Implement the responsive layout specified by task 49 with panes for channels, messages, users, compose input, and status/help. At widths below the minimum, render a clear size warning instead of overlapping content. Verify snapshots or render-buffer assertions for narrow, minimum, and wide dimensions, including Unicode content.

## Notes

### 2026-09-16T19:56:59Z — Christopher Vachon (user)

Implemented in src/tui/layout.rs (top-level vertical split into body/status, body split horizontally into sidebar/main) and src/tui/widgets/mod.rs (centered_rect helper for modal-style screens like the login form and NotApproved message). Verified visually in a tmux pty at 100x30.
