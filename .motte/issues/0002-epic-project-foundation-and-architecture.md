---
id: 2
title: "Epic: Project foundation and architecture"
state: Done
parent: 1
labels: [epic, architecture]
created: 2026-08-25T21:37:48Z
updated: 2026-09-16T19:58:17Z
---

## Description

Establish the Rust CLI/TUI project structure, dependency choices, runtime model, and core application boundaries.

## Plan

Create a Rust binary using Ratatui/Crossterm for terminal UI, Tokio for async work, Reqwest/Serde for REST, Clap for command entry points, and clear modules for auth, API, app state, TUI rendering, input handling, and persistence.
