---
id: 17
title: Handle session expiry and logout
state: Done
parent: 3
assignee: Christopher Vachon
labels: [task, auth]
blockedBy: [16]
created: 2026-08-25T21:38:34Z
updated: 2026-09-16T19:56:49Z
---

## Description

Keep API access working across runs and let the user clear stored credentials.

## Plan

There is no refresh-token grant to implement (see #3). On any /api/v1 401, drop the cached session, clear the keychain entry, and drop the TUI into the logged-out state (#18) prompting re-login — do not silently retry with the same dead token. Logout command/action calls POST /api/auth/sign-out (best-effort — proceed with local cleanup even if that call fails, e.g. offline) and deletes the keychain entry.

## Notes

### 2026-09-16T19:56:49Z — Christopher Vachon (user)

Implemented in src/auth/login.rs (resume() drops the cached token + clears the keychain on any failure, including a dead/expired session; sign_out() calls POST /api/auth/sign-out best-effort then always clears locally) and src/tui/mod.rs (Command::Logout wired to the 'l' key, immediately drops to the LoggedOut screen without waiting on the network call). No refresh-token grant exists to implement, per #3 — matches this issue's plan exactly.
