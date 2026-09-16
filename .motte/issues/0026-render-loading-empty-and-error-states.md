---
id: 26
title: Render loading, empty, and error states
state: Todo
parent: 5
labels: [task, tui]
blockedBy: [24]
created: 2026-08-25T21:38:35Z
updated: 2026-09-16T19:57:20Z
---

## Description

Make every pane understandable before and after data loads.

## Plan

Add consistent loading indicators, empty-channel/user/message states, and pane-local errors.

## Notes

### 2026-09-16T19:57:20Z — Christopher Vachon (user)

Partial, as a byproduct of the prototype: loading indicator ('Loading messages…'), empty-channel-list state ('No channels yet.'), empty-message-list state ('No messages yet.' — added after noticing a real successfully-loaded-but-empty channel would otherwise render a blank pane), and a status-line error surface all exist in src/tui/widgets/mod.rs. Not covered: errors are shown in one shared bottom status line rather than pane-local to whichever pane failed (channels vs. messages), and there's no empty-user/presence state since #50 (load users/presence) isn't built yet.
