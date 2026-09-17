---
id: 34
title: Define config file format and locations
state: Done
parent: 7
labels: [task, config]
blockedBy: [10]
created: 2026-08-25T21:38:35Z
updated: 2026-09-17T14:37:36Z
---

## Description

Choose where non-secret CLI settings live and how profiles are represented.

## Plan

Use a small TOML config for API base URL, profile name, optional defaults, and non-secret UI preferences.

## Notes

### 2026-09-17T14:37:35Z — Christopher Vachon (user)

Defined: config::Config (src/config/mod.rs) with server, profile, and default_channel — all Option and #[serde(default)]-backed so the format is additive (an empty file, or an older file missing a field, still parses). Location: config::path() resolves to $XDG_CONFIG_HOME/family-chat-cli/config.toml, falling back to $HOME/.config/family-chat-cli/config.toml — the same XDG-with-fallback convention main::log_dir already uses for the state directory. path()'s env-var resolution is split into a pure resolve_path() helper so it's unit-testable without mutating the real process environment.

The plan's fourth category (non-secret UI preferences) has no field yet — nothing existing needs one, and #[serde(default)] means adding one later (or a nested table) is additive, not breaking.

Deliberately out of scope here: loading/saving/validating the file and CLI-flag precedence (#35), profile-aware credential namespacing (#36), and config subcommands (#37). Nothing in this module is wired into main yet, hence the #[allow(dead_code)] at the module level — that comes off once #35 consumes it.

6 new unit tests (path resolution with/without XDG_CONFIG_HOME, missing-env error case, empty/partial/full TOML round-trips). 61 tests passing overall, clippy/fmt clean.
