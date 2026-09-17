---
id: 58
title: Define rich-text terminal rendering and compose encoding
state: Done
parent: 5
assignee: Christopher Vachon
labels: [task, tui, chat, mvp]
created: 2026-08-26T01:33:09Z
updated: 2026-09-17T00:10:14Z
---

## Description

Define how the native TUI safely displays the API's sanitized HTML message bodies and sends terminal-authored text back to the HTML API.

## Plan

Specify a bounded supported HTML subset and convert it to Ratatui spans for paragraphs, line breaks, emphasis, code, links, mentions, and entities; preserve readable fallback text for unsupported or malformed markup and never interpret terminal escape sequences from content. Define plain-text compose escaping and newline-to-HTML encoding. Verify with fixtures from web-authored messages, Unicode, links, mentions, malformed HTML, and control characters.

## Notes

### 2026-09-16T20:18:59Z — Christopher Vachon (user)

Partial: the compose-encoding half is done — text::html::plain_text_to_html escapes typed text so it round-trips safely as literal text rather than being interpreted as markup (unit tested, including a round-trip test that caught a real bug: to_plain_text wasn't decoding HTML entities back, now fixed in the same change). The rendering half is still just plain-text flattening (to_plain_text collapses each block element to one line, dropping all formatting) rather than styled ratatui spans for bold/links/mentions/code — that upgrade is still open.

### 2026-09-17T00:10:13Z — Christopher Vachon (user)

Rendering half done, completing this ticket. text::html::to_lines replaces the old plain-text flattener with a real HTML->ratatui::text::Line tree walker, built against the confirmed server allowlist (DOMPurify in apps/web/lib/messaging/rich-text.ts: p br strong em s u code pre blockquote ul ol li a span, href restricted to http(s)/mailto) rather than guessing: bold/italic/underline/strikethrough modifiers, code/link/mention/blockquote colors, br as an in-paragraph line break (a real gap in the old renderer — web-authored Shift+Enter soft breaks used to run together with no separator), bulleted ul and numbered ol, blockquote with a '> ' marker per nesting level, and mentions (Tiptap's default extension-mention span, data-type=mention) styled distinctly using their existing '@Name' text content. Unrecognized/malformed tags fall back to plain inline passthrough rather than dropping content.

Security: control characters (ESC, BEL, etc.) are now stripped from every text run before it becomes a Span (text::html::sanitize_for_terminal) — the sanitizer only strips dangerous tags/attributes, not literal control bytes sitting in a text node, so without this a message could inject real terminal escape sequences (cursor moves, OSC title-bar/clipboard tricks). This is the concrete implementation of the plan's 'never interpret terminal escape sequences from content,' verified with a dedicated test embedding a literal ESC+ANSI-color sequence and confirming it's neutralized.

Per-author message color (stable hash of author.id, small readable palette) makes a busy multi-person channel easier to scan.

Images/videos: no inline rendering (sixel/kitty graphics support is inconsistent across terminals, and video can never render inline regardless) — each attachment gets its own line: [image|video|pdf|file], filename, dimensions when known, and the raw secure_url as plain visible text (not an OSC-8 hyperlink — several modern terminals auto-linkify bare URLs on their own, and embedding raw escape sequences in a ratatui cell isn't safe per the point above). Modeled the previously-unmodeled Attachment/attachments fields on Message to support this.

Verified: 12 new text::html unit tests (bold/br/links/mentions/lists/blockquote/control-characters/malformed-input), 2 tui::widgets tests already covering scroll now exercise the styled-Line path too, plus a new #[ignore]'d visual_preview_of_rich_rendering test (run via 'cargo test visual_preview -- --ignored --nocapture') that renders a synthetic message set covering every formatting kind to the real terminal for eyeballing — used it live in a tmux pty and visually confirmed bold/italic/link/code/mention/list/blockquote/attachment rendering and per-author colors all look right. 44 tests passing, clippy/fmt clean.
