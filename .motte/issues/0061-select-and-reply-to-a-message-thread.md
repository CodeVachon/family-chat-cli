---
id: 61
title: Select and reply to a message thread
state: Done
parent: 6
labels: [task, chat]
created: 2026-09-18T14:09:56Z
updated: 2026-09-18T14:34:50Z
---

## Description

Threads are currently read-only (#60 added viewing). There is no way to select a message and reply into its thread, or start a new thread by replying to a message that doesn't have one yet.

## Plan

From Compose focus, with an empty draft, Right arrow enters thread-reply mode targeting the channel's last message; Up/Down change the targeted message; the messages pane shows just that message's thread; Esc exits. Sending while targeting a thread attaches threadRootId to the outgoing message (confirmed live that the server's POST /messages accepts this field and creates a real reply).

## Notes

### 2026-09-18T14:19:49Z — Christopher Vachon (user)

Implemented exactly to spec: Right on an empty compose draft targets the last visible message; Up/Down move the target (clamped, not wrapping); Esc exits. The messages pane fully swaps to a 'Replying to: <root> / Replies: <cached replies>' view while targeting is active (title becomes '<channel> — Thread', compose title becomes 'Reply', hint line becomes 'Replying — ↑/↓ change message · Enter send · Esc cancel'). Switching channels while targeting clears the target. Sending attaches threadRootId — client.rs's send_message now takes Option<&str> and conditionally sets the JSON field.

Verified live end-to-end against the real Testing channel through the actual TUI (tmux): entered thread mode on the newest message, moved selection up with arrows, typed and sent a reply, confirmed via curl (GET .../thread) that the reply landed with threadRootId correctly pointing at the targeted root, then confirmed Esc returned the UI to the normal Message/hint state.

137 tests passing (added ~10 for state transitions + 2 render tests), clippy clean, fmt clean.

### 2026-09-18T14:34:50Z — Christopher Vachon (user)

Follow-up fix: a sent reply never showed up in the CLI (thread view or inline preview) until something else evicted the thread_replies cache, even though the reply count updated and the web app showed it instantly. Root cause: on_message_sent reloaded the plain message list (refreshing reply_count) but never invalidated the already-cached thread for that root. Fixed with a threads_pending_refresh flag set on a successful reply and consumed by mark_threads_loading, forcing threads_needing_fetch to re-fetch an already-cached thread. Verified live in Testing: a second reply to an already-cached thread now appears immediately in both views. See commit 27879db.
