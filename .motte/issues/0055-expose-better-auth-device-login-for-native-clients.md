---
id: 55
title: Expose Better Auth device login for native clients
state: Todo
parent: 3
labels: [task, auth, backend, external, mvp]
created: 2026-08-26T01:33:08Z
updated: 2026-08-26T01:33:08Z
---

## Description

Add the server capability required for a terminal client to obtain the Better Auth session token accepted by family-chat PR #81 without a localhost callback.

## Plan

In the family-chat backend, enable Better Auth standalone Device Authorization for first-party session tokens, migrate its device-code schema, define and validate the CLI client id, add the remote verification and approval page, document /device/code and /device/token, and test pending, approval, denial, expiry, slow polling, and approved-user enforcement. Keep bearer() enabled so the resulting session token authenticates /api/v1. Do not add OAuth Provider unless a future resource-server boundary requires OAuth access tokens.
