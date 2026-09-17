---
id: 26
title: Render loading, empty, and error states
state: Done
parent: 5
labels: [task, tui]
blockedBy: [24]
created: 2026-08-25T21:38:35Z
updated: 2026-09-17T15:31:08Z
---

## Description

Make every pane understandable before and after data loads.

## Plan

Add consistent loading indicators, empty-channel/user/message states, and pane-local errors.

## Notes

### 2026-09-16T19:57:20Z — Christopher Vachon (user)

Partial, as a byproduct of the prototype: loading indicator ('Loading messages…'), empty-channel-list state ('No channels yet.'), empty-message-list state ('No messages yet.' — added after noticing a real successfully-loaded-but-empty channel would otherwise render a blank pane), and a status-line error surface all exist in src/tui/widgets/mod.rs. Not covered: errors are shown in one shared bottom status line rather than pane-local to whichever pane failed (channels vs. messages), and there's no empty-user/presence state since #50 (load users/presence) isn't built yet.

### 2026-09-17T15:31:07Z — Christopher Vachon (user)

Completed the remaining item: pane-local errors. Replaced the single shared LoggedInState::status field with three targeted ones — channels_error, messages_error, send_error — each rendered inside the pane whose request actually failed, rather than one shared bottom-line message with no indication of which subsystem broke.

Each error only replaces a pane's content when there's nothing cached to fall back on — same principle as the anti-flash fix (a background reload, an older-page fetch, or a refresh failing must never blank an already-loaded channel list or message history). When valid content is still showing, the pane gets a short, fixed title marker ("— error" / "send failed") instead of the raw message: the sidebar is a fixed 28 columns and the compose box is 1 line tall inside its border, and ratatui titles don't wrap, so an arbitrary-length error there would just get silently clipped mid-word (caught this directly — an earlier version embedded the full error text in these titles and several new tests failed until I checked the rendered output and saw exactly that). The full message is used as pane body content in the "nothing cached" case where wrapping is available, and is otherwise always in the log file already (see tui::log_if_err from #23).

send_error clears on the next compose keystroke (treated as "the user is retrying") or the next successful send.

The one item still not covered — an empty-user/presence state — stays deferred to #50 (load users/presence into the TUI), same as this ticket's prior note said: there's no users/presence pane at all yet for an empty state to belong to. Closing this now rather than leaving it open indefinitely blocked on a completely separate, unstarted ticket; the empty-state treatment naturally lands as part of building that pane.

10 new render tests (ratatui TestBackend) covering all three panes' cached-vs-empty error behavior, plus a test confirming the bottom line always shows the key hints and never an error. 85 tests passing, clippy/fmt clean.
