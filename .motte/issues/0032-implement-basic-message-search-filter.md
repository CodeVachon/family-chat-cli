---
id: 32
title: Implement basic message search/filter
state: Done
parent: 6
labels: [task, chat, post-mvp]
blockedBy: [30]
created: 2026-08-25T21:38:35Z
updated: 2026-09-17T18:01:21Z
---

## Description

Provide fast local filtering over loaded messages.

## Plan

Add slash-triggered search input and highlight or filter matching loaded messages.

## Notes

### 2026-09-17T18:01:20Z — Christopher Vachon (user)

Completed. Press / from Channels focus to enter a new Search focus mode; typing filters the selected channel's cached messages live via a case-insensitive substring match against the author's display name and the message's plain-text body (a new text::html::to_plain_text, built on the existing to_lines HTML parse so search matching stays consistent with what's actually rendered — no separate/divergent stripping logic). Enter keeps the filter and stops editing it; Esc clears it too, matching a pager's /search convention. The filter persists across focus changes and channel switches until explicitly cleared.

No dedicated search box pane: the status line becomes the input itself while actively typing ("/query"), since there's nowhere else in this layout with spare room and the usual hints aren't useful mid-search. The messages pane's title gets a "— /query" suffix whenever a filter is active but not being edited.

LoggedInState::visible_messages() is the one place both filtered and unfiltered cases meet (empty query returns everything, otherwise the matched subset), so windowed_messages/messages_to_lines now operate on &[&Message] uniformly. Distinguishes "no messages at all" from "the filter matched nothing" with different pane text.

Verified live against the real account: searching "star" in "Christopher & Louise" correctly matched a literal "Star Trek" message and "started" (a genuine, expected substring match — not a bug, since this is plain substring search, not word-boundary search). Enter kept the filter active through a re-render; Esc restored full history.

13 new tests (state-level: focus entry/exit, filtering by body and author name, Enter-vs-Esc; render-level: filtered content, no-match message, the status-line input box; plus 2 for to_plain_text). 121 tests passing, clippy/fmt clean.
