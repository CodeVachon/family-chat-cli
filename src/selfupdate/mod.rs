//! Managed install layout, release lookup, download/verify, upgrade and uninstall (#64).
//!
//! Mirrors the same mechanism `CodeVachon/merge-pipeline` already uses, so a machine with both
//! tools installed behaves identically for each:
//!
//! ```text
//! <root>/versions/v<X.Y.Z>/bin/family-chat-cli   the binary
//! <root>/current -> versions/v<X.Y.Z>            the active version
//! <bin>/family-chat-cli -> <root>/current/bin/family-chat-cli
//! <root>/update-check.json                       when we last looked for a release
//! ```
//!
//! `<root>` is `~/.family-chat-cli` unless `FAMILY_CHAT_CLI_INSTALL_DIR` says otherwise, and
//! `<bin>` is `~/.local/bin` unless `FAMILY_CHAT_CLI_BIN_DIR` says otherwise. `install.sh` and
//! `install.ps1` implement the same layout; `layout::tests` reads `install.sh` to make sure they
//! agree.
//!
//! Deliberately synchronous (via `ureq`), not built on the app's own `reqwest`/tokio stack:
//! `upgrade`/`uninstall` run as a one-shot subcommand dispatched directly from `main` before the
//! TUI (or its async runtime) ever starts, the same way the `config` subcommand does.

// Some items here (e.g. `layout::install_root`, `layout::HostTarget::exe_name`,
// `releases::resolve_latest_version`) aren't called by this binary's own production code yet —
// they exist to keep this module's shape matching merge-pipeline's, exercised directly by this
// module's own tests, and available for whatever comes next (a `doctor`-style diagnostic command,
// for instance) without redesigning the module then.
#[allow(dead_code)]
pub mod download;
#[allow(dead_code)]
pub mod layout;
pub mod nudge;
#[allow(dead_code)]
pub mod releases;
pub mod upgrade;

use std::io::Write;

// Re-exported at the module root the same way merge-pipeline's does, even though this binary
// crate only calls a couple of these paths directly right now — the rest are reachable through
// `selfupdate::layout`/`download`/`releases` either way. A binary crate has no external consumer
// for a `pub use` to serve, so rustc's `unused_imports` fires on the ones nothing here calls yet.
#[allow(unused_imports)]
pub use download::{
    DownloadError, PruneResult, fetch_verified_binary, install_binary, prune_versions,
};
#[allow(unused_imports)]
pub use layout::{
    Arch, BIN_NAME, HostTarget, ManagedInstall, Platform, REPO, UnsupportedHostError,
    download_base, host_target, install_root, installed_versions, locate_install,
    normalize_version,
};
#[allow(unused_imports)]
pub use releases::{ReleaseError, resolve_latest_version};
#[allow(unused_imports)]
pub use upgrade::{UninstallOptions, UpgradeContext, UpgradeError, UpgradeOptions};

use crate::cli::{UninstallArgs, UpgradeArgs};

impl From<&UpgradeArgs> for UpgradeOptions {
    fn from(args: &UpgradeArgs) -> Self {
        Self {
            target: args.target.clone(),
            check: args.check,
            keep: Some(args.keep),
            force: args.force,
            json: args.json,
        }
    }
}

impl From<&UninstallArgs> for UninstallOptions {
    fn from(args: &UninstallArgs) -> Self {
        Self {
            yes: args.yes,
            json: args.json,
        }
    }
}

/// Exit the process with `code` when it is non-zero, flushing stdout first.
///
/// `uninstall` without `--yes` reports what it would do and exits 1 without that being an
/// error, so it cannot travel back through `anyhow::Result` without being printed as one.
fn finish(code: i32) -> anyhow::Result<()> {
    if code != 0 {
        let _ = std::io::stdout().flush();
        std::process::exit(code);
    }
    Ok(())
}

/// `family-chat-cli upgrade`: entry point used by `main`.
pub fn run_upgrade(args: &UpgradeArgs) -> anyhow::Result<()> {
    let options = UpgradeOptions::from(args);
    let code = upgrade::run_upgrade(&options, &mut std::io::stdout())?;
    finish(code)
}

/// `family-chat-cli uninstall`: entry point used by `main`.
pub fn run_uninstall(args: &UninstallArgs) -> anyhow::Result<()> {
    let options = UninstallOptions::from(args);
    let code = upgrade::run_uninstall(&options, &mut std::io::stdout())?;
    finish(code)
}
