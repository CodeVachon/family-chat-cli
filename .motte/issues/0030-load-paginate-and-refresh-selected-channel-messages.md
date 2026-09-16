---
id: 30
title: Load, paginate, and refresh selected channel messages
state: Todo
parent: 6
labels: [task, chat]
blockedBy: [22, 29]
created: 2026-08-25T21:38:35Z
updated: 2026-09-16T19:57:21Z
---

## Description

Show message history for the active channel and refresh it on demand.

## Plan

Load the newest page, fetch older pages in the direction supported by the API, preserve scroll position, and provide manual refresh. Reject stale responses after channel changes and merge pages with stable ordering and duplicate suppression. Leave continuous update transport to task 51. Verify empty, one-page, multipage, refresh, channel-switch race, and API-error cases.

## Notes

### 2026-09-16T19:57:21Z — Christopher Vachon (user)

Partial: initial load of a channel's messages on selection is implemented and covered by a wiremock test (messages_parse_nested_author_and_timestamps) plus the app::state channel-navigation unit test. Manual refresh exists (r key, added while building the prototype — reloads the selected channel unconditionally, bypassing the messages-already-cached check). Not covered: pagination (older-message loading via beforeId/beforeCreatedAt, see docs/api-contract.md) or any automatic refresh/live-update path (that's #51's job, SSE integration).
