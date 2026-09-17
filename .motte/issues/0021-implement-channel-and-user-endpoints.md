---
id: 21
title: Implement channel and user endpoints
state: Done
parent: 4
labels: [task, api]
blockedBy: [20]
created: 2026-08-25T21:38:34Z
updated: 2026-09-17T14:44:54Z
---

## Description

Expose typed methods for listing channels and users/presence data.

## Plan

Add list_channels, get_channel if needed, list_users, and current_user methods.

## Notes

### 2026-09-16T20:18:59Z — Christopher Vachon (user)

Partial: list_channels and current-user (me, which also returns preferences/unread) are implemented in src/api/client.rs and used by the TUI. list_users isn't implemented — that's #50's job (load users and presence into the TUI), which hasn't been picked up yet.

### 2026-09-17T14:44:53Z — Christopher Vachon (user)

Completed: added ApiClient::channel_members(channel_id) -> ChannelMembersResponse (src/api/client.rs), the remaining item from this ticket's plan ("list_users").

Confirmed live against the real server that there's no flat, instance-wide "list all users" endpoint — only per-channel membership (GET /channels/:id/members, which this uses) and a single-user profile lookup (GET /users/:userId/profile, which returns far more than needed — email, bio, phone, a full file gallery — for what this ticket needs). channel_members is the closest real match to "list_users," scoped per-channel. ChannelMember models the actual response shape (userId/role/name/colorHue/avatarUrl) verified against a live query against "The Vachons".

get_channel deliberately left out: list_channels already returns full channel objects and nothing in the app needs a single-channel-detail fetch distinct from that.

Not wired into the TUI — that's #50 (load users/presence into the TUI), which owns actually building a users/presence pane on top of this. Method is #[allow(dead_code)]'d until then, same pattern already used for types::Message's not-yet-consumed fields.

1 new wiremock test (channel_members_parses_the_real_server_shape) using the real server's field names/types. 62 tests passing, clippy/fmt clean.
