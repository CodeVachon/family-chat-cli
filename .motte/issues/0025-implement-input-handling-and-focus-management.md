---
id: 25
title: Implement input handling and focus management
state: Done
parent: 5
assignee: Christopher Vachon
labels: [task, tui]
blockedBy: [24]
created: 2026-08-25T21:38:35Z
updated: 2026-09-16T20:18:49Z
---

## Description

Support keyboard navigation across panes and compose mode.

## Plan

Handle tab/shift-tab or equivalent focus cycling, arrows/jk movement, enter, escape, quit, refresh, and compose shortcuts.

## Notes

### 2026-09-16T19:57:20Z — Christopher Vachon (user)

Partial, as a byproduct of building the login form and channel list for the prototype: Tab focus-cycling (login form), arrow/j-k navigation (channel list), Enter (submit login), q/Ctrl-C (quit), and (added while testing) r (refresh current channel's messages) are all implemented and manually verified in a tmux pty. Not covered: Escape (nothing yet needs dismissing) and compose-mode shortcuts — there's no message composer yet, that's #31's job to build first.

### 2026-09-16T20:18:48Z — Christopher Vachon (user)

Completed by #31: Tab now cycles focus between the channel list and the compose box within the logged-in screen too (not just the login form), and compose-mode key handling (typing, Backspace, Enter-to-send) is implemented and unit-tested. Remaining gap from the original plan — Escape — still has no assigned behavior, since nothing in the UI currently needs dismissing (no modals/overlays exist yet); revisit if one is added.
