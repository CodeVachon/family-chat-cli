---
id: 28
title: Handle terminal resize and clean exit
state: Done
parent: 5
labels: [task, tui]
blockedBy: [24]
created: 2026-08-25T21:38:35Z
updated: 2026-09-17T15:52:03Z
---

## Description

Keep the interface usable across terminal sizes and restore the terminal on exit.

## Plan

React to resize events, constrain minimum usable dimensions, and verify raw mode/alternate screen cleanup.

## Notes

### 2026-09-16T19:57:21Z — Christopher Vachon (user)

Partial: resize is handled for free — ratatui's Terminal::draw re-queries the backend's size every frame, so no explicit resize-event handling was needed; verified by resizing the tmux pane mid-session. Raw-mode/alternate-screen cleanup is implemented (restore_terminal() runs on every exit path from run(), plus a panic hook in main.rs) and verified manually: quit via Ctrl-C, confirmed exit code 0 and the shell's tty was back to normal (non-raw) input afterward. Not covered: constraining/warning on a minimum usable terminal size — untested at extreme small sizes, no explicit floor enforced.

### 2026-09-17T15:52:02Z — Christopher Vachon (user)

Completed the remaining item: a minimum-size floor. First verified there's no actual crash risk at any size — tiny_terminal_sizes_never_panic renders every screen (login form, resuming, logged-in with messages) down to 1x1 through ratatui's real TestBackend and confirms nothing panics; ratatui's Layout constraint solver degrades gracefully on its own.

The real gap was usability, not safety: an ignored probe test (dump_small_sizes) dumped the actual rendered frame at a range of sizes to see what "degrades gracefully" looks like in practice — at 40x10 the fixed 28-column sidebar (see layout::split) leaves so little room for the main pane that message text wraps one word per line; at 30x8 and below the main pane effectively disappears. 50x12 was the smallest size that still looked genuinely usable.

render() now checks frame.area() against that 50x12 floor up front and shows "Terminal too small (WxH) — resize to at least 50x12" instead of attempting the squeezed layout — no border, just centered/wrapped text, since even a 1-cell border might not fit below the floor.

Verified live: resized a real tmux pane down to 30x8 mid-session (while actually logged in against the live server) and got the resize message; resized back to 100x30 and the full UI came back instantly with the exact same channel, scroll position, and messages — confirming resize itself needs no explicit handling (Terminal::draw already re-queries the backend's size every frame, as the prior note said) and that crossing the floor in either direction doesn't lose any state.

4 new tests (floor boundary in both directions + the tiny-size panic probe, plus the ignored eyeball-the-render probe). 88 tests passing, clippy/fmt clean.
