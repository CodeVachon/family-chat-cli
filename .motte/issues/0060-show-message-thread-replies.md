---
id: 60
title: Show message thread replies
state: Done
parent: 6
labels: [task, chat, bug]
created: 2026-09-18T13:54:59Z
updated: 2026-09-18T13:56:34Z
---

## Description

The server never includes thread replies in the main paginated channel history (GET /channels/:id/messages) — only a replyCount on the root message. Replies only exist behind GET /channels/:id/messages/:messageId/thread, which the CLI never called, so entire side-conversations were completely invisible with no indication they existed.

## Plan

Model threadRootId/replyCount on Message and the thread endpoint's response shape. Fetch a root's thread automatically once its reply count is known (right after any successful messages load), cache replies per root, and render them indented under the root with a reply-count marker. Verified live against a real 10th-anniversary thread on The Vachons that was completely hidden before this fix.

## Notes

### 2026-09-18T13:56:34Z — Christopher Vachon (user)

Completed. Confirmed live (curl against the real server) that GET /channels/:id/messages never includes a thread reply inline — only a replyCount on the root message. Replies exist ONLY behind GET /channels/:id/messages/:messageId/thread, which the CLI never called at all, so entire side-conversations were completely invisible with zero indication they existed. Confirmed via the actual response shape: the thread endpoint returns the root first (threadRootId: null), then its replies (threadRootId set to the root's id), oldest first.

Added Message::thread_root_id/reply_count and ApiClient::thread(). AppState::threads_needing_fetch finds roots (in a given channel's cache) with replies not yet cached or already loading; tui::run calls it right after every successful messages load (initial or older-page) and spawns a fetch per root needing one — same no-dedicated-Command pattern already used for mark_channel_read/load_members (a side effect riding along with an event rather than needing its own Command variant). A failed thread fetch just leaves that one thread's replies unavailable (still logged) rather than a pane-wide error — the root's own reply count keeps showing regardless.

Rendering: a root with replies gets a "💬 N replies" marker right after its own lines — shown even before the thread is actually fetched, so the existence of a thread is never hidden even momentarily while the fetch is in flight. Once cached, each reply renders indented underneath with a "↳" marker.

Verified live against real data on "The Vachons": two threads that were completely hidden before this fix now show immediately (reply counts) and, once fetched, the actual previously-invisible conversation — including a genuine 10th-anniversary well-wish and thank-you exchange between real family members that this bug had been hiding.

Also fixed two pre-existing "#60" doc-comment references in api/types.rs that predated this ticket's existence and had drifted to the wrong number — corrected to #58, where that work (system-event rendering) actually landed.

12 new tests: fetch-need detection, dedup via the loading guard, caching replies while excluding the root, failure handling (state-level), plus render tests for the reply marker and indented reply text (widget-level). 128 tests passing, clippy/fmt clean.
