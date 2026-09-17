---
id: 33
title: Add unread or new-message indicators
state: Done
parent: 6
labels: [task, chat, post-mvp]
blockedBy: [30, 51]
created: 2026-08-25T21:38:35Z
updated: 2026-09-17T16:09:16Z
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
