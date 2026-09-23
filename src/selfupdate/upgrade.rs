//! `family-chat-cli upgrade` and `family-chat-cli uninstall`.

use std::cmp::Ordering;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

use super::download::{DownloadError, fetch_verified_binary_from, install_binary, prune_versions};
use super::layout::{
    HostTarget, ManagedInstall, REPO, UnsupportedHostError, candidate_bin_dirs,
    compare_versions_descending, download_base, host_target, installed_versions, locate_install,
    normalize_version,
};
use super::releases::{ReleaseError, api_base, resolve_latest_version_at};

/// Printed when the running binary is not part of a managed installation.
pub const INSTALL_HINT: &str = "family-chat-cli is not running from a managed installation, so there is nothing to upgrade in place.
  This happens when running from source, or when the binary was copied somewhere by hand.
  To install a managed copy:
    curl -fsSL https://raw.githubusercontent.com/CodeVachon/family-chat-cli/main/install.sh | sh";

/// Why an upgrade or uninstall did not complete.
#[derive(Debug, Error)]
pub enum UpgradeError {
    #[error("{INSTALL_HINT}")]
    NotInstalled,
    #[error(transparent)]
    UnsupportedHost(#[from] UnsupportedHostError),
    #[error(transparent)]
    Release(#[from] ReleaseError),
    #[error(transparent)]
    Download(#[from] DownloadError),
    #[error("could not update the installation: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not write output: {0}")]
    Output(std::io::Error),
}

/// Flags for `upgrade`.
#[derive(Debug, Clone, Default)]
pub struct UpgradeOptions {
    /// Install this version instead of the newest.
    pub target: Option<String>,
    /// Report whether an update is available, changing nothing.
    pub check: bool,
    /// How many versions to keep on disk (default 2).
    pub keep: Option<usize>,
    /// Reinstall even if already on the target version.
    pub force: bool,
    /// Machine-readable output.
    pub json: bool,
}

/// Flags for `uninstall`.
#[derive(Debug, Clone, Default)]
pub struct UninstallOptions {
    /// Skip the confirmation.
    pub yes: bool,
    /// Machine-readable output.
    pub json: bool,
}

/// Everything `upgrade` needs from its environment, so tests can supply a fake install,
/// version and servers without touching the real home directory.
#[derive(Debug, Clone)]
pub struct UpgradeContext {
    pub install: ManagedInstall,
    pub current_version: String,
    pub host: HostTarget,
    pub api_base: String,
    /// Base URL assets are downloaded from, or `None` to derive it from the version.
    pub download_base: Option<String>,
    pub api_timeout: Duration,
}

impl UpgradeContext {
    /// The real environment: the running binary's install, version and host.
    pub fn detect() -> Result<Self, UpgradeError> {
        let install = locate_install().ok_or(UpgradeError::NotInstalled)?;
        Ok(Self {
            install,
            current_version: crate::VERSION.to_owned(),
            host: host_target()?,
            api_base: api_base(),
            download_base: None,
            api_timeout: Duration::from_secs(20),
        })
    }
}

/// What `update-check.json` records.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckRecord {
    pub last_attempt_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_success_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest: Option<String>,
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Path of the update-check cache under `root`.
pub fn check_record_path(root: &Path) -> PathBuf {
    root.join("update-check.json")
}

/// Read the cache, tolerating absence and corruption.
pub fn read_check_record(root: &Path) -> Option<CheckRecord> {
    let text = fs::read_to_string(check_record_path(root)).ok()?;
    serde_json::from_str(&text).ok()
}

/// Record a lookup attempt and, when it succeeded, the version it found. Best effort.
pub fn record_check(root: &Path, latest: Option<&str>) {
    let now = now_millis();
    let mut record = read_check_record(root).unwrap_or_default();
    record.last_attempt_at = now;
    if let Some(latest) = latest {
        record.last_success_at = Some(now);
        record.latest = Some(latest.to_owned());
    }
    if let Ok(text) = serde_json::to_string_pretty(&record) {
        let _ = fs::create_dir_all(root);
        let _ = fs::write(check_record_path(root), format!("{text}\n"));
    }
}

fn emit(out: &mut dyn Write, line: impl AsRef<str>) -> Result<(), UpgradeError> {
    writeln!(out, "{}", line.as_ref()).map_err(UpgradeError::Output)
}

fn emit_json(out: &mut dyn Write, value: serde_json::Value) -> Result<(), UpgradeError> {
    emit(
        out,
        serde_json::to_string_pretty(&value).unwrap_or_default(),
    )
}

/// Run `upgrade` against the real environment. Returns the process exit code.
pub fn run_upgrade(options: &UpgradeOptions, out: &mut dyn Write) -> Result<i32, UpgradeError> {
    let ctx = UpgradeContext::detect()?;
    run_upgrade_with(options, &ctx, out)
}

/// Run `upgrade` with an explicit context. Returns the process exit code.
pub fn run_upgrade_with(
    options: &UpgradeOptions,
    ctx: &UpgradeContext,
    out: &mut dyn Write,
) -> Result<i32, UpgradeError> {
    let install = &ctx.install;
    let wanted = match &options.target {
        Some(target) => normalize_version(target),
        None => {
            let latest = resolve_latest_version_at(&ctx.api_base, ctx.api_timeout);
            record_check(&install.root, latest.as_deref().ok());
            normalize_version(&latest?)
        }
    };
    let current = normalize_version(&ctx.current_version);

    // Semantic comparison, so `upgrade 0.0.9` is recognised as a downgrade rather than treated
    // like an upgrade. `compare_versions_descending(a, b)` is Greater when a is OLDER than b.
    let ordering = compare_versions_descending(&wanted, &current);
    let up_to_date = ordering == Ordering::Equal;
    let is_downgrade = ordering == Ordering::Greater;

    if options.check {
        if options.json {
            emit_json(
                out,
                json!({
                    "current": current,
                    "latest": wanted,
                    "upToDate": up_to_date,
                    "isDowngrade": is_downgrade,
                    "installRoot": install.root,
                    "installed": installed_versions(&install.versions_dir),
                }),
            )?;
            return Ok(0);
        }
        if up_to_date {
            emit(
                out,
                format!("✓ family-chat-cli {current} is the newest release"),
            )?;
        } else {
            emit(
                out,
                format!("✓ family-chat-cli {wanted} is available (you have {current})"),
            )?;
            emit(out, "  family-chat-cli upgrade")?;
        }
        return Ok(0);
    }

    if up_to_date && !options.force {
        emit(
            out,
            format!("✓ already on {current} — pass --force to reinstall"),
        )?;
        return Ok(0);
    }

    if is_downgrade {
        emit(
            out,
            format!("! {wanted} is older than the running {current}"),
        )?;
    }

    emit(
        out,
        format!(
            "downloading family-chat-cli {wanted} for {}-{}...",
            ctx.host.platform.as_str(),
            ctx.host.arch.as_str()
        ),
    )?;
    let base = ctx
        .download_base
        .clone()
        .unwrap_or_else(|| download_base(&wanted));
    let binary = fetch_verified_binary_from(&base, &wanted, &ctx.host)?;
    emit(out, "✓ checksum verified")?;

    install_binary(&install.root, &wanted, &binary, ctx.host.is_windows())?;
    emit(out, format!("✓ installed {wanted}"))?;

    let ordered = installed_versions(&install.versions_dir);
    let pruned = prune_versions(
        &install.versions_dir,
        &ordered,
        options.keep.unwrap_or(2),
        &install.version,
    );

    if !pruned.removed.is_empty() {
        emit(out, format!("pruned {}", pruned.removed.join(", ")))?;
    }
    if let Some(kept) = &pruned.kept_running {
        emit(
            out,
            format!(
                "kept {kept} — it is the binary currently running; the next upgrade will remove it"
            ),
        )?;
    }

    if options.json {
        let mut value = json!({
            "from": current,
            "to": wanted,
            "pruned": pruned.removed,
            "installRoot": install.root,
        });
        if let Some(kept) = &pruned.kept_running {
            value["keptRunning"] = json!(kept);
        }
        emit_json(out, value)?;
        return Ok(0);
    }

    emit(out, "")?;
    emit(
        out,
        "Open a new shell, or run `family-chat-cli --version` to confirm.",
    )?;
    Ok(0)
}

/// Symlinks on PATH that resolve into `root`. A `family-chat-cli` from a package manager or a
/// different installation is not ours to delete.
pub fn owned_bin_links(root: &Path) -> Vec<PathBuf> {
    let root = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let mut links = Vec::new();
    for dir in candidate_bin_dirs() {
        for name in ["family-chat-cli", "family-chat-cli.exe"] {
            let link = dir.join(name);
            if fs::symlink_metadata(&link).is_err() {
                continue;
            }
            if let Ok(real) = fs::canonicalize(&link)
                && real.starts_with(&root)
                && !links.contains(&link)
            {
                links.push(link);
            }
        }
    }
    links
}

/// Run `uninstall` against the real environment. Returns the process exit code.
pub fn run_uninstall(options: &UninstallOptions, out: &mut dyn Write) -> Result<i32, UpgradeError> {
    let install = locate_install().ok_or(UpgradeError::NotInstalled)?;
    run_uninstall_with(options, &install, out)
}

/// Run `uninstall` for an explicit installation. Returns the process exit code.
pub fn run_uninstall_with(
    options: &UninstallOptions,
    install: &ManagedInstall,
    out: &mut dyn Write,
) -> Result<i32, UpgradeError> {
    let links = owned_bin_links(&install.root);

    if !options.yes {
        emit(
            out,
            format!("! this will remove {}", install.root.display()),
        )?;
        for link in &links {
            emit(out, format!("  and the symlink {}", link.display()))?;
        }
        emit(out, "")?;
        emit(
            out,
            "Your login session and config file are untouched — only the installation is removed.",
        )?;
        emit(out, "Re-run with --yes to proceed.")?;
        return Ok(1);
    }

    for link in &links {
        let _ = fs::remove_file(link);
    }
    fs::remove_dir_all(&install.root)?;

    let mut removed: Vec<PathBuf> = vec![install.root.clone()];
    removed.extend(links.iter().cloned());

    if options.json {
        emit_json(out, json!({ "removed": removed }))?;
        return Ok(0);
    }

    for path in &removed {
        emit(out, format!("✓ removed {}", path.display()))?;
    }
    emit(out, "")?;
    emit(
        out,
        format!(
            "Reinstall any time: curl -fsSL https://raw.githubusercontent.com/{REPO}/main/install.sh | sh"
        ),
    )?;
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_record_round_trips_and_tolerates_corruption() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        assert!(read_check_record(&root).is_none());

        record_check(&root, None);
        let first = read_check_record(&root).unwrap();
        assert!(first.last_attempt_at > 0);
        assert!(first.latest.is_none());

        record_check(&root, Some("v1.2.3"));
        let second = read_check_record(&root).unwrap();
        assert_eq!(second.latest.as_deref(), Some("v1.2.3"));
        assert!(second.last_success_at.is_some());

        fs::write(check_record_path(&root), "{ not json").unwrap();
        assert!(read_check_record(&root).is_none());
        record_check(&root, Some("v2.0.0"));
        assert_eq!(
            read_check_record(&root).unwrap().latest.as_deref(),
            Some("v2.0.0")
        );
    }

    #[test]
    fn install_hint_names_the_installer() {
        assert!(INSTALL_HINT.contains("CodeVachon/family-chat-cli/main/install.sh"));
        assert!(
            UpgradeError::NotInstalled
                .to_string()
                .contains("install.sh | sh")
        );
    }

    // --- FakeServer-backed integration tests --------------------------------
    //
    // A local std TcpListener stands in for the GitHub API and release-asset host, so nothing
    // here touches the real network. Ported from merge-pipeline's tests/selfupdate.rs — this
    // crate has no lib.rs, so there is no external-crate boundary for a separate `tests/`
    // integration file to cross, and this lives as a unit test instead.

    use std::collections::HashMap;
    use std::io::Read as _;
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;

    use flate2::Compression;
    use flate2::write::GzEncoder;

    use super::super::download::sha256_hex;
    use super::super::layout::{host_target_for, locate_install_from};

    type Routes = Arc<Mutex<HashMap<String, (u16, Vec<u8>)>>>;

    /// Minimal HTTP/1.1 server: one response per path, `Connection: close`.
    struct FakeServer {
        base: String,
        routes: Routes,
    }

    impl FakeServer {
        fn start() -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
            let base = format!("http://{}", listener.local_addr().unwrap());
            let routes: Routes = Arc::new(Mutex::new(HashMap::new()));
            let handler_routes = routes.clone();
            thread::spawn(move || {
                for stream in listener.incoming().flatten() {
                    let routes = handler_routes.clone();
                    thread::spawn(move || serve_one(stream, &routes));
                }
            });
            Self { base, routes }
        }

        fn route(&self, path: &str, status: u16, body: impl Into<Vec<u8>>) {
            self.routes
                .lock()
                .unwrap()
                .insert(path.to_owned(), (status, body.into()));
        }
    }

    fn serve_one(mut stream: std::net::TcpStream, routes: &Routes) {
        stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
        let mut buf = Vec::new();
        let mut chunk = [0u8; 1024];
        loop {
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    buf.extend_from_slice(&chunk[..n]);
                    if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        let request = String::from_utf8_lossy(&buf);
        let path = request
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or("/")
            .to_owned();
        let (status, body) = routes
            .lock()
            .unwrap()
            .get(&path)
            .cloned()
            .unwrap_or((404, b"not found".to_vec()));
        let reason = match status {
            200 => "OK",
            403 => "Forbidden",
            404 => "Not Found",
            429 => "Too Many Requests",
            500 => "Internal Server Error",
            _ => "Status",
        };
        let _ = write!(
            stream,
            "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let _ = stream.write_all(&body);
        let _ = stream.flush();
    }

    fn gz(bytes: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(bytes).unwrap();
        encoder.finish().unwrap()
    }

    /// A fake managed install rooted in a temp dir, running `running_version`.
    fn fake_install(root: &Path, running_version: &str) -> ManagedInstall {
        fs::create_dir_all(root).unwrap();
        // Canonical from the start: macOS puts temp dirs under /var, which resolves to /private/var.
        let root = &fs::canonicalize(root).unwrap();
        let bin = root.join("versions").join(running_version).join("bin");
        fs::create_dir_all(&bin).unwrap();
        let exe = bin.join("family-chat-cli");
        fs::write(&exe, b"#!/bin/sh\necho old\n").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(
            root.join("versions").join(running_version),
            root.join("current"),
        )
        .unwrap();
        locate_install_from(&exe).expect("fake install has the managed shape")
    }

    fn context(install: ManagedInstall, current: &str, server: &FakeServer) -> UpgradeContext {
        UpgradeContext {
            install,
            current_version: current.to_owned(),
            host: host_target_for("linux", "x86_64").unwrap(),
            api_base: server.base.clone(),
            download_base: Some(format!("{}/download", server.base)),
            api_timeout: Duration::from_secs(5),
        }
    }

    /// Publish a fake `version` release on the server: API responses plus asset and checksums.
    fn publish(server: &FakeServer, version: &str, binary: &[u8]) {
        let archive = gz(binary);
        let asset = "family-chat-cli-linux-x64.gz";
        server.route(
            "/repos/CodeVachon/family-chat-cli/releases/latest",
            200,
            format!(r#"{{"tag_name":"{version}","draft":false,"prerelease":false}}"#),
        );
        server.route(&format!("/download/{asset}"), 200, archive.clone());
        server.route(
            "/download/checksums.txt",
            200,
            format!(
                "{}  {asset}\nffff  family-chat-cli-darwin-arm64.gz\n",
                sha256_hex(&archive)
            ),
        );
    }

    fn current_target(root: &Path) -> PathBuf {
        fs::read_link(root.join("current")).unwrap()
    }

    #[test]
    fn resolve_latest_prefers_the_stable_release() {
        let server = FakeServer::start();
        server.route(
            "/repos/CodeVachon/family-chat-cli/releases/latest",
            200,
            r#"{"tag_name":"v1.4.0"}"#,
        );
        let tag = resolve_latest_version_at(&server.base, Duration::from_secs(5)).unwrap();
        assert_eq!(tag, "v1.4.0");
    }

    #[test]
    fn resolve_latest_falls_back_to_the_newest_non_draft() {
        let server = FakeServer::start();
        server.route(
            "/repos/CodeVachon/family-chat-cli/releases/latest",
            404,
            r#"{"message":"Not Found"}"#,
        );
        server.route(
            "/repos/CodeVachon/family-chat-cli/releases",
            200,
            r#"[{"tag_name":"v0.3.0","draft":true},{"tag_name":"v0.2.0","draft":false,"prerelease":true}]"#,
        );
        let tag = resolve_latest_version_at(&server.base, Duration::from_secs(5)).unwrap();
        assert_eq!(tag, "v0.2.0");
    }

    #[test]
    fn resolve_latest_reports_rate_limiting_and_missing_releases() {
        let server = FakeServer::start();
        server.route(
            "/repos/CodeVachon/family-chat-cli/releases/latest",
            403,
            r#"{"message":"API rate limit exceeded for 1.2.3.4."}"#,
        );
        let err = resolve_latest_version_at(&server.base, Duration::from_secs(5)).unwrap_err();
        assert!(matches!(err, ReleaseError::RateLimited), "{err}");

        let server = FakeServer::start();
        server.route(
            "/repos/CodeVachon/family-chat-cli/releases/latest",
            404,
            "{}",
        );
        server.route("/repos/CodeVachon/family-chat-cli/releases", 200, "[]");
        let err = resolve_latest_version_at(&server.base, Duration::from_secs(5)).unwrap_err();
        assert!(matches!(err, ReleaseError::NoRelease), "{err}");

        let server = FakeServer::start();
        server.route(
            "/repos/CodeVachon/family-chat-cli/releases/latest",
            404,
            "{}",
        );
        server.route("/repos/CodeVachon/family-chat-cli/releases", 500, "boom");
        let err = resolve_latest_version_at(&server.base, Duration::from_secs(5)).unwrap_err();
        assert!(matches!(err, ReleaseError::Status(500)), "{err}");
    }

    #[test]
    fn fetch_verified_binary_refuses_a_bad_checksum() {
        let server = FakeServer::start();
        let target = host_target_for("linux", "x86_64").unwrap();
        server.route(
            "/download/family-chat-cli-linux-x64.gz",
            200,
            gz(b"payload"),
        );
        server.route(
            "/download/checksums.txt",
            200,
            "0000  family-chat-cli-linux-x64.gz\n",
        );
        let err =
            fetch_verified_binary_from(&format!("{}/download", server.base), "v1.0.0", &target)
                .unwrap_err();
        assert!(err.to_string().contains("checksum mismatch"), "{err}");

        server.route("/download/checksums.txt", 404, "gone");
        let err =
            fetch_verified_binary_from(&format!("{}/download", server.base), "v1.0.0", &target)
                .unwrap_err();
        assert!(err.to_string().contains("HTTP 404"), "{err}");
    }

    #[cfg(unix)]
    #[test]
    fn upgrade_installs_the_new_version_repoints_current_and_prunes() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        // Three older versions already on disk; v0.2.0 is the one "running".
        for old in ["v0.0.1", "v0.1.0"] {
            fs::create_dir_all(root.join("versions").join(old).join("bin")).unwrap();
        }
        let install = fake_install(&root, "v0.2.0");
        let root = install.root.clone();

        let server = FakeServer::start();
        publish(&server, "v0.3.0", b"#!/bin/sh\necho new\n");

        let mut out = Vec::new();
        let code = run_upgrade_with(
            &UpgradeOptions {
                keep: Some(1),
                json: true,
                ..Default::default()
            },
            &context(install, "0.2.0", &server),
            &mut out,
        )
        .unwrap();
        assert_eq!(code, 0);
        let text = String::from_utf8(out).unwrap();

        assert!(
            text.contains("downloading family-chat-cli v0.3.0 for linux-x64..."),
            "{text}"
        );
        assert!(text.contains("✓ checksum verified"), "{text}");
        assert!(text.contains("✓ installed v0.3.0"), "{text}");
        assert!(text.contains("pruned v0.1.0, v0.0.1"), "{text}");
        assert!(
            text.contains("kept v0.2.0 — it is the binary currently running"),
            "{text}"
        );

        let json_start = text.find('{').unwrap();
        let report: serde_json::Value = serde_json::from_str(&text[json_start..]).unwrap();
        assert_eq!(report["from"], "v0.2.0");
        assert_eq!(report["to"], "v0.3.0");
        assert_eq!(report["pruned"], serde_json::json!(["v0.1.0", "v0.0.1"]));
        assert_eq!(report["keptRunning"], "v0.2.0");
        assert_eq!(report["installRoot"], root.to_str().unwrap());

        assert_eq!(current_target(&root), root.join("versions/v0.3.0"));
        assert_eq!(
            fs::read(root.join("current/bin/family-chat-cli")).unwrap(),
            b"#!/bin/sh\necho new\n"
        );
        assert!(
            root.join("versions/v0.2.0").exists(),
            "running version kept"
        );
        assert!(!root.join("versions/v0.1.0").exists());
        assert!(!root.join("versions/v0.0.1").exists());

        let record = read_check_record(&root).expect("update-check.json written");
        assert_eq!(record.latest.as_deref(), Some("v0.3.0"));
    }

    #[cfg(unix)]
    #[test]
    fn upgrade_check_reports_without_changing_anything() {
        let temp = tempfile::tempdir().unwrap();
        let install = fake_install(&temp.path().join("root"), "v0.2.0");
        let root = install.root.clone();
        let server = FakeServer::start();
        publish(&server, "v0.3.0", b"new");

        let mut out = Vec::new();
        let code = run_upgrade_with(
            &UpgradeOptions {
                check: true,
                ..Default::default()
            },
            &context(install.clone(), "v0.2.0", &server),
            &mut out,
        )
        .unwrap();
        assert_eq!(code, 0);
        let text = String::from_utf8(out).unwrap();
        assert!(
            text.contains("✓ family-chat-cli v0.3.0 is available (you have v0.2.0)"),
            "{text}"
        );
        assert!(text.contains("  family-chat-cli upgrade"), "{text}");
        assert!(
            !root.join("versions/v0.3.0").exists(),
            "check must not install"
        );
        assert_eq!(current_target(&root), root.join("versions/v0.2.0"));

        let mut out = Vec::new();
        run_upgrade_with(
            &UpgradeOptions {
                check: true,
                json: true,
                ..Default::default()
            },
            &context(install, "v0.2.0", &server),
            &mut out,
        )
        .unwrap();
        let report: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(report["current"], "v0.2.0");
        assert_eq!(report["latest"], "v0.3.0");
        assert_eq!(report["upToDate"], false);
        assert_eq!(report["isDowngrade"], false);
        assert_eq!(report["installed"], serde_json::json!(["v0.2.0"]));
    }

    #[cfg(unix)]
    #[test]
    fn upgrade_recognises_up_to_date_force_and_downgrades() {
        let temp = tempfile::tempdir().unwrap();
        let install = fake_install(&temp.path().join("root"), "v0.3.0");
        let root = install.root.clone();
        let server = FakeServer::start();
        publish(&server, "v0.3.0", b"same");

        let mut out = Vec::new();
        run_upgrade_with(
            &UpgradeOptions::default(),
            &context(install.clone(), "0.3.0", &server),
            &mut out,
        )
        .unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap().trim(),
            "✓ already on v0.3.0 — pass --force to reinstall"
        );

        // --force reinstalls the same version.
        let mut out = Vec::new();
        run_upgrade_with(
            &UpgradeOptions {
                force: true,
                ..Default::default()
            },
            &context(install.clone(), "0.3.0", &server),
            &mut out,
        )
        .unwrap();
        assert_eq!(
            fs::read(root.join("versions/v0.3.0/bin/family-chat-cli")).unwrap(),
            b"same"
        );

        // An explicit older target is a downgrade: warned about, then installed.
        let archive = gz(b"older");
        server.route(
            "/download/family-chat-cli-linux-x64.gz",
            200,
            archive.clone(),
        );
        server.route(
            "/download/checksums.txt",
            200,
            format!("{}  family-chat-cli-linux-x64.gz\n", sha256_hex(&archive)),
        );
        let mut out = Vec::new();
        run_upgrade_with(
            &UpgradeOptions {
                target: Some("0.1.0".into()),
                ..Default::default()
            },
            &context(install, "0.3.0", &server),
            &mut out,
        )
        .unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(
            text.contains("! v0.1.0 is older than the running v0.3.0"),
            "{text}"
        );
        assert_eq!(current_target(&root), root.join("versions/v0.1.0"));
        // No API lookup happens for an explicit target, so the record still holds what the earlier
        // latest-lookups found rather than v0.1.0.
        assert_eq!(
            read_check_record(&root).unwrap().latest.as_deref(),
            Some("v0.3.0")
        );
    }

    #[cfg(unix)]
    #[test]
    fn uninstall_requires_yes_then_removes_the_root_and_owned_links() {
        let temp = tempfile::tempdir().unwrap();
        let install = fake_install(&temp.path().join("root"), "v0.2.0");
        let root = install.root.clone();
        let bin_dir = temp.path().join("local-bin");
        fs::create_dir_all(&bin_dir).unwrap();
        let ours = bin_dir.join("family-chat-cli");
        std::os::unix::fs::symlink(root.join("current/bin/family-chat-cli"), &ours).unwrap();
        // A family-chat-cli.exe link pointing elsewhere must be left alone.
        let foreign_target = temp.path().join("elsewhere");
        fs::write(&foreign_target, b"x").unwrap();
        let foreign = bin_dir.join("family-chat-cli.exe");
        std::os::unix::fs::symlink(&foreign_target, &foreign).unwrap();

        // The bin dir is discovered through this variable; set it for the duration of the test.
        // SAFETY: tests in this binary that read FAMILY_CHAT_CLI_BIN_DIR all run through this
        // function, and cargo runs test functions on separate threads only within one process;
        // the variable is scoped to this test's lifetime.
        unsafe { std::env::set_var("FAMILY_CHAT_CLI_BIN_DIR", &bin_dir) };

        let mut out = Vec::new();
        let code = run_uninstall_with(&UninstallOptions::default(), &install, &mut out).unwrap();
        assert_eq!(code, 1);
        let text = String::from_utf8(out).unwrap();
        assert!(
            text.contains(&format!("! this will remove {}", root.display())),
            "{text}"
        );
        assert!(
            text.contains(&format!("and the symlink {}", ours.display())),
            "{text}"
        );
        assert!(text.contains("Re-run with --yes to proceed."), "{text}");
        assert!(root.exists(), "nothing removed without --yes");

        let mut out = Vec::new();
        let code = run_uninstall_with(
            &UninstallOptions {
                yes: true,
                json: true,
            },
            &install,
            &mut out,
        )
        .unwrap();
        assert_eq!(code, 0);
        let report: serde_json::Value = serde_json::from_slice(&out).unwrap();
        let removed = report["removed"].as_array().unwrap();
        assert_eq!(removed[0], root.to_str().unwrap());
        assert_eq!(removed[1], ours.to_str().unwrap());
        assert!(!root.exists());
        assert!(fs::symlink_metadata(&ours).is_err(), "our link removed");
        assert!(fs::symlink_metadata(&foreign).is_ok(), "foreign link kept");

        unsafe { std::env::remove_var("FAMILY_CHAT_CLI_BIN_DIR") };
    }
}
