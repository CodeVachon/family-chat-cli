---
id: 44
title: Add auth flow tests around polling behavior
state: Done
parent: 9
assignee: Christopher Vachon
labels: [task, quality]
blockedBy: [15]
created: 2026-08-25T21:38:35Z
updated: 2026-09-16T23:44:17Z
---

## Description

Verify device authorization edge cases without calling a real auth server.

## Plan

Test pending, success, denied, expired, timeout, and polling interval handling.

## Notes

### 2026-09-16T23:44:17Z — Christopher Vachon (user)

Superseded: this ticket assumed the OAuth device-authorization polling flow from the original epic #3, which doesn't exist server-side (see #3/#55's notes) — there's no 'pending/denied/expired/polling-interval' state machine to test because auth is a single email/password POST, not a poll loop. Equivalent coverage for the actual flow exists: sign_in_parses_the_token_and_user_from_the_response_body and sign_in_surfaces_the_servers_own_error_message (src/api/client.rs), plus not_approved_error_routes_to_its_own_screen (src/app/state.rs) for the one real 'pending' state family-chat has (account approval, not auth polling).
