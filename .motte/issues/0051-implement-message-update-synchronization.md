---
id: 51
title: Implement message update synchronization
state: Todo
parent: 6
labels: [task, chat, mvp]
blockedBy: [22, 27, 30]
created: 2026-08-25T21:45:44Z
updated: 2026-09-16T20:45:25Z
---

## Description

Keep the selected channel current without blocking terminal input or corrupting state.

## Plan

Implement the update transport selected in the API contract task, with cancellation on channel changes, bounded retry and backoff, duplicate suppression, deterministic ordering, stale-response rejection, and a manual refresh fallback.

## Notes

### 2026-09-16T20:45:25Z — Christopher Vachon (user)

Partial, alongside #57 (SSE): stale-response rejection is now implemented — every LoadMessages request carries a monotonic per-channel sequence number (LoggedInState::request_messages/message_request_seq), and on_messages_loaded drops any response that isn't the answer to the latest outstanding request for its channel, so an old slow response can no longer clobber a newer one when a manual refresh, a channel switch, and an SSE-triggered reload overlap (unit-tested: a_stale_messages_response_is_dropped_in_favor_of_the_newer_request). Deterministic ordering and duplicate suppression are non-issues here — each load replaces the whole per-channel list rather than appending. Manual refresh fallback exists (r key). NOT implemented: 'cancellation on channel changes' is handled by consequence (the stale response is ignored) rather than literally aborting the in-flight HTTP request — wastes a little bandwidth but doesn't corrupt state. Still missing: bounded automatic retry/backoff on a failed message load — right now a failed load just shows a status error and waits for the user to press r or for the next SSE event to trigger another attempt; there's no self-driven retry loop. Given SSE-driven reloads and the manual fallback, this felt like a reasonable place to stop for now rather than add a retry-with-backoff wrapper speculatively — releasing this back rather than closing it.
