---
id: 46
title: Add CI or preflight check
state: Todo
parent: 9
labels: [task, quality]
blockedBy: [11]
created: 2026-08-25T21:38:35Z
updated: 2026-08-25T21:47:04Z
---

## Description

Run the project’s core verification commands consistently.

## Plan

Immediately after scaffolding, add one local preflight command and CI workflow covering formatting checks, Clippy with warnings denied, and the current test suite. Keep the same command green as later test tasks add coverage. Verify it from a clean checkout and record any required toolchain components.
