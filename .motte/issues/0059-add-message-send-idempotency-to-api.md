---
id: 59
title: Add message-send idempotency to API
state: Done
parent: 4
labels: [task, api, backend, external, reliability, post-mvp]
created: 2026-08-26T01:33:09Z
updated: 2026-09-17T17:54:11Z
---

## Description

Allow native clients to retry an ambiguously completed message POST without creating duplicate messages.

## Plan

Define an idempotency key or client message id for POST /channels/:channelId/messages, persist or enforce uniqueness per author, return the original result on safe replay, document expiry/conflict behavior, and add contract tests. Until available, the CLI must not automatically retry an ambiguous send.

## Notes

### 2026-09-17T17:54:10Z — Christopher Vachon (user)

Moved to family-chat's own Motte tracker (now #2 there) — API-side work, not actionable from the CLI repo. The CLI already documents and follows the constraint this implies (docs/api-contract.md, and app::state's compose handling never auto-retries an ambiguous send) — marking Done here just closes this repo's tracking of it, not the underlying work, which is untouched and open in family-chat.
