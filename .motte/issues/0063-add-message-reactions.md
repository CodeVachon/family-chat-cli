---
id: 63
title: Add message reactions
state: Done
parent: 6
labels: [task, chat]
created: 2026-09-21T20:03:33Z
updated: 2026-09-21T20:19:48Z
---

## Description

Messages can carry reactions (server field 'reactions': [{emoji, count, reactedByMe}], PUT|DELETE /messages/:id/reactions/:emoji, a curated 10-emoji set) but the CLI never parsed or showed them, and had no way to add/remove one.

## Plan

Parse Message.reactions and add RealtimeEvent::ReactionChanged (confirmed via the backend's message_reactions_notify Postgres trigger). From Compose focus with an empty draft, Left targets a message to react to (mirroring Right for thread-reply mode, mutually exclusive with it); Up/Down move the target; a digit 1-9/0 toggles the matching curated emoji; Esc cancels. Render each message's reactions as 'emoji count', with the current user's own reactions styled distinctly, and highlight the currently reaction-targeted message.

## Notes

### 2026-09-21T20:03:56Z — Christopher Vachon (user)

Implemented exactly per plan. Message.reactions: Vec<Reaction> parses {emoji,count,reactedByMe}; REACTION_EMOJIS is the curated 10-emoji set from the backend's apps/web/lib/validation/channel.ts. ApiClient::add_reaction/remove_reaction hit PUT|DELETE /messages/:id/reactions/:emoji (confirmed reqwest/url percent-encodes the raw emoji path segment correctly via a wiremock test asserting the exact %F0%9F%91%8D wire bytes). RealtimeEvent::ReactionChanged shape confirmed against the backend's packages/db/drizzle/0007_reaction_trigger.sql trigger (not just the TS type union, which doesn't show whether it's actually ever published).

State: LoggedInState gained reaction_target/reaction_error, mirroring thread_reply_target/thread's error handling. Left (empty compose, neither mode active) targets the last visible message; Up/Down move it (shared move_target helper refactored out of move_thread_selection); a digit toggles the corresponding curated emoji, decided by that message's own cached reactedByMe rather than tracked separately. Switching channels clears reaction_target the same way it already cleared thread_reply_target.

Rendering: message_lines appends an 'emoji count' line per reaction (reacted-by-me ones in bold yellow); messages_to_lines/windowed_messages gained a reaction_target parameter to prefix the targeted message's first line with a yellow '» ' marker; compose title becomes 'React', status line becomes 'Reacting — ↑/↓ change message · 1👍 2❤️ ... 0✅ · Esc cancel'.

Realtime: a reaction could belong to an already-cached thread reply, which a plain messages reload never touches (only top-level messages come back from that endpoint) — reused threads_pending_refresh (#61) to mark every currently cached thread stale on any reaction.changed for the selected channel, rather than adding a message-id->root lookup.

Verified live end-to-end in the real 'Testing' channel via tmux: toggled a reaction on then off through the actual UI with immediate visible feedback, confirmed via curl (GET .../messages) that the server's own reactedByMe state matched at each step, and confirmed a reaction added via a separate direct API call (simulating another client) appeared live with zero local action, via the reaction.changed SSE event.

163 tests passing (17 new: 12 state, 2 client wiremock, 3 widget render), clippy/fmt clean.

### 2026-09-21T20:19:48Z — Christopher Vachon (user)

Follow-up polish: the reaction summary line sat flush at the pane's left edge, out of place under the timestamp/author prefix. Indented it 8 columns to align under the author name ("[HH:MM] " is always 8 columns). Also fixed mark_reaction_target to pad every other line of the marked message by the marker's 2-column width, not just leave the first line prefixed — otherwise the reaction line under the currently-targeted message (the one most likely being looked at) drifted 2 columns out of alignment relative to every other message. Verified live in Testing with two reactions on the targeted message. See commit 0f36af2.
