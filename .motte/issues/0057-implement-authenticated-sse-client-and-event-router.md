---
id: 57
title: Implement authenticated SSE client and event router
state: Todo
parent: 4
labels: [task, api, realtime, mvp]
created: 2026-08-26T01:33:08Z
updated: 2026-08-26T01:33:08Z
---

## Description

Maintain the single authenticated /api/v1/stream connection used to drive TUI presence and data reconciliation.

## Plan

Send the bearer session token, parse SSE data and retry fields plus heartbeat comments, handle ready, resync, presence snapshot/change, typing, channel/user/settings changes, message mutations, reactions, mentions, and read updates. Respect 429 connection caps, reconnect with bounded jittered backoff, and perform a full REST resync after reconnect because the server provides no event id or replay cursor. Emit typed application events; SSE notifications trigger selective REST refetches rather than carrying complete objects.
