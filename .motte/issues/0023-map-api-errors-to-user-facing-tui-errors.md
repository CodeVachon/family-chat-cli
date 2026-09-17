---
id: 23
title: Map API errors to user-facing TUI errors
state: Done
parent: 4
labels: [task, api]
blockedBy: [20]
created: 2026-08-25T21:38:34Z
updated: 2026-09-17T17:50:20Z
---

## Description

Convert HTTP/network/auth/rate-limit errors into actionable application errors.

## Plan

Define display-ready error categories for offline, unauthorized, forbidden, validation, server, and unexpected failures.

## Notes

### 2026-09-16T19:57:20Z — Christopher Vachon (user)

Partial, as a byproduct of #20/#15: ApiError has display-ready Server/Unauthorized/NotApproved/Forbidden/Request(network) variants, and login-time errors show the server's own message (see #15's note on the from_auth_response vs from_response split). Not yet covered: a distinct 'validation' (422 + issues) category — nothing in this prototype posts data yet, so no caller produces a 422 to design against. Natural to pick this back up alongside #31 (compose and send messages), which will be the first thing that can 422.

### 2026-09-16T20:33:01Z — Christopher Vachon (user)

Added error logging (src/tui/mod.rs's log_if_err, used by every spawn_* command) so a failed command is diagnosable from the log file afterward — this was the actual gap when #31's send-message bug first showed up as an opaque 'server error' with nothing in the log to explain it. login::resume also now logs why a stored session was dropped. Display-ready error categories (Server/Unauthorized/NotApproved/Forbidden/Request) are otherwise unchanged from the #20 note — still no distinct 'validation' category, still nothing to post 422s against yet.

### 2026-09-17T17:50:19Z — Christopher Vachon (user)

Completed the last remaining item: a distinct 'validation' (422) category, plus a 'rate-limit' (429) one alongside it since the plan's description named both and #31 (send message, since built) is the first real caller that can produce either.

Confirmed the real 422 envelope shape live rather than guessing — POSTed directly against the Testing channel with an invalid/empty body: {"error":{"message":"Validation failed","issues":[{"code":"custom","path":["body"],"message":"Message cannot be empty"}]}}. ApiError gained Validation(String) (combines every issue into "path: message" per issue, joined with "; "; falls back to the plain message when there are no issues, matching a pre-existing test's mock that omits the issues field) and RateLimited(String) (429 — the server already formats a human-readable retry message per docs/api-contract.md, so this just passes it through under its own distinct name rather than the generic Server).

Neither new variant needed any downstream matching: nothing in the app pattern-matches ApiError exhaustively except the one NotApproved check in login handling, and every other call site already just Displays the error via a "Couldn't X: {error}" format string — so the new variants show up with the right text with zero changes needed elsewhere.

4 new/updated tests using the real captured 422 shape and a 429 case, asserting the specific variant (not just the message text, so a future change can't silently collapse Validation/RateLimited back into a generic Server without a test noticing). Verified live that an ordinary send is unaffected (Testing channel, no new errors in the log). 111 tests passing, clippy/fmt clean.

This closes out the ticket's original three notes — display-ready categories for offline/unauthorized/forbidden/server/validation/rate-limit are now all in place.
