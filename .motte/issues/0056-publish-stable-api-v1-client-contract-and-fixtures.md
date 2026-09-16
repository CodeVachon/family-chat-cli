---
id: 56
title: Publish stable API v1 client contract and fixtures
state: Todo
parent: 4
labels: [task, api, backend, external, contract, mvp]
created: 2026-08-26T01:33:08Z
updated: 2026-08-26T01:33:08Z
---

## Description

Make family-chat PR #81 consumable by an independently versioned Rust client without deriving public schemas from TypeScript database query internals.

## Plan

Document or generate schemas and authenticated fixtures for GET /me, /channels, /channels/:id, /members, paginated /messages, POST /messages, errors, 204 responses, and SSE events. Specify camelCase/date/null encoding, the beforeId plus beforeCreatedAt cursor, sanitized HTML bodies, decorated history versus send response shapes, event compatibility, and API versioning. Add integration/contract tests covering an authenticated approved user and permissions.
