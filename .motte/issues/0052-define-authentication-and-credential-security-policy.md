---
id: 52
title: Define authentication and credential security policy
state: Todo
parent: 3
labels: [task, auth, security]
blockedBy: [14, 34]
created: 2026-08-25T21:45:45Z
updated: 2026-08-25T21:46:11Z
---

## Description

Define the security invariants for a public native CLI before persisting or forwarding credentials.

## Plan

Require HTTPS outside explicit local development; validate issuer, audience/resource, and scopes; never log tokens or device codes at unsafe verbosity; bind credentials to profile and API origin; document keychain-unavailable behavior; and prohibit silently falling back to plaintext token storage.
