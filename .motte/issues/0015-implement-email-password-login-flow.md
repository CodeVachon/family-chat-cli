---
id: 15
title: Implement email/password login flow
state: Done
parent: 3
assignee: Christopher Vachon
labels: [task, auth]
blockedBy: [11, 14]
created: 2026-08-25T21:38:34Z
updated: 2026-09-16T19:56:47Z
---

## Description

Authenticate the CLI with the account's email and password, no browser/local web app involved.

## Plan

Prompt for email and (masked) password in the terminal. POST /api/auth/sign-in/email; on success, read the set-auth-token response header and hand the token to the credential store (#16). Handle: invalid credentials (401 — show a plain retry prompt, don't leak whether the email exists), an authenticated-but-unapproved account (403 from a subsequent /api/v1 call — show a distinct 'waiting on admin approval' state, not a login error), network/timeout errors, and malformed responses. Verify all these paths against a fake/mock server (401, 403-after-200, network error, malformed JSON) plus one live run once #14 confirms the real behavior.

## Notes

### 2026-09-16T19:56:47Z — Christopher Vachon (user)

Implemented: src/auth/login.rs (sign_in orchestrates ApiClient::sign_in_email + stores the token via CredentialStore), TUI login form in src/tui/widgets/mod.rs + key handling in src/app/state.rs (Tab between email/password, masked password, Enter submits, empty-field validation before submitting). Verified live and manually in a tmux pty against https://chat.thevachonfamily.ca: typing/masking render correctly; submitting with a made-up email correctly round-trips to a real 401 and displays the server's own message ('Invalid email or password') rather than a generic one — this caught a real bug (see #20/#23 note) where 401s were collapsed to a generic message regardless of cause, now fixed by separating from_auth_response (login: show the real reason) from from_response (an existing session dying mid-use: generic, since there's no specific reason to show). Did not attempt a real successful login (no account credentials available to me) — that's Chris's to try.
