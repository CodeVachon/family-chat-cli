---
id: 58
title: Define rich-text terminal rendering and compose encoding
state: Todo
parent: 5
labels: [task, tui, chat, mvp]
created: 2026-08-26T01:33:09Z
updated: 2026-09-16T20:18:59Z
---

## Description

Define how the native TUI safely displays the API's sanitized HTML message bodies and sends terminal-authored text back to the HTML API.

## Plan

Specify a bounded supported HTML subset and convert it to Ratatui spans for paragraphs, line breaks, emphasis, code, links, mentions, and entities; preserve readable fallback text for unsupported or malformed markup and never interpret terminal escape sequences from content. Define plain-text compose escaping and newline-to-HTML encoding. Verify with fixtures from web-authored messages, Unicode, links, mentions, malformed HTML, and control characters.

## Notes

### 2026-09-16T20:18:59Z — Christopher Vachon (user)

Partial: the compose-encoding half is done — text::html::plain_text_to_html escapes typed text so it round-trips safely as literal text rather than being interpreted as markup (unit tested, including a round-trip test that caught a real bug: to_plain_text wasn't decoding HTML entities back, now fixed in the same change). The rendering half is still just plain-text flattening (to_plain_text collapses each block element to one line, dropping all formatting) rather than styled ratatui spans for bold/links/mentions/code — that upgrade is still open.
