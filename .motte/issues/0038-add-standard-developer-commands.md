---
id: 38
title: Add standard developer commands
state: Done
parent: 8
assignee: Christopher Vachon
labels: [task, packaging]
blockedBy: [11]
created: 2026-08-25T21:38:35Z
updated: 2026-09-16T23:45:22Z
---

## Description

Make local development predictable.

## Plan

Define cargo fmt, cargo clippy, cargo test, cargo run, and any Makefile/justfile aliases the repo prefers.

## Notes

### 2026-09-16T23:45:21Z — Christopher Vachon (user)

Added a Makefile with fmt, fmt-check, clippy (all-targets, -D warnings), test, build, run, and a preflight target that chains fmt-check+clippy+test — the single command CI also runs (#46), so they can't drift apart. Verified: make preflight passes clean locally.
