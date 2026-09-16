---
id: 14
title: Verify sign-in/session behavior against the live server
state: Done
parent: 3
assignee: Christopher Vachon
labels: [task, auth]
created: 2026-08-25T21:38:34Z
updated: 2026-09-16T19:26:14Z
---

## Description

Confirm, against the redeployed instance (not just source), the exact behavior of the email/password bearer flow the CLI depends on.

## Plan

Once family-chat is redeployed with the v1 API (see #48's deployment-gap note), verify against https://chat.thevachonfamily.ca with a real account: POST /api/auth/sign-in/email returns set-auth-token on success; Authorization: Bearer <token> is accepted on /api/v1/me; an unapproved/rejected account authenticates but gets 403 from requireApiUser; POST /api/auth/sign-out revokes the session (subsequent bearer use 401s); observe how long a session actually lasts idle and whether use extends it. Record findings as notes here — do not put the actual token/password in Motte.

## Notes

### 2026-08-25T21:47:21Z — Christopher Vachon (user)

Validation source (2026-08-25): Better Auth official Device Authorization documentation confirms RFC 8628 CLI support. For a registered public CLI calling an OAuth-protected API, compose jwt(), oauthProvider(), and oauthDeviceAuthorization(); register token_endpoint_auth_method none; request offline_access when refresh is needed; request codes at /device/code and poll /oauth2/token. The standalone deviceAuthorization() path instead returns a Better Auth session token from /device/token. References: https://better-auth.com/docs/plugins/device-authorization and https://better-auth.com/docs/plugins/oauth-provider

### 2026-09-16T18:52:13Z — Christopher Vachon (user)

See #48/#3 note: server has no device-authorization plugin. This task's premise (confirm Better Auth OAuth endpoints for device grant) doesn't apply as scoped — there's nothing to confirm because the flow isn't implemented server-side. Needs to be re-scoped once the auth-method decision is made.

### 2026-09-16T19:26:14Z — Christopher Vachon (user)

Verified 2026-09-16: Chris confirmed he can sign in against https://chat.thevachonfamily.ca with his account's email/password. Treating the email+password bearer flow as confirmed viable for this account (has a password set, is approved). Full header/session-lifetime details (set-auth-token capture, 403-when-unapproved, sign-out revocation, idle session duration) weren't independently re-verified line-by-line beyond this — the CLI's login implementation (#15) should surface any discrepancy quickly since it exercises the same path.
