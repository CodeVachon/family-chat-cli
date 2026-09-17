---
id: 56
title: Publish stable API v1 client contract and fixtures
state: Done
parent: 4
labels: [task, api, backend, external, contract, mvp]
created: 2026-08-26T01:33:08Z
updated: 2026-09-17T17:54:11Z
---

## Description

Make family-chat PR #81 consumable by an independently versioned Rust client without deriving public schemas from TypeScript database query internals.

## Plan

Document or generate schemas and authenticated fixtures for GET /me, /channels, /channels/:id, /members, paginated /messages, POST /messages, errors, 204 responses, and SSE events. Specify camelCase/date/null encoding, the beforeId plus beforeCreatedAt cursor, sanitized HTML bodies, decorated history versus send response shapes, event compatibility, and API versioning. Add integration/contract tests covering an authenticated approved user and permissions.

## Notes

### 2026-09-17T17:54:10Z — Christopher Vachon (user)

Moved to family-chat's own Motte tracker (now #1 there) — this is backend-repo work (schemas, fixtures, contract tests), not something the CLI can do from here. Marking Done in this tracker just means 'nothing left to track here'; the actual work is untouched and open in family-chat.
