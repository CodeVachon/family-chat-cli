---
id: 29
title: Load initial channels and current user
state: Done
parent: 6
assignee: Christopher Vachon
labels: [task, chat]
blockedBy: [15, 21, 24, 27]
created: 2026-08-25T21:38:35Z
updated: 2026-09-16T19:57:01Z
---

## Description

Populate the TUI after startup/authentication.

## Plan

Fetch current identity and channels, select a sensible default channel, and show errors without exiting unnecessarily.

## Notes

### 2026-09-16T19:57:01Z — Christopher Vachon (user)

Implemented: on successful resume or login, Command::LoadChannels fires GET /api/v1/channels; on_channels_loaded stores them and auto-selects/loads the first channel's messages. Verified via wiremock (authenticated_calls_send_the_bearer_token test) and the app::state unit tests for the selection/auto-load logic.
