---
id: 3
title: "Epic: Better Auth email/password login"
state: Todo
parent: 1
labels: [epic, auth]
created: 2026-08-25T21:37:48Z
updated: 2026-09-16T19:02:52Z
---

## Description

Implement login/logout for a single-user CLI client (personal tool, not distributed) against a Better Auth server that only exposes bearer(), magicLink(), and passkey() — no OAuth/device-authorization plugin.

## Plan

Confirmed 2026-09-16: email+password is the chosen flow (Chris's call — this is a personal tool, not a distributed client, so a terminal password prompt is acceptable UX). CLI collects email+password, POSTs to /api/auth/sign-in/email, captures the session token from the set-auth-token response header, and sends it back as Authorization: Bearer <token> on every /api/v1 request. There is no refresh-token grant — Better Auth sessions are sliding-expiry (extended server-side on use); the CLI's job on expiry is to detect the 401 and re-prompt, not to refresh a token itself. Logout calls POST /api/auth/sign-out then clears the local keychain entry.

## Notes

### 2026-09-16T18:52:12Z — Christopher Vachon (user)

Contract inventory (see #48, docs/api-contract.md) found the deployed Better Auth server has no OAuth/device-authorization plugin — only bearer(), magicLink(), and passkey(). This epic's title/plan assumed a device grant that doesn't exist server-side. Needs a decision: email+password bearer flow (simplest, pending confirmation the account has a password), a magic-link loopback trick, or adding a device-auth plugin server-side (out of this repo's control). Blocking #14/#15/#17 until decided.
