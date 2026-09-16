---
id: 11
title: Scaffold Rust binary project
state: Done
parent: 2
assignee: Christopher Vachon
labels: [task, architecture]
blockedBy: [10]
created: 2026-08-25T21:38:34Z
updated: 2026-09-16T19:34:45Z
---

## Description

Initialize the Rust application layout and baseline commands.

## Plan

Create Cargo project files, main entrypoint, module skeletons, formatting/lint defaults, and a placeholder executable that starts cleanly.

## Notes

### 2026-09-16T19:34:45Z — Christopher Vachon (user)

Scaffolded per docs/architecture.md: Cargo.toml with the verified dependency set, .gitignore, rustfmt.toml, and the module skeleton (cli, config, auth::{login,store}, api::{client,types,stream,error}, app::{state,event}, tui::{layout,widgets}, text::html) as empty stub files with a one-line doc comment naming the issue that will fill each one in. main.rs initializes tracing (stderr for now — moves to a file once #13 puts the terminal in raw/alt-screen mode), parses an empty clap::Cli, and prints a placeholder line. Verified: cargo build (clean, no warnings), cargo run (prints 'family-chat-cli: scaffold ok', exits 0), cargo fmt --check (clean), cargo clippy --all-targets -- -D warnings (clean).
