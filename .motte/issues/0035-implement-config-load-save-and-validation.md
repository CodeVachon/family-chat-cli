---
id: 35
title: Implement config load/save and validation
state: Done
parent: 7
labels: [task, config]
blockedBy: [34]
created: 2026-08-25T21:38:35Z
updated: 2026-09-17T15:14:31Z
---

## Description

Read, validate, and update local non-secret settings.

## Plan

Add clear errors for missing base URL, invalid URL, malformed config, and unsupported profile selection.

## Notes

### 2026-09-17T15:14:30Z — Christopher Vachon (user)

Completed. Config::validate() (src/config/mod.rs) covers the four error cases from the plan: an explicitly-blank server URL, a syntactically invalid URL, a non-http(s) scheme, and a profile name that isn't safe as a keyring namespace component (non-empty, ASCII alphanumeric/-/_ only — anticipating #36's use of it as a keyring service-name suffix). config::load() reads the file at config::path(), treats a missing file as Config::default() (a fresh install, not an error), and surfaces a malformed file or an invalid value as a real error rather than silently falling back to defaults — a typo should be visible immediately, not silently ignored. config::save() validates then writes, creating the parent directory as needed.

Also wired the CLI/config precedence promised in #34's Config doc comment: cli::Cli::server became Option<String> (no more hardcoded default_value) so main.rs can tell whether --server/FAMILY_CHAT_URL was actually set; main::resolve_server layers CLI/env > config file > a built-in DEFAULT_SERVER constant (the same URL that used to live in cli.rs's default_value).

save(), Config::profile, and Config::default_channel are still #[allow(dead_code)] — there's no config-editing UI yet (#37 owns that) and nothing consumes profile (#36) or default_channel (no "jump to channel on startup" feature exists) yet.

16 new tests: validation cases (blank/malformed/wrong-scheme URL, empty/unsafe profile name), load/save round-trips against a real tempdir (missing file, malformed TOML, invalid value, nested-directory creation, refusing to write an invalid config), and the three-way CLI/config/default precedence in main.rs. Verified live: the app starts fine with no config file present (falls through to the default, unchanged behavior), and a hand-written malformed config.toml fails fast at startup with "config file at ... is not valid TOML" plus the underlying parse error, rather than a panic or a silent fallback. 77 tests passing, clippy/fmt clean.
