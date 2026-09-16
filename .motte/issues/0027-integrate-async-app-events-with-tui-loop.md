---
id: 27
title: Integrate async app events with TUI loop
state: Done
parent: 5
assignee: Christopher Vachon
labels: [task, tui]
blockedBy: [25]
created: 2026-08-25T21:38:35Z
updated: 2026-09-16T19:57:00Z
---

## Description

Connect background API operations to terminal rendering without blocking input.

## Plan

Use channels or tasks to send app events for loaded data, send completion, refresh results, and auth changes.

## Notes

### 2026-09-16T19:57:00Z — Christopher Vachon (user)

Implemented in src/tui/mod.rs's run_app: tokio::select! merges the crossterm EventStream (key input) with an mpsc channel carrying async Command results (login, channel/message loads, resume). Commands returned by app::state's pure key handlers are executed by spawning a task per command that sends its Event back over the channel; every event (whichever branch fires) triggers exactly one redraw. Verified end-to-end manually against the live server (login submission -> async request -> state update -> redraw, observed in tmux).
