---
id: 64
title: "Add self-update: upgrade/uninstall commands, install scripts, release pipeline"
state: Done
parent: 8
labels: [task, packaging]
created: 2026-09-23T15:31:59Z
updated: 2026-09-23T15:48:16Z
---

## Description

family-chat-cli has no way to update itself or be installed as a managed, upgradeable binary — copy-the-binary-yourself only (README). CodeVachon/family-chat-cli is now a public GitHub repo (was previously local-only), so it can host tagged releases the way CodeVachon/merge-pipeline already does. Match that repo's exact self-update workflow: a managed ~/.family-chat-cli install layout, upgrade/uninstall subcommands, install.sh/install.ps1, and a build-matrix release pipeline in GitHub Actions.

## Plan

Port merge-pipeline's src/selfupdate/{layout,releases,download,upgrade,nudge}.rs, renaming identifiers/env-vars for family-chat-cli (FAMILY_CHAT_CLI_* env vars, distinct from the existing FAMILY_CHAT_URL). Add ureq/flate2/sha2/semver/dirs deps (same versions merge-pipeline resolves to). Wire upgrade/uninstall subcommands in cli.rs/main.rs as early synchronous exits, same pattern as the existing config subcommand, plus a best-effort update nudge printed after the TUI exits. Add install.sh/install.ps1 mirroring merge-pipeline's, and a release.yml (gate -> build matrix -> checksums -> verify -> publish) plus scripts/smoke.sh, trimmed to what applies here (no MCP/git-workflow checks). One deliberate deviation from merge-pipeline: Linux targets build against glibc (x86_64/aarch64-unknown-linux-gnu on native runners), not musl — keyring's dbus-secret-service backend links libdbus via FFI (libdbus-sys), which a musl static build can't satisfy the way a dependency-free tool like merge-pipeline can.

## Notes

### 2026-09-23T15:48:02Z — Christopher Vachon (user)

Ported merge-pipeline's src/selfupdate/{layout,releases,download,upgrade,nudge}.rs wholesale, renaming REPO/BIN_NAME/ROOT_DIR_NAME and every FAMILY_CHAT_CLI_* env var (kept distinct from the pre-existing FAMILY_CHAT_URL). Wired upgrade/uninstall as early synchronous exits in main.rs (same pattern as #37's config subcommand), added crate::VERSION and clap's version=, and a best-effort update nudge printed after the TUI exits (mirrors merge-pipeline's, minus the color-palette dependency it doesn't have a reason to add here).

New deps: ureq (rustls), flate2, sha2, semver, dirs — versions pinned to what merge-pipeline's own Cargo.lock already resolves to. Deliberate deviation: Linux release assets build against glibc, not musl (keyring's dbus-secret-service backend links libdbus via FFI, incompatible with a musl static build) — both Linux legs of the release matrix build natively (ubuntu-latest / ubuntu-24.04-arm) instead of cross-compiling to a static target.

Added install.sh/install.ps1 (checksum-verified, mirroring merge-pipeline's), scripts/smoke.sh (trimmed to what applies here — no MCP/git-workflow checks, just version/help/config/upgrade --check), and .github/workflows/release.yml (gate -> build 5-target matrix -> checksums -> verify + verify-windows -> publish).

Verified live: --version/--help/upgrade --help work (--version was never wired up at all before this); a fake managed install with the real debug binary correctly hit the real GitHub API via upgrade --check and cleanly reported no release published yet (there isn't one); uninstall's dry-run report and --yes --json removal both verified against a fake install, including that a foreign symlink is left alone; scripts/smoke.sh passes against both the debug and release (LTO) profile builds. 195 tests passing (23 new — the FakeServer-backed integration tests from merge-pipeline's tests/selfupdate.rs live as unit tests in upgrade.rs's own #[cfg(test)] module, since this crate has no lib.rs for a separate tests/ file to import as an external crate).

README/architecture.md updated. Not yet done (follow-up, once ready to actually publish): tag v0.1.0 and push it to trigger the release workflow for real, to confirm the whole pipeline end-to-end against real GitHub Releases rather than the local fakes used here.
