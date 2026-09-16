---
id: 45
title: Manual TUI verification checklist
state: Todo
parent: 9
labels: [task, quality]
blockedBy: [17, 18, 24, 26, 28, 31, 50, 51]
created: 2026-08-25T21:38:35Z
updated: 2026-09-16T19:57:22Z
---

## Description

Record hands-on terminal checks that automated tests do not cover well.

## Plan

Run and record hands-on checks against a development server for startup, remote-browser device approval, logout and restart, token refresh, channel and user loading, pagination and live updates, compose/send, offline and server errors, narrow and wide layouts, Unicode, resize, suspend/resume where supported, and clean terminal restoration after normal exit, error, and panic. This task completes only when failures are fixed or tracked as blockers.

## Notes

### 2026-09-16T19:57:22Z — Christopher Vachon (user)

Manually verified so far, in a tmux pty against https://chat.thevachonfamily.ca (not the full checklist — this issue stays blocked on #31/#50/#51 etc.): login form renders and accepts typed input correctly (email plain, password masked); submitting invalid credentials round-trips to a real 401 and surfaces the server's own 'Invalid email or password' message; Ctrl-C quits cleanly (exit code 0, terminal returns to normal non-raw mode); resizing the pane mid-session doesn't break rendering. Real successful-login → channel list → message view was NOT verified by me (no account credentials available) — that's the next hands-on check, for Chris to try.
