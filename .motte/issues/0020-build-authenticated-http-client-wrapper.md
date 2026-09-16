---
id: 20
title: Build authenticated HTTP client wrapper
state: Done
parent: 4
assignee: Christopher Vachon
labels: [task, api]
blockedBy: [11, 19]
created: 2026-08-25T21:38:34Z
updated: 2026-09-16T19:56:33Z
---

## Description

Centralize base URL handling, auth headers, JSON parsing, timeouts, and error conversion.

## Plan

Wrap Reqwest behind a ChatApi interface with canonical base URL handling, bounded connect/request timeouts, typed JSON parsing, structured errors, and an injected asynchronous TokenProvider. Keep concrete Better Auth login out of this layer so API work and tests can use a fake token provider. Verify headers, URLs, timeouts, response parsing, and token-provider failures with a local mock server.

## Notes

### 2026-09-16T19:56:32Z — Christopher Vachon (user)

Implemented in src/api/client.rs: ApiClient (base URL + bearer token via Arc<RwLock<Option<String>>>, cheap to Clone), sign_in_email, sign_out, me, list_channels, channel_messages. Error mapping in src/api/error.rs distinguishes /api/v1's {error:{message}} envelope from Better Auth's own {message,code} shape, and from_auth_response vs from_response (see #17/#23 notes — a 401 during sign-in means bad credentials and should show the server's own message, not the generic session-expired one). Verified with 4 wiremock integration tests (token/user parsing, error-message passthrough, bearer-header attachment, nested payload parsing) plus live manual testing against https://chat.thevachonfamily.ca (see #15).
