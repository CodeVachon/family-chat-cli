---
id: 42
title: Add unit tests for app state transitions
state: Done
parent: 9
assignee: Christopher Vachon
labels: [task, quality]
blockedBy: [12]
created: 2026-08-25T21:38:35Z
updated: 2026-09-16T23:44:08Z
---

## Description

Test the logic that drives the TUI independently of terminal rendering.

## Plan

Cover channel selection, message loading state, send success/failure, search mode, and auth state transitions.

## Notes

### 2026-09-16T23:44:08Z — Christopher Vachon (user)

Covered by the unit tests accumulated across #12/#25/#31/#51/#57 and this session's scroll fix: channel selection (navigation, auto-load, stale-response rejection), message loading state (loading_messages, seq-guarded on_messages_loaded), send success/failure (draft kept on failure, cleared+reloaded on success, duplicate-submit guard), auth state transitions (login/resume success, NotApproved routing, error display), plus tui::widgets render tests via ratatui's TestBackend for the scrolling/windowing behavior. Not covered: 'search mode' — search is post-MVP (#32), nothing to test yet.
