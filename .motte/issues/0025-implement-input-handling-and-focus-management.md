---
id: 25
title: Implement input handling and focus management
state: Todo
parent: 5
labels: [task, tui]
blockedBy: [24]
created: 2026-08-25T21:38:35Z
updated: 2026-09-16T19:57:20Z
---

## Description

Support keyboard navigation across panes and compose mode.

## Plan

Handle tab/shift-tab or equivalent focus cycling, arrows/jk movement, enter, escape, quit, refresh, and compose shortcuts.

## Notes

### 2026-09-16T19:57:20Z — Christopher Vachon (user)

Partial, as a byproduct of building the login form and channel list for the prototype: Tab focus-cycling (login form), arrow/j-k navigation (channel list), Enter (submit login), q/Ctrl-C (quit), and (added while testing) r (refresh current channel's messages) are all implemented and manually verified in a tmux pty. Not covered: Escape (nothing yet needs dismissing) and compose-mode shortcuts — there's no message composer yet, that's #31's job to build first.
