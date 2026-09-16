---
id: 46
title: Add CI or preflight check
state: Done
parent: 9
assignee: Christopher Vachon
labels: [task, quality]
blockedBy: [11]
created: 2026-08-25T21:38:35Z
updated: 2026-09-16T23:45:23Z
---

## Description

Run the project’s core verification commands consistently.

## Plan

Immediately after scaffolding, add one local preflight command and CI workflow covering formatting checks, Clippy with warnings denied, and the current test suite. Keep the same command green as later test tasks add coverage. Verify it from a clean checkout and record any required toolchain components.

## Notes

### 2026-09-16T23:45:22Z — Christopher Vachon (user)

Added .github/workflows/ci.yml: checks out, installs libdbus-1-dev+pkg-config (keyring's Linux Secret Service backend links against system D-Bus), sets up stable Rust with rustfmt+clippy via dtolnay/rust-toolchain, caches via Swatinem/rust-cache, then runs 'make preflight' — the exact same command as local dev (#38), so CI can't silently diverge from what passes locally. Verified: make preflight passes locally from a clean state; YAML syntax validated. NOT verified: an actual GitHub Actions run — this repo has no git remote configured yet, so the workflow has never executed on real CI infrastructure. Worth a first real run once this is pushed somewhere.
