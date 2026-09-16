---
id: 57
title: Implement authenticated SSE client and event router
state: Done
parent: 4
assignee: Christopher Vachon
labels: [task, api, realtime, mvp]
created: 2026-08-26T01:33:08Z
updated: 2026-09-16T20:40:55Z
---

## Description

Maintain the single authenticated /api/v1/stream connection used to drive TUI presence and data reconciliation.

## Plan

Send the bearer session token, parse SSE data and retry fields plus heartbeat comments, handle ready, resync, presence snapshot/change, typing, channel/user/settings changes, message mutations, reactions, mentions, and read updates. Respect 429 connection caps, reconnect with bounded jittered backoff, and perform a full REST resync after reconnect because the server provides no event id or replay cursor. Emit typed application events; SSE notifications trigger selective REST refetches rather than carrying complete objects.

## Notes

### 2026-09-16T20:40:55Z — Christopher Vachon (user)

Implemented: api::stream::RealtimeStream wraps reqwest-eventsource's EventSource against GET /api/v1/stream with the bearer token attached (ApiClient::stream_request). Server frames are all default-'message' SSE events carrying a JSON {type,...} body (confirmed from family-chat's encodeSSE) — parsed into a typed RealtimeEvent enum (api::types) with an #[serde(other)] catch-all for kinds this prototype doesn't act on yet (typing/presence/reactions/mentions/read-receipts/users.changed/settings.changed — no UI for any of those). Acted-on kinds: message.created/updated/deleted (reload messages if it's the currently-selected channel), channels.changed and resync (reload the channel list). Reconnect/backoff is the crate's own default (300ms exponential up to 5s, unlimited retries) — verified by reading its source rather than assumed; it does NOT retry a bad initial response (e.g. a dead token) at all, which is the right behavior (that needs a fresh login, not a retry loop), so RealtimeStream::next() returns None permanently in that case and the caller doesn't try to restart it. The stream starts once per login session (piggybacked on the first Command::LoadChannels from login/resume, guarded so later LoadChannels calls triggered by realtime events themselves don't restart it) and is aborted on logout. Caught and fixed a real latent bug in the same change: on_channels_loaded always reloaded the FIRST channel's messages regardless of what was selected — harmless before (only ever called once, right after login), but wrong now that channels.changed/resync reload live while the user might be viewing a different channel. Fixed to reload the selected channel (clamped to the new list) instead, with a unit test. Verified: 4 new app::state unit tests (message events routed correctly, resync/channels.changed reload, selection preserved across a reload) plus a live manual smoke test in a tmux pty — a previously-saved session resumed automatically, channels and real message history rendered correctly, composer box appeared, clean Ctrl-C exit. Did not verify a live push actually arriving while the TUI is open (would need a second client posting while this one watches) — worth trying now that send-message is fixed server-side too.
