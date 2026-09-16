---
id: 18
title: Add logged-out and auth-error TUI states
state: Done
parent: 3
assignee: Christopher Vachon
labels: [task, auth]
blockedBy: [15]
created: 2026-08-25T21:38:34Z
updated: 2026-09-16T19:56:50Z
---

## Description

Make auth status visible in the terminal UI.

## Plan

Render: an initial login form (email + masked password fields) when logged out, an inline error state for invalid credentials, a distinct 'pending approval' state for a valid login that isn't yet admin-approved (403), a network-error/retry state, and a logout action from within the authenticated TUI.

## Notes

### 2026-09-16T19:56:49Z — Christopher Vachon (user)

Implemented in src/tui/widgets/mod.rs: login form (email/password fields, focus highlighting, inline error text), a distinct NotApproved screen (since the server doesn't distinguish pending vs rejected in its message, see docs/api-contract.md), and a Resuming/'Connecting…' screen shown before the stored-session check resolves. Verified manually in a tmux pty against the live server.
