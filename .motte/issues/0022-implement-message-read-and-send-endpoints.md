---
id: 22
title: Implement message read and send endpoints
state: Done
parent: 4
assignee: Christopher Vachon
labels: [task, api]
blockedBy: [20]
created: 2026-08-25T21:38:34Z
updated: 2026-09-16T20:18:33Z
---

## Description

Expose typed methods for loading history and posting new messages.

## Plan

Add paginated message loading, newest-message refresh, and send_message with server response handling.

## Notes

### 2026-09-16T20:18:33Z — Christopher Vachon (user)

Implemented in src/api/client.rs: channel_messages (read, initial page only — see #30's note on pagination) and send_message (POST /channels/:channelId/messages with just {body}, relying on the server's Zod defaults for threadRootId/attachments/mentionUserIds). Verified with wiremock tests for both the success path and a 422 validation error surfacing the server's message.
