---
id: 7
title: "Epic: Configuration, storage, and environments"
state: Todo
parent: 1
labels: [epic, config]
created: 2026-08-25T21:37:48Z
updated: 2026-08-25T21:37:48Z
---

## Description

Handle API base URLs, profiles/environments, local configuration, credential storage, and safe defaults.

## Plan

Use a small config file for non-secret settings and OS keychain for tokens. Support explicit API base URL configuration, profile selection if needed, and useful diagnostics for missing or invalid config.
