---
id: 23
title: Map API errors to user-facing TUI errors
state: Todo
parent: 4
labels: [task, api]
blockedBy: [20]
created: 2026-08-25T21:38:34Z
updated: 2026-09-16T19:57:20Z
---

## Description

Convert HTTP/network/auth/rate-limit errors into actionable application errors.

## Plan

Define display-ready error categories for offline, unauthorized, forbidden, validation, server, and unexpected failures.

## Notes

### 2026-09-16T19:57:20Z — Christopher Vachon (user)

Partial, as a byproduct of #20/#15: ApiError has display-ready Server/Unauthorized/NotApproved/Forbidden/Request(network) variants, and login-time errors show the server's own message (see #15's note on the from_auth_response vs from_response split). Not yet covered: a distinct 'validation' (422 + issues) category — nothing in this prototype posts data yet, so no caller produces a 422 to design against. Natural to pick this back up alongside #31 (compose and send messages), which will be the first thing that can 422.
