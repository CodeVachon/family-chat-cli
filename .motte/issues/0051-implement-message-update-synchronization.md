---
id: 51
title: Implement message update synchronization
state: Todo
parent: 6
labels: [task, chat, mvp]
blockedBy: [22, 27, 30]
created: 2026-08-25T21:45:44Z
updated: 2026-08-25T21:46:09Z
---

## Description

Keep the selected channel current without blocking terminal input or corrupting state.

## Plan

Implement the update transport selected in the API contract task, with cancellation on channel changes, bounded retry and backoff, duplicate suppression, deterministic ordering, stale-response rejection, and a manual refresh fallback.
