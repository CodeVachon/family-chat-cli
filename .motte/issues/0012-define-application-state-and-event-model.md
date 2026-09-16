---
id: 12
title: Define application state and event model
state: Done
parent: 2
assignee: Christopher Vachon
labels: [task, architecture]
blockedBy: [11]
created: 2026-08-25T21:38:34Z
updated: 2026-09-16T19:56:30Z
---

## Description

Design the internal state shape that the TUI and async API tasks will share.

## Plan

Define state structs for session, channels, selected channel, messages, users, compose buffer, loading/error state, and app events.

## Notes

### 2026-09-16T19:56:29Z — Christopher Vachon (user)

Implemented in src/app/{state.rs,event.rs}: AppState/Screen (Resuming, LoggedOut(LoginForm), LoggingIn, NotApproved, LoggedIn(LoggedInState)), Event (async results delivered over an mpsc channel — key input is handled directly as crossterm::event::KeyEvent, no round trip needed), and Command (the async side effects the pure state layer asks the loop to perform). state.rs is pure/no-IO by construction and has 6 unit tests covering login-form key handling and screen transitions. Verified: cargo test.
