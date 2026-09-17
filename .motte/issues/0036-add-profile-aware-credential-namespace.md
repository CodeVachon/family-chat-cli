---
id: 36
title: Add profile-aware credential namespace
state: Done
parent: 7
labels: [task, config]
blockedBy: [34]
created: 2026-08-25T21:38:35Z
updated: 2026-09-17T15:19:24Z
---

## Description

Prevent credentials for one API environment from leaking into another.

## Plan

Key credential entries by profile and API origin, and keep token storage separate from config.

## Notes

### 2026-09-17T15:19:23Z — Christopher Vachon (user)

Completed. auth::store::KeyringStore::new now takes (profile, origin) instead of just profile, combined via credential_namespace() into one keyring "username" (e.g. "default@https://chat.thevachonfamily.ca"). Previously a stored session token was keyed by profile alone, so switching --server/FAMILY_CHAT_URL/the config file's server under the same profile could silently reuse or overwrite a token belonging to a different server — exactly the leak this ticket asked to prevent.

main.rs now derives the origin from the resolved server URL (parsed with the url crate from #35) and actually reads config.profile (defaulting to "default") rather than hardcoding it — Config::profile's #[allow(dead_code)] came off.

"Keep token storage separate from config" was already true before this ticket and needed no change: tokens live in the OS keyring; config.toml (#34/#35) never stores or touches them.

2 new unit tests for credential_namespace (combines profile+origin; different origins under the same profile get different namespaces so a leak can't happen). 79 tests passing, clippy/fmt clean.

One real-world consequence worth flagging: this changes the keyring entry identity for anyone with an existing session under the old profile-only namespacing (confirmed via secret-tool: the live entry was service=family-chat-cli, username=default). The old entry won't be found under the new namespace, so the app will show the login screen once on the next launch. Nothing is deleted — signing back in creates a fresh entry under the new namespace, and it's a one-time cost.
