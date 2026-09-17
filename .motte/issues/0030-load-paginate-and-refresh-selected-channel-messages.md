---
id: 30
title: Load, paginate, and refresh selected channel messages
state: Done
parent: 6
assignee: Christopher Vachon
labels: [task, chat]
blockedBy: [22, 29]
created: 2026-08-25T21:38:35Z
updated: 2026-09-17T14:30:23Z
---

## Description

Show message history for the active channel and refresh it on demand.

## Plan

Load the newest page, fetch older pages in the direction supported by the API, preserve scroll position, and provide manual refresh. Reject stale responses after channel changes and merge pages with stable ordering and duplicate suppression. Leave continuous update transport to task 51. Verify empty, one-page, multipage, refresh, channel-switch race, and API-error cases.

## Notes

### 2026-09-16T19:57:21Z — Christopher Vachon (user)

Partial: initial load of a channel's messages on selection is implemented and covered by a wiremock test (messages_parse_nested_author_and_timestamps) plus the app::state channel-navigation unit test. Manual refresh exists (r key, added while building the prototype — reloads the selected channel unconditionally, bypassing the messages-already-cached check). Not covered: pagination (older-message loading via beforeId/beforeCreatedAt, see docs/api-contract.md) or any automatic refresh/live-update path (that's #51's job, SSE integration).

### 2026-09-17T12:53:07Z — Christopher Vachon (user)

Completed, prompted by Chris noticing a channel was missing its last couple of days of messages. Root cause turned out NOT to be missing pagination — it was that switching to an already-cached channel skipped refreshing it entirely (see #51's note), so messages posted while that channel wasn't the active one never appeared until something else happened to trigger a reload. Fixed load_selected to always request the latest page on channel switch now.

Pagination itself (the part actually asked for) is also now implemented: MessagesResponse carries hasMore; ApiClient::channel_messages takes an optional (before_id, before_created_at) cursor; PageUp, once state.message_scroll reaches/exceeds the cached message count for the selected channel (a message-count proxy for 'nearing the top' since app::state doesn't do HTML rendering and can't know the exact wrapped line count — an early-prefetch heuristic, not a precision requirement), requests the page before the oldest cached message and prepends the result. Guarded against duplicate concurrent requests per channel (loading_older set) and against fetching past the server-confirmed end (has_more map). Scroll position is preserved (approximately — same 1-line-per-message assumption as the trigger) across the prepend so the view doesn't jump.

Also handled a documented edge case: the server's cursor is exclusive on a millisecond boundary and can hand back a row already cached (see docs/api-contract.md's own note on this) — the merge now dedupes by message id before combining pages, with a test reproducing exactly that overlap.

Verified: 6 new app::state unit tests (trigger threshold, duplicate-request guard, has_more gating, prepend ordering + scroll preservation, dedup on overlap) plus the existing wiremock cursor test (#43's note). Not covered: automatic retry/backoff on a failed page load (same gap #51 already noted, still open — a failed load still just shows a status error).

### 2026-09-17T14:30:23Z — Christopher Vachon (user)

Reopened investigation after Chris reported the identical symptom again post-fix (last message from Sep 13, server had through Sep 16) — the load_selected fix from the prior note didn't resolve it, because the actual bug was elsewhere.

Direct log inspection plus a live curl comparison against the server (via the stored session token) proved the fetch itself was always correct and current for every channel — this was never a data-staleness or pagination bug at all, in either report. The real cause: tui::widgets::windowed() decided how many message-history lines fit the pane by counting each logical Line as exactly one rendered row, ignoring that a long attachment URL wraps across multiple terminal rows. That undercounted the space wrapped lines actually need, so Paragraph silently clipped the genuinely newest messages off the bottom of the pane — with zero error or visual indication. It looked exactly like stale/paginated data, twice.

Fixed by having windowed() count each line's actual wrapped row count (Line::width().div_ceil(pane_width)) when reserving space for the visible window, instead of one row per line. Added a regression test (a_wide_line_reserves_its_actual_wrapped_row_count) and INFO-level logging of load results/realtime events (src/tui/mod.rs) that was essential to isolate this from the data layer. Verified live against "The Vachons" (the channel with 3 image attachments and long Cloudinary URLs that originally exhibited the clipping) — now shows through the server's actual newest message.
