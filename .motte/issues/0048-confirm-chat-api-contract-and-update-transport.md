---
id: 48
title: Confirm chat API contract and update transport
state: Done
parent: 4
assignee: Christopher Vachon
labels: [task, api, planning, auth]
created: 2026-08-25T21:45:43Z
updated: 2026-09-16T19:57:59Z
---

## Description

Inventory the deployed chat API contract before binding Rust domain types or refresh behavior to assumptions.

## Plan

Record the OpenAPI/schema or representative fixtures for identity, channels, users/presence, messages, pagination, error envelopes, rate limits, and send idempotency. Confirm whether new data arrives through polling, long polling, SSE, WebSocket, or manual refresh, and record the MVP choice.

## Notes

### 2026-09-16T18:52:09Z — Christopher Vachon (user)

Inventoried the deployed contract from ../family-chat source (commit 98b9c38) since /api/v1 isn't live on https://chat.thevachonfamily.ca yet (404s — needs a release cut + redeploy; /api/health and /api/auth are live). Full writeup: docs/api-contract.md. Key findings: transport is SSE (GET /api/v1/stream, Postgres LISTEN/NOTIFY fan-out, event types enumerated); messages paginate by keyset cursor (beforeId+beforeCreatedAt, page size 50), images by offset (page size 60); errors are {error:{message}} with 422+issues for validation; rate limits are 20 msgs/60s/user and 1 typing-ping/2.5s/(user,channel); no send idempotency key exists server-side, CLI must dedupe client-side. Biggest finding: there is NO OAuth device-authorization plugin on the server (only bearer + magicLink + passkey) — epic #3 and tasks #14/#15/#17 assumed a device grant that doesn't exist. Recommend bearer-token flow instead (capture set-auth-token header from POST /api/auth/sign-in/email). Flagged as an open decision for Chris; not resolved without his input.

### 2026-09-16T19:10:28Z — Christopher Vachon (user)

Deployment gap resolved: re-probed 2026-09-16 and /api/v1 is live — GET /health returns 200 {"ok":true}, GET /settings returns real app settings, GET /me (unauthenticated) returns 401 with the documented {error:{message}} envelope instead of the earlier 404 HTML. Contract matches source exactly. docs/api-contract.md updated.

### 2026-09-16T19:57:59Z — Christopher Vachon (user)

Merged from #0054 — “Validate CLI plan against family-chat PR 81”.

What it said:

Audit the CLI/TUI Motte backlog against the concrete REST, Better Auth bearer-session, and SSE implementation in CodeVachon/family-chat PR #81.

Its plan:

Inspect the exact PR head, map MVP workflows to routes and response shapes, identify backend prerequisites and contract gaps, update affected Motte tasks and dependencies, run Motte integrity checks, and record evidence.
