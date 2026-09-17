---
id: 4
title: "Epic: REST API client and domain models"
state: Done
parent: 1
labels: [epic, api]
created: 2026-08-25T21:37:48Z
updated: 2026-09-17T17:54:17Z
---

## Description

Build the typed client layer used by the TUI for channels, users, messages, and current identity.

## Plan

Define typed request and response models, centralize auth headers and error mapping, add pagination support, and expose simple async methods for channels, users, messages, posting messages, and current user lookup.

## Notes

### 2026-09-17T17:54:17Z — Christopher Vachon (user)

All child issues (19/20/21/22/23/56/57/59) are Done — the last two (56/59) by being moved to family-chat's own Motte tracker rather than actually implemented here, since they're backend-repo work.
