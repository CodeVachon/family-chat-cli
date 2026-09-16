---
id: 19
title: Define chat API domain types
state: Done
parent: 4
assignee: Christopher Vachon
labels: [task, api]
blockedBy: [48]
created: 2026-08-25T21:38:34Z
updated: 2026-09-16T19:56:32Z
---

## Description

Model channels, users, messages, pagination, and current identity.

## Plan

Create serde-compatible structs and enums that match the REST API contract while keeping UI-specific state separate.

## Notes

### 2026-09-16T19:56:31Z — Christopher Vachon (user)

Implemented in src/api/types.rs: User, SignInResponse, MeResponse, Channel, ChannelsResponse, Message (+ author/preferences), MessagesResponse — only the fields this prototype actually reads, matching docs/api-contract.md; serde ignores whatever else the server sends. Verified against realistic payloads (real field names/casing, nested author.preferences, chrono timestamp parsing) via wiremock-backed tests in src/api/client.rs, not just hand-picked test data.
