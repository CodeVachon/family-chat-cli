---
id: 52
title: Define authentication and credential security policy
state: Done
parent: 3
labels: [task, auth, security]
blockedBy: [14, 34]
created: 2026-08-25T21:45:45Z
updated: 2026-09-17T17:42:07Z
---

## Description

Define the security invariants for a public native CLI before persisting or forwarding credentials.

## Plan

Require HTTPS outside explicit local development; validate issuer, audience/resource, and scopes; never log tokens or device codes at unsafe verbosity; bind credentials to profile and API origin; document keychain-unavailable behavior; and prohibit silently falling back to plaintext token storage.

## Notes

### 2026-09-17T17:42:06Z — Christopher Vachon (user)

Completed. Audited each item in the plan against what the code actually does (not aspirationally) and wrote the full policy up in docs/security-policy.md. Two real gaps found and fixed; the rest were either already true or genuinely not applicable to this app's auth model.

HTTPS required outside local development — real gap, fixed. Neither the config file's server nor a --server/FAMILY_CHAT_URL value enforced this before; a plain http:// URL to a real host would have sent the bearer token (and the sign-in password, during login) in plaintext. Added config::require_https_outside_local_dev (allows localhost/loopback IPs over http, requires https everywhere else), wired into both Config::validate (the config file's own value) and main (the fully resolved server — CLI flags and env vars bypass Config::validate entirely, so they needed their own check on the same final value).

Issuer/audience/scope validation — not applicable, documented why. Those are OAuth/OIDC concepts; per #3's own decision this app uses email+password bearer auth against Better Auth with no token format it parses at all, so there's nothing in that shape to validate. The closest real equivalent is credential-to-origin binding, which is #36's job.

Bind credentials to profile + origin — already true (#36), verified and cited rather than re-implemented.

Never log tokens/passwords at unsafe verbosity — audited every log call near auth/credential code (none pass a secret to tracing; verified live across a full session — zero matches for the token or "Authorization" in the log file). Found a real transitive risk in the process: hyper's own instrumentation can dump the Authorization header at debug/trace, and since this app's own log level is user-controlled via RUST_LOG, a blanket RUST_LOG=debug meant to get more detail from THIS app would have silently turned that on too. Fixed with main::capped_directives, which caps hyper/reqwest/h2/rustls at warn unless the RUST_LOG string already names one of them explicitly.

Keychain-unavailable behavior — documented (KeyringStore::new runs before the terminal enters raw mode, so a failure is a normal readable error, never a corrupted display) and added real .context() so the error is actionable instead of a bare keyring-crate message.

No silent plaintext fallback — verified true by construction (only KeyringStore is ever wired into main; InMemoryStore is test-only, confirmed via grep) and written down explicitly as a decision to preserve, not just an accident of the current structure.

Verified live: a plain http:// URL to a real host is now a clear startup error ("only https is allowed, except for local development"); http://localhost still passes the policy check (fails differently, for the expected reason — nothing listening there); normal https production use against the real account is unaffected; the log file shows zero token/Authorization leakage across a full session.

7 new tests. 109 tests passing, clippy/fmt clean.
