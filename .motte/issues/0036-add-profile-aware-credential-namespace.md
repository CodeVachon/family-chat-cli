---
id: 36
title: Add profile-aware credential namespace
state: Todo
parent: 7
labels: [task, config]
blockedBy: [34]
created: 2026-08-25T21:38:35Z
updated: 2026-08-25T21:45:09Z
---

## Description

Prevent credentials for one API environment from leaking into another.

## Plan

Key credential entries by profile and API origin, and keep token storage separate from config.
