---
id: 50
title: Load users and presence into TUI
state: Done
parent: 6
labels: [task, chat, mvp]
blockedBy: [21, 27, 29]
created: 2026-08-25T21:45:44Z
updated: 2026-09-17T17:08:36Z
---

## Description

Populate and refresh the users pane using the API capability confirmed for the MVP.

## Plan

Fetch users for the selected scope, render stable identity fields, handle absent or unsupported presence data, and preserve selection across refreshes.

## Notes

### 2026-09-17T17:08:35Z — Christopher Vachon (user)

Completed. Added a Users pane (24-col third column, right of the main area) listing the selected channel's members via #21's channel_members endpoint, with a real online/offline marker.

Fetching: tui::run's Command::LoadMessages handling now also spawns a members fetch, extending the same reasoning already applied to mark_channel_read (#33) — every LoadMessages means "the user is looking at this channel," which is also the right moment to keep its member list current. Refetched every time (not cached-once), so the manual refresh key ('r') refreshes users too, satisfying both "populate" and "refresh" from the plan via the same existing trigger. No new Command variant needed — the result comes back through a new Event::MembersLoaded, handled by a new on_members_loaded, mirroring how mark_channel_read already has no Command counterpart.

Presence: rather than guess at the SSE payload shape from docs alone (the read.updated incident earlier this session was a direct lesson in that), eavesdropped on the raw stream with curl to capture a real presence.snapshot payload: {"type":"presence.snapshot","onlineUserIds":[...],"ts":...}, sent once right after connecting, before "ready". Modeled that as RealtimeEvent::PresenceSnapshot. The incremental per-user presence event (someone going online/offline mid-session) is deliberately left unmodeled — its exact delta shape never showed up in a live capture (tried triggering one with a second connection; no luck in the time available), and guessing wrong risks silently misreporting someone's status. It falls safely into Other; presence only updates on the next reconnect's snapshot in the meantime. This directly satisfies the plan's "handle absent or unsupported presence data" — the gap is real and explicit, not silently papered over.

Rendered as a filled (online, confirmed by the snapshot) or hollow (unknown — never shown as a false "offline") dot per member. No selection/focus of its own: nothing in the app acts on a specific selected member yet, so "preserve selection across refreshes" is satisfied by construction — loading members never touches channel selection, and there's no separate users-pane selection to lose.

Re-derived the minimum terminal-size floor from #28 (50x12 → 80x12): the old floor left the main pane at zero width once a second fixed-width column (24 cols) existed alongside the channels sidebar (28 cols). Re-ran the same rendering-based probe methodology #28 used across a range of sizes with the real 3-column layout to pick the new number from evidence, not a guess.

Verified live against the real account: "Christopher & Louise" (2 members) shows both online, matching a live presence.snapshot with exactly those two ids; "The Vachons" (4 members) correctly shows a mix of online (Louise, Christopher) and unknown/offline (Martin Vachon, Rachel). No new errors in the log from this verification run.

98 tests passing (6 new), clippy/fmt clean.
