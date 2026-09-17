---
id: 33
title: Add unread or new-message indicators
state: Done
parent: 6
labels: [task, chat, post-mvp]
blockedBy: [30, 51]
created: 2026-08-25T21:38:35Z
updated: 2026-09-17T16:13:52Z
---

## Description

Make channel activity visible while the user is focused elsewhere.

## Plan

Track latest seen message per channel locally if the API does not expose unread state; prefer server unread state if available.

## Notes

### 2026-09-17T16:09:15Z — Christopher Vachon (user)

Completed. The plan said "prefer server unread state if available" — it was available (Channel.unreadCount, GET /me's unread) and already displayed in the channel list as a byproduct of earlier work, but nothing ever told the server the user had caught up, so the badge could never clear from inside this client. That was the actual gap.

Added ApiClient::mark_channel_read (POST /channels/:id/read — confirmed live it responds 204 with no body). app::state::request_messages — the single chokepoint every "now viewing this channel" trigger passes through (a switch, a manual refresh, or an SSE reload for the already-selected channel) — optimistically zeroes that channel's local unread_count immediately rather than waiting on a round trip. tui::run's Command::LoadMessages handling also fires the actual mark_channel_read call in the background (fire-and-forget, logged on failure like everything else). Added RealtimeEvent::ReadUpdated (read.updated) folded into the same full-channels-reload bucket as Resync/ChannelsChanged, so a channel marked read from another client (e.g. the web app) updates this client's counts too.

Verified live: navigated channels against the real account and confirmed zero mark-channel-read failures in the log. More notably, this also confirmed two previously-unverified assumptions against the real server at once — the /read endpoint's actual response shape (204, no body), and that the server genuinely emits a `read.updated` SSE event with that exact type string (the docs listed it but I hadn't seen one fire before); it deserialized correctly into the new variant instead of falling into Other.

4 new tests: unread cleared on both initial-select and channel-switch, ReadUpdated joining the reload-channels bucket, the 204-no-body response, and read.updated deserializing. 92 tests passing, clippy/fmt clean.

### 2026-09-17T16:13:52Z — Christopher Vachon (user)

Regression found immediately after landing: Chris reported "changing channels now gets stuck on loading messages." Root cause was an infinite feedback loop — this client's own mark_channel_read call makes the server echo a real read.updated event back at the same client; the ReadUpdated handling added here reloaded channels on that event, which re-requests the selected channel's messages (request_messages always does, per its own design), which fires another mark_channel_read, which triggers another read.updated. Confirmed live: the log showed hundreds of these cycles per few seconds, and a still-running process (from before the fix) was caught actively stuck in it, hammering the production server.

Fixed by no longer reacting to ReadUpdated at all — it's ignored like Ready/Other now. The actual, intended behavior of #33 (your own unread badge clearing when you view a channel, both locally and on the server) is untouched and still verified working. What's now explicitly NOT implemented: picking up another client's (e.g. the web app's) read state live — doing that safely needs a way to tell "this client caused this echo" from "another client did," which the event's payload doesn't give us enough to distinguish yet. Noting this as a known gap rather than a silent scope cut.

34 lines changed, 1 test updated + 1 new regression test (read_updated_is_ignored_to_avoid_an_infinite_mark_read_loop) explaining why. 93 tests passing, clippy/fmt clean.
