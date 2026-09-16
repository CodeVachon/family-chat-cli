---
id: 49
title: Define MVP TUI interaction specification
state: Done
parent: 5
assignee: Christopher Vachon
labels: [task, tui, planning]
created: 2026-08-25T21:45:44Z
updated: 2026-09-16T23:42:33Z
---

## Description

Turn the pane concept into an implementable terminal interaction contract.

## Plan

Specify responsive layouts and minimum size behavior, focus order, keymap, channel selection, scrolling, single-line versus multiline compose behavior, status/help presentation, Unicode text wrapping, and the login flow. Mark search and unread indicators as post-MVP.

## Notes

### 2026-09-16T23:42:33Z — Christopher Vachon (user)

Written retroactively (matches this project's build-then-document pattern) as docs/tui-interaction.md: layout, focus order/keymap (login form, channels focus, compose focus, PageUp/PageDown scrolling), channel-selection and message-scrolling behavior, compose behavior, status-line presentation, Unicode/wrap handling, and the already-decided post-MVP exclusions (search, unread indicators). Minimum-size behavior is explicitly documented as NOT enforced (a real gap, cross-referenced from #28's note) rather than glossed over.
