---
id: 59
title: Add message-send idempotency to API
state: Todo
parent: 4
labels: [task, api, backend, external, reliability, post-mvp]
created: 2026-08-26T01:33:09Z
updated: 2026-08-26T01:33:09Z
---

## Description

Allow native clients to retry an ambiguously completed message POST without creating duplicate messages.

## Plan

Define an idempotency key or client message id for POST /channels/:channelId/messages, persist or enforce uniqueness per author, return the original result on safe replay, document expiry/conflict behavior, and add contract tests. Until available, the CLI must not automatically retry an ambiguous send.
