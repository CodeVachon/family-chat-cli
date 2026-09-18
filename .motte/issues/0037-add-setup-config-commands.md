---
id: 37
title: Add setup/config commands
state: Done
parent: 7
labels: [task, config]
blockedBy: [35]
created: 2026-08-25T21:38:35Z
updated: 2026-09-18T15:16:28Z
---

## Description

Let users configure the CLI without manually editing files.

## Plan

Implement commands such as config set api-url, config get, and config path as appropriate.

## Notes

### 2026-09-18T15:16:27Z — Christopher Vachon (user)

Implemented config get/set/unset/path as subcommands under 'config'. Design: ConfigKey lives in config/mod.rs (derives clap::ValueEnum) with kebab-case names (server, profile, default-channel); Config gained get/set/unset methods keyed on it. cli.rs adds Cli::command: Option<Command> (Config { action: ConfigAction }) alongside the existing --server flag. main.rs dispatches the subcommand as an early synchronous exit before init_logging/the TUI, since config command output is meant to print directly to the terminal. set/unset go through the existing load -> mutate -> config::save() path, so Config::validate() rejects a bad value before anything touches disk.

Removed the #[allow(dead_code)] on default_channel and save() now that both are actually used.

Verified manually against a throwaway XDG_CONFIG_HOME: path, get (all + single key), set (all three keys), unset, and the file's real TOML contents all round-tripped correctly; setting an invalid server URL was rejected with a clear multi-line error and wrote nothing to disk; running with no subcommand still launches the TUI unchanged. Added README 'Configuring' section with the four commands. 146 tests passing (7 new), clippy/fmt clean.
