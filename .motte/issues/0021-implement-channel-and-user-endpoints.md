---
id: 21
title: Implement channel and user endpoints
state: Todo
parent: 4
labels: [task, api]
blockedBy: [20]
created: 2026-08-25T21:38:34Z
updated: 2026-09-16T20:18:59Z
---

## Description

Expose typed methods for listing channels and users/presence data.

## Plan

Add list_channels, get_channel if needed, list_users, and current_user methods.

## Notes

### 2026-09-16T20:18:59Z — Christopher Vachon (user)

Partial: list_channels and current-user (me, which also returns preferences/unread) are implemented in src/api/client.rs and used by the TUI. list_users isn't implemented — that's #50's job (load users and presence into the TUI), which hasn't been picked up yet.
