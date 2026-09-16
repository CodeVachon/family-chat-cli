---
id: 55
title: Expose Better Auth device login for native clients
state: Done
parent: 3
labels: [task, auth, backend, external, mvp]
created: 2026-08-26T01:33:08Z
updated: 2026-09-16T20:11:56Z
---

## Description

Add the server capability required for a terminal client to obtain the Better Auth session token accepted by family-chat PR #81 without a localhost callback.

## Plan

In the family-chat backend, enable Better Auth standalone Device Authorization for first-party session tokens, migrate its device-code schema, define and validate the CLI client id, add the remote verification and approval page, document /device/code and /device/token, and test pending, approval, denial, expiry, slow polling, and approved-user enforcement. Keep bearer() enabled so the resulting session token authenticates /api/v1. Do not add OAuth Provider unless a future resource-server boundary requires OAuth access tokens.

## Notes

### 2026-09-16T20:11:56Z — Christopher Vachon (user)

Superseded: 2026-09-16 Chris confirmed email/password is fine ("this is a tool for me to play with, not something we'll be distributing") — see #3's note. No backend device-authorization work is needed; the CLI authenticates via POST /api/auth/sign-in/email against the bearer plugin already deployed, which is implemented and working (see #15, manually verified against the live server).
