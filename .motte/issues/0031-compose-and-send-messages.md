---
id: 31
title: Compose and send messages
state: Todo
parent: 6
labels: [task, chat]
blockedBy: [22, 30]
created: 2026-08-25T21:38:35Z
updated: 2026-08-25T21:47:01Z
---

## Description

Allow the user to write and post messages from the TUI.

## Plan

Use the compose behavior and key bindings from task 49. Submit a client request id when the API supports idempotency, keep the draft until success, prevent accidental duplicate submissions, insert or reconcile the server response in stable order, and surface validation, auth, rate-limit, and network failures without losing text. Verify success, retry, duplicate-submit, and failure paths.
