---
id: 16
title: Store and retrieve credentials from OS keychain
state: Done
parent: 3
assignee: Christopher Vachon
labels: [task, auth]
blockedBy: [15, 36, 52]
created: 2026-08-25T21:38:34Z
updated: 2026-09-16T19:56:48Z
---

## Description

Persist the bearer session token outside plaintext config files.

## Plan

Use OS-backed secure credential storage (keychain/Secret Service/Credential Manager) for the bearer session token, namespaced by profile and server URL — no OAuth issuer/client-id/resource namespacing needed since there's no OAuth client. Keep the token in memory for the process lifetime, only touching the keychain on login/logout/startup load. Define behavior for a locked or unavailable keychain (fail closed with a clear error, don't fall back to plaintext). Verify round-trip, namespace isolation between profiles, deletion on logout, and unavailable-backend behavior.

## Notes

### 2026-09-16T19:56:48Z — Christopher Vachon (user)

Implemented in src/auth/store.rs: CredentialStore trait, KeyringStore (OS keychain via the keyring crate — needed non-default Cargo features per platform backend, apple-native/windows-native/sync-secret-service/crypto-rust, corrected after #10's original decision omitted them), InMemoryStore test double. Verified: unit test for InMemoryStore's round trip, plus a #[ignore]'d keyring_store_round_trips_on_this_machine test that exercises the REAL OS keychain (gnome-keyring/Secret Service on this dev machine) — save/load/clear all confirmed working when run explicitly.
