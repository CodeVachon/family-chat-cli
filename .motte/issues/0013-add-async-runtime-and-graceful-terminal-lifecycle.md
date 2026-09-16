---
id: 13
title: Add async runtime and graceful terminal lifecycle
state: Done
parent: 2
assignee: Christopher Vachon
labels: [task, architecture]
blockedBy: [11]
created: 2026-08-25T21:38:34Z
updated: 2026-09-16T19:56:31Z
---

## Description

Wire Tokio startup and terminal setup/restore primitives.

## Plan

Ensure panic/error paths restore the terminal, expose an async run function, and keep terminal concerns isolated from business logic.

## Notes

### 2026-09-16T19:56:30Z — Christopher Vachon (user)

Implemented in src/tui/mod.rs and main.rs: tokio multi-thread runtime, raw-mode + alternate-screen setup in init_terminal(), teardown in restore_terminal() called on every exit path from run() (success or error) plus from a panic hook installed in main() so a mid-session panic still restores the terminal before the default handler prints. Verified manually in a tmux pty: launched the binary, drove it, quit via Ctrl-C, confirmed exit code 0 and that the shell's tty returned to normal (non-raw) input afterward.
