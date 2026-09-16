---
id: 31
title: Compose and send messages
state: Done
parent: 6
assignee: Christopher Vachon
labels: [task, chat]
blockedBy: [22, 30]
created: 2026-08-25T21:38:35Z
updated: 2026-09-16T23:35:06Z
---

## Description

Allow the user to write and post messages from the TUI.

## Plan

Use the compose behavior and key bindings from task 49. Submit a client request id when the API supports idempotency, keep the draft until success, prevent accidental duplicate submissions, insert or reconcile the server response in stable order, and surface validation, auth, rate-limit, and network failures without losing text. Verify success, retry, duplicate-submit, and failure paths.

## Notes

### 2026-09-16T20:18:34Z — Christopher Vachon (user)

Implemented: TUI composer (Tab toggles focus between the channel list and a compose box — 'q'/'l'/'r' are shortcuts only in channel-list focus, plain characters everywhere in compose focus, so a message can contain those letters). Enter sends when the draft is non-empty and no send is already in flight (state.sending guards against duplicate submits — the plan's 'client request id' idempotency isn't available server-side yet, see #59 in family-chat, so this in-TUI guard is the only protection against a double-Enter). The draft is kept until a send is CONFIRMED successful (not cleared optimistically) so a failure never loses text — verified by unit test. On success, reconciles by reloading the channel's messages (simple and correct without SSE yet — the send response doesn't carry the decorated shape GET returns anyway) rather than splicing a locally-constructed message into the list. Plain text is escaped to HTML via text::html::plain_text_to_html (round-trip tested) so typed markup-looking characters render literally rather than being interpreted. Not yet manually verified against the live server (no test-safe way to send a real message without an account) — that's for Chris to try next.

### 2026-09-16T23:35:06Z — Christopher Vachon (user)

Confirmed live 2026-09-16: Chris verified sending a real message from the TUI works end to end now that family-chat's postMessageSchema.omit() bug (see #48's note) is fixed and redeployed.
