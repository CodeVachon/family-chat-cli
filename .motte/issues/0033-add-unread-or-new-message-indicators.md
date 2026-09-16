---
id: 33
title: Add unread or new-message indicators
state: Todo
parent: 6
labels: [task, chat, post-mvp]
blockedBy: [30, 51]
created: 2026-08-25T21:38:35Z
updated: 2026-08-25T21:46:09Z
---

## Description

Make channel activity visible while the user is focused elsewhere.

## Plan

Track latest seen message per channel locally if the API does not expose unread state; prefer server unread state if available.
