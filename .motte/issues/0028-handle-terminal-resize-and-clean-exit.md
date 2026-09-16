---
id: 28
title: Handle terminal resize and clean exit
state: Todo
parent: 5
labels: [task, tui]
blockedBy: [24]
created: 2026-08-25T21:38:35Z
updated: 2026-09-16T19:57:21Z
---

## Description

Keep the interface usable across terminal sizes and restore the terminal on exit.

## Plan

React to resize events, constrain minimum usable dimensions, and verify raw mode/alternate screen cleanup.

## Notes

### 2026-09-16T19:57:21Z — Christopher Vachon (user)

Partial: resize is handled for free — ratatui's Terminal::draw re-queries the backend's size every frame, so no explicit resize-event handling was needed; verified by resizing the tmux pane mid-session. Raw-mode/alternate-screen cleanup is implemented (restore_terminal() runs on every exit path from run(), plus a panic hook in main.rs) and verified manually: quit via Ctrl-C, confirmed exit code 0 and the shell's tty was back to normal (non-raw) input afterward. Not covered: constraining/warning on a minimum usable terminal size — untested at extreme small sizes, no explicit floor enforced.
