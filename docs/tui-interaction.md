# TUI interaction contract (as built, 2026-09-16)

Written retroactively against the actual implementation (`src/tui/`,
`src/app/state.rs`) rather than spec-first — see AGENTS.md's iterate-then-
document pattern already used for `docs/architecture.md` and
`docs/api-contract.md`. Covers Motte #49.

## Layout

Full-screen alternate-screen TUI (`src/tui/mod.rs`). Logged-in screen:
sidebar (channel list, fixed 28 columns) + main area (message pane, fills
remaining width) on top, a 3-row compose box under the message pane, and a
1-row status/help line at the bottom (`src/tui/layout.rs`,
`tui/widgets::logged_in_view`). Login, "connecting", "signing in", and
"not approved" screens are a single centered box (`centered_rect`).

**Minimum size**: not enforced. Ratatui clips gracefully at small sizes
(borders/text truncate rather than panicking), but there's no explicit floor
or a "terminal too small" message. Untested below roughly 40×10.

**Resize**: handled for free — `Terminal::draw` re-queries the backend size
every frame, so no explicit resize-event code exists.

## Focus order and keymap

Two focus targets in the logged-in screen (`LoggedInFocus`): **Channels**
(the sidebar) and **Compose** (the message box). `Tab` toggles between them;
there is no `Shift-Tab` (only two targets, so one direction is enough).

Global (any screen): `Ctrl-C` quits immediately.

**Login screen**: `Tab`/`↑`/`↓` cycle Email ↔ Password, typing fills the
focused field (`Backspace` deletes), `Enter` submits (validates both fields
non-empty first — a client-side check, not a round trip).

**Not-approved screen**: `q`/`Esc` quits.

**Channels focus**: `↑`/`k` and `↓`/`j` move the selection (loading that
channel's messages if not already cached); `r` force-refreshes the current
channel; `l` logs out; `q` quits.

**Compose focus**: any character key types into the draft, `Backspace`
deletes, `Enter` sends (no-op on an empty draft or while a send is already
in flight — see docs/architecture.md's #31 note on why there's no
idempotency key to lean on instead).

**Either focus**: `PageUp`/`PageDown` scroll the message pane by a fixed
10-line step (`AppState::SCROLL_STEP`) — not text input, so it works
regardless of which pane has focus.

## Channel selection and message scrolling

Selecting a channel loads its messages if not cached (or always, for a
forced refresh/live update — see docs/architecture.md's #51 note on the
per-channel request sequence number that keeps overlapping reloads from
racing). The message pane is **bottom-anchored**: it shows the newest
content by default and `PageUp` scrolls up into history; selecting a
different channel, or any fresh load landing for the current one, resets
the scroll back to the bottom. This is a real fix, not a hypothetical: the
pane used to render from the top of the raw text, silently clipping the
newest messages off the bottom in any channel with more history than fits.

The window is computed over **raw lines**, not post-wrap rendered rows, so
a single very long line can still push the count off by a row or two — an
accepted approximation given how short most chat messages are.

## Compose behavior

Single-line only — `Enter` sends rather than inserting a newline, so there's
no multiline compose mode yet. Typed text is escaped to HTML
(`text::html::plain_text_to_html`) rather than interpreted as markup.

## Status/help presentation

One shared status line at the bottom of the logged-in screen: an error/info
message when `LoggedInState::status` is set, otherwise a static hint
listing the current keymap. Not pane-local — an error from loading channels
and an error from loading messages both land in the same line (see #26/#23's
notes on this being one of the still-open gaps).

## Unicode / text wrapping

Ratatui's own `Wrap { trim: false }` handles wrapping; nothing custom is
done for wide characters, combining marks, etc. beyond what ratatui/
`unicode-width` already provide. Not specifically tested against emoji,
right-to-left text, or other edge cases.

## Post-MVP (per #1's plan, unchanged)

Search and unread-message indicators are explicitly out of scope for now.
