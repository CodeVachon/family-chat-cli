# Credential and authentication security policy (as of 2026-09-17)

Covers Motte #52. This app is a native CLI holding a long-lived bearer
session token — this document states the invariants that follow from that,
checked against what the code actually does (not aspirational), and grounded
in the auth decision already recorded on #3 (email+password bearer, no
OAuth — see `docs/architecture.md`).

## HTTPS is required outside local development

**Enforced.** A bearer token (and, during sign-in, the account password
itself) must never go out over a plaintext connection to a real host.
`config::require_https_outside_local_dev` rejects any server URL whose
scheme isn't `https`, unless the host is unambiguously local development
(`localhost`, or a loopback IP — `127.0.0.0/8` / `::1`). This runs in two
places, since a server URL can come from two independent paths that don't
share validation otherwise:

- `config::Config::validate` — the config file's own `server` field.
- `main`'s startup sequence — the *fully resolved* server (config file,
  `--server`, or `FAMILY_CHAT_URL`, whichever won), since a CLI flag or env
  var never passes through `Config::validate` at all.

A plain `http://` URL to a real host is a startup error, not a silent
downgrade. `http://localhost:...` (a local dev server) is allowed.

## Issuer, audience/resource, and scope validation

**Not applicable to this app's auth model.** Those are OAuth/OIDC concepts;
this app authenticates with email+password against Better Auth's own
`/api/auth/sign-in/email` and receives an opaque bearer token scoped to
exactly one server (see `docs/api-contract.md`) — there's no issuer,
audience, or scope claims to validate because there's no token *format* this
client parses at all, by design (#3's decision to skip OAuth entirely for a
single-server personal tool). The closest equivalent invariant — that a
token is never presented to, or reused against, a server other than the one
that issued it — is what "bind credentials to profile and API origin"
(below) actually enforces.

## Credentials are bound to profile *and* API origin

**Enforced** (#36). `auth::store::credential_namespace` combines the
profile and the server's origin (`scheme://host[:port]`) into one keyring
entry identity. Switching `--server`/`FAMILY_CHAT_URL`/the config file's
`server` under the same profile — e.g. pointing at a staging server — can
never silently reuse, or overwrite, a session token that belongs to a
different server; it gets its own keyring entry instead.

## Tokens and passwords are never logged at unsafe verbosity

**Enforced**, on two levels:

- **This app's own code** never passes a token, password, or session cookie
  to a `tracing::*!` call — audited directly (`grep` for every log call
  near auth/credential code): `auth::store` logs the *error* from a failed
  keychain operation (a platform-level failure description, e.g. "no
  backend available"), never the secret itself; `auth::login::resume` logs
  why a stored session was dropped (the server's error message), never the
  token that was rejected. Verified live: a full session (login, channel
  browsing, sending) produces zero matches for the bearer token or the
  string "Authorization" anywhere in the log file.
- **Transitive risk from `reqwest`/`hyper`:** hyper's own instrumentation
  can log full request/response headers — including `Authorization`, so the
  raw bearer token — at `debug`/`trace` verbosity. Since this app's log
  level is user-controlled via `RUST_LOG`, a blanket `RUST_LOG=debug` or
  `=trace` (meant to get more detail from *this app's* logging) would
  otherwise also crank up hyper's. `main::capped_directives` caps
  `hyper`/`reqwest`/`h2`/`rustls` at `warn` whenever the `RUST_LOG` string
  doesn't already name one of them explicitly — so the default experience
  is safe, while `RUST_LOG=hyper=trace` still works when the HTTP layer
  itself is what's actually being debugged.

## Keychain-unavailable behavior

**Documented, not silently swallowed.** `auth::KeyringStore::new` is called
before the terminal enters raw mode/the alternate screen, so a keychain
failure (no Secret Service running on Linux, an unavailable Keychain/
Credential Manager, etc.) surfaces as a normal, readable startup error —
never a corrupted terminal — with context pointing at what's missing and
why (see `main`'s `.context(...)` on that call). There is deliberately no
fallback path here: see the next section.

## No silent fallback to plaintext token storage

**Prohibited, by construction.** The only `CredentialStore` implementation
wired into `main` is `auth::KeyringStore` (the OS keychain/Secret
Service/Credential Manager). `auth::InMemoryStore` exists solely for unit
tests (never `main`) and is not persistent — nothing in this codebase
writes a session token to a plain file, and there is no code path that
falls back to one if the keychain is unavailable; a keychain failure is a
startup error (see above) rather than a silent downgrade to something less
secure. This is a decision to preserve deliberately, not just an accident of
how the code happens to be structured today — if a future change ever adds
a second `CredentialStore` implementation, it must not be reachable from
`main` unless it meets the same bar.
