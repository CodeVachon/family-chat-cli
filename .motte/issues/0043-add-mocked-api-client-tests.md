---
id: 43
title: Add mocked API client tests
state: Done
parent: 9
assignee: Christopher Vachon
labels: [task, quality]
blockedBy: [20]
created: 2026-08-25T21:38:35Z
updated: 2026-09-16T23:44:09Z
---

## Description

Verify HTTP request construction and response/error handling.

## Plan

Use an HTTP mock server or trait-backed fake client to test auth headers, pagination, send_message, and error mapping.

## Notes

### 2026-09-16T23:44:08Z — Christopher Vachon (user)

Covered via wiremock in src/api/client.rs: bearer-token auth headers, sign-in success/error-message passthrough, send_message success and 422 validation-error mapping, nested-payload/timestamp parsing. Not covered: pagination — the client doesn't implement it yet (see #30's open note), so there's nothing to test there until that's built.
