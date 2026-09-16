---
id: 10
title: Choose initial Rust crate structure and dependency set
state: Done
parent: 2
assignee: Christopher Vachon
labels: [task, architecture]
blockedBy: [14, 48]
created: 2026-08-25T21:38:34Z
updated: 2026-09-16T19:30:46Z
---

## Description

Create the project architecture decision for a Rust binary centered on a TUI.

## Plan

After tasks 14 and 48 establish protocol requirements, record an architecture decision covering the Rust toolchain/MSRV, one binary crate versus workspace, module boundaries, dependency choices and feature flags, target platforms, test seams for auth and API clients, and keychain backend implications. Verify by reviewing the decision against every MVP capability and by producing a dependency tree with no unexplained duplicate runtime stacks.

## Notes

### 2026-09-16T19:30:46Z — Christopher Vachon (user)

Architecture decision recorded in docs/architecture.md: single binary crate (no workspace), edition 2024, module layout (auth/api/app/tui/text), and a verified dependency set (tokio, ratatui+crossterm event-stream, reqwest+rustls+stream, reqwest-eventsource, serde, clap, keyring, chrono, thiserror+anyhow, tracing family, tl for HTML). Verified by building a scratch crate with this exact set: cargo check clean, cargo tree -d shows only harmless build-time proc-macro duplication (thiserror v1/v2, syn v2/v3, unicode-width from ratatui itself), no duplicate TLS/HTTP/async-runtime stacks. Chose tl over scraper for HTML parsing after confirming scraper pulls in the full html5ever/selectors/cssparser/phf CSS stack for no benefit here. Test seams: ApiClient against a wiremock mock server, CredentialStore trait with a fake in-memory impl, pure app::state reducers with no IO.
