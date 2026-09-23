#!/bin/sh
# Smoke-test an installed family-chat-cli binary.
#
#   scripts/smoke.sh <binary> <expected-version>
#
# Run by the release workflow's `verify` job against the binary that install.sh just installed
# from the freshly built assets, so a broken build is caught before anything is published. It also
# runs locally against `target/release/family-chat-cli`, where the check that needs a managed
# install (upgrade) is skipped.
#
# The interactive TUI itself needs a real account against a real server, so it isn't exercised
# here — this checks what can be checked without one: the binary starts, reports its version, and
# (when installed and pointed at a fake API) resolves an upgrade correctly.
#
# Environment honoured:
#   FAMILY_CHAT_CLI_API_BASE   when set, `upgrade --check` (latest lookup) is exercised against it
#
# POSIX sh on purpose; no jq, no bash-isms.

set -eu

BIN="${1:?usage: smoke.sh <binary> <expected-version>}"
VERSION="${2:?usage: smoke.sh <binary> <expected-version>}"
VERSION="${VERSION#v}"

pass() { printf '\342\234\223 %s\n' "$1"; }
fail() { printf '\342\234\227 %s\n' "$1" >&2; exit 1; }

resolve() {
    # readlink -f is GNU coreutils and macOS 12.3+; realpath covers the rest.
    readlink -f "$1" 2>/dev/null || realpath "$1"
}

# --- 1. version ---------------------------------------------------------------

actual="$("$BIN" --version)"
[ "$actual" = "family-chat-cli $VERSION" ] ||
    fail "--version printed '$actual', expected 'family-chat-cli $VERSION'"
pass "--version reports $VERSION"

# --- 2. help --------------------------------------------------------------------

"$BIN" --help >/dev/null || fail "--help exited non-zero"
"$BIN" upgrade --help >/dev/null || fail "upgrade --help exited non-zero"
pass "--help exits 0"

# --- 3. config (no network, no keyring, safe to exercise here) -----------------

cfg_tmp="$(mktemp -d)"
trap 'rm -rf "$cfg_tmp"' EXIT INT TERM
export XDG_CONFIG_HOME="$cfg_tmp"

"$BIN" config set server https://smoke.example.com >/dev/null || fail "config set exited non-zero"
got="$("$BIN" config get server)"
[ "$got" = "https://smoke.example.com" ] || fail "config get server returned '$got'"
pass "config set/get round-trips"

# --- 4. upgrade, only meaningful from a managed install ------------------------

case "$(resolve "$BIN")" in
    */versions/*/bin/*)
        check="$("$BIN" upgrade "v$VERSION" --check --json)"
        case "$check" in
            *'"upToDate": true'* | *'"upToDate":true'*) pass "upgrade v$VERSION --check reports up to date" ;;
            *) fail "upgrade --check --json did not report upToDate true: $check" ;;
        esac
        if [ -n "${FAMILY_CHAT_CLI_API_BASE:-}" ]; then
            latest="$("$BIN" upgrade --check --json)"
            case "$latest" in
                *'"upToDate": true'* | *'"upToDate":true'*) pass "upgrade --check (latest lookup via $FAMILY_CHAT_CLI_API_BASE) reports up to date" ;;
                *) fail "upgrade --check via FAMILY_CHAT_CLI_API_BASE did not report upToDate true: $latest" ;;
            esac
        fi
        ;;
    *)
        printf '! skipping upgrade checks: %s is not inside a managed install (versions/<v>/bin)\n' "$BIN" >&2
        ;;
esac

pass "smoke test complete"
