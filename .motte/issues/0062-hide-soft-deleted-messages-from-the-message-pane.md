---
id: 62
title: Hide soft-deleted messages from the message pane
state: Done
parent: 6
labels: [bug, chat]
created: 2026-09-18T14:45:54Z
updated: 2026-09-18T14:46:11Z
---

## Description

The server keeps a soft-deleted message (deletedAt set) in the same paginated GET /channels/:id/messages response as everything else, with its original body intact. The Message struct already parses deleted_at but nothing filtered on it, so a deleted message stayed fully visible (confirmed live: an accidentally-sent test message deleted from the real 'Christopher & Louise' channel still showed up).

## Plan

Filter deleted_at.is_some() out of LoggedInState::visible_messages (the single point everything else reads a channel's messages through) and out of on_thread_loaded's cached replies, and skip deleted roots in threads_needing_fetch. Adjusted the empty-state message text so an all-deleted channel reads as 'No messages yet.' rather than a search-style 'No messages match'.

## Notes

### 2026-09-18T14:46:10Z — Christopher Vachon (user)

Fixed by filtering deleted_at.is_some() out of visible_messages (state.rs), on_thread_loaded's cached replies, and threads_needing_fetch's candidate roots. Added an all-deleted-channel wording fix in widgets/mod.rs so it reads 'No messages yet.' instead of a search-style no-match message. 5 new tests (3 state, 2 render). Verified live against the real 'Christopher & Louise' channel, which had exactly one soft-deleted message (chronologically the last one) — it no longer renders. See commit 9c35151.
