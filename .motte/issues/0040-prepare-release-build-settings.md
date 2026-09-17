---
id: 40
title: Prepare release build settings
state: Done
parent: 8
labels: [task, packaging]
blockedBy: [38]
created: 2026-08-25T21:38:35Z
updated: 2026-09-17T16:22:04Z
---

## Description

Tune binary build and packaging basics.

## Plan

Set release profile preferences if useful and document building/installing the native binary.

## Notes

### 2026-09-17T16:22:03Z — Christopher Vachon (user)

Completed. Cargo.toml's [profile.release] now has codegen-units = 1 and strip = true alongside the existing lto = true — smaller, faster binary at the cost of release-build time, the right tradeoff for a binary built rarely and run often. Deliberately did not set panic = "abort": background tokio tasks (message loads, the SSE stream) already isolate a panic to just that task, and aborting the whole process on any single one of them would trade a recoverable failure for an unrecoverable one — not worth it for a binary-size/perf tweak.

Added README.md (there wasn't one at all) covering: building (cargo build --release / make release, and why release is slower but smaller), installing (copy the binary to a PATH directory, or cargo install --path .), running (the --server/FAMILY_CHAT_URL/config-file precedence, log location), and pointers to the Makefile targets and docs/ for the rest. Login/everyday-use walkthrough is explicitly left to #39, which owns that content — didn't want to duplicate or preempt it.

Added a `make release` target alongside the existing `make build`, matching the README's instructions.

Verified: cargo build --release succeeds (took a while — LTO + codegen-units=1 is genuinely slow to compile, as expected), produces a 5.7M stripped binary, and runs correctly against the live production server (confirmed via a real tmux session, same as every other live verification this session). 93 tests passing, clippy/fmt clean.
