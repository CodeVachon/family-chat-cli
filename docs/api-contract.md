# Chat API contract (as inventoried 2026-09-16)

Source of truth: `../family-chat` at commit `98b9c38` ("Add Hono REST API for
external clients (#81)"), specifically `apps/web/lib/api/v1.ts`,
`apps/web/lib/auth.ts`, `apps/web/lib/realtime/{stream,broker}.ts`,
`apps/web/lib/security/rate-limit.ts`, `packages/db/src/schema/*`, and that
repo's own `docs/rest-api.md`. Deployed instance: `https://chat.thevachonfamily.ca`.

## Deployment status

`/api/v1` was confirmed **live** on 2026-09-16 (re-verified after initially
finding it 404ing — see git history of this file for that earlier state):

```
GET https://chat.thevachonfamily.ca/api/v1/health   → 200 {"ok":true}
GET https://chat.thevachonfamily.ca/api/v1/settings → 200 (real app settings JSON)
GET https://chat.thevachonfamily.ca/api/v1/me       → 401 {"error":{"message":"Authentication required"}} (correct envelope)
```

Matches the source-derived contract below exactly. Deploys are cut via GitHub
Release → `release-docker.yml` → GHCR image, not auto-deployed on merge to
`main` — worth remembering if a future contract change in `family-chat` lands
in source but isn't live yet.

## Identity / auth

Better Auth (`apps/web/lib/auth.ts`), mounted at `/api/auth/[...all]`. Plugins:
`bearer()`, `magicLink()`, `passkey()`, `nextCookies()`.

**There is no OAuth/device-authorization plugin.** Epic #3 ("Better Auth OAuth
device login") and tasks #14/#15/#17 assumed a device grant that doesn't exist
server-side. **Resolved 2026-09-16**: this is a personal, non-distributed
tool, so a terminal email+password prompt is acceptable — epic #3 and tasks
#14/#15/#16/#17/#18 have been rewritten around the bearer-token flow below.

- Sign-in methods enabled: email+password, magic link (existing accounts only —
  `disableSignUp: true`), passkey (WebAuthn).
- **Bearer token mechanics** (confirmed from `better-auth` plugin source): any
  response that would set a session cookie also echoes the raw session token
  in a `set-auth-token` response header. A native client captures that header
  once (from sign-in, magic-link verify, etc.) and thereafter sends
  `Authorization: Bearer <token>` — Better Auth's bearer middleware converts it
  back into a session internally. There is no separate "refresh token"; a
  session is a sliding-expiry token (Better Auth defaults: ~7-day expiry,
  refreshed on use after ~1 day — not overridden in this app's config, but not
  independently confirmed here either).
- Logout = `POST /api/auth/sign-out` (revokes server-side) + discard the local
  token.
- User record carries app-specific fields: `appRole` (`owner|admin|user`) and
  `approvalStatus` (`pending|approved|rejected`). New sign-ups start
  `pending`; the very first user in the instance is auto-approved as `owner`.
  `requireApiUser` (in `lib/api/auth.ts`) rejects anything not `approved` with
  403 — so a pending/rejected account authenticates fine but gets 403 on every
  `/api/v1` resource until an admin approves it.

### Auth method — decided

**Email + password**, prompted in-terminal: `POST /api/auth/sign-in/email`,
capture `set-auth-token`, send it back as `Authorization: Bearer <token>`.
Chosen over magic-link (would need an untested loopback-redirect trick to get
the token back to the CLI process) and passkey (impractical in a terminal).
Live verification that the account actually has a password set, and that the
sign-in/session behavior matches source, is tracked in #14.

## Error envelope

All `/api/v1/*` errors: `{ "error": { "message": "..." } }`.
Validation failures (Zod) additionally include `"issues"` and use **422**
rather than 400. Auth failures are 401 (no session) or 403 (session exists but
not approved / not authorized for the action). Not-found is 404. Message
posting over budget is 429 with a human-readable retry message (see rate
limits below). Unhandled errors are 500 with a generic message (server logs
the real error; client never sees it).

## Rate limits

In-memory, per-process (not shared across instances — informational only for
client-side backoff, not a hard guarantee):

| Action | Limit | Notes |
|---|---|---|
| `POST /channels/:id/messages` | 20 / 60s per user, across all channels | 429 with `retryAfterMs`-derived message |
| `POST /channels/:id/typing` | 1 / 2.5s per (user, channel) | silently no-ops (204), not an error |
| Better Auth magic-link request | 5 / 60s | enforced inside the plugin, not app code |

No idempotency key exists on `POST /messages` — a retried request after a
dropped response creates a duplicate message. The CLI must de-dupe client-side
(e.g. an optimistic local id it reconciles against the SSE echo) rather than
rely on the server.

## Resources (all under `/api/v1`, all require `Authorization: Bearer <token>`
or the browser session cookie unless noted "public")

- `GET /health` — public, `{ ok: true }`.
- `GET /me` — `{ user, preferences, unread }`.
- `GET /settings` — public, app branding (`name`, `iconUrl`, `defaultChannelIds`).
- `GET /vapid-public-key` — public, web-push key (irrelevant to a terminal client).
- `GET /unread` — `{ total }`.
- `GET /activity` — recent-message previews per visible channel (for a digest/overview pane).
- `GET /stream` — the SSE feed (see below).
- `GET|POST /channels`, `GET /channels/public`, `GET|PATCH|DELETE /channels/:channelId`
- `POST /channels/:id/{join,leave,read,typing}`, `PATCH /channels/:id/{favorite,archive}`
- `GET /channels/:id/members`, `GET /channels/:id/addable-users`
- `POST /channels/:id/members`, `PATCH|DELETE /channels/:id/members/:userId`
- `GET /channels/:id/messages` (paginated, see below), `POST /channels/:id/messages`
- `GET /channels/:id/messages/:messageId/thread`
- `PATCH|DELETE /messages/:messageId`
- `PUT|DELETE /messages/:messageId/reactions/:emoji`
- `GET /channels/:id/images` (gallery, offset-paginated)
- `GET /users/:userId/profile`
- `GET|PATCH /preferences` (+ `/profile`, `/avatar`, `/banner`, `/appearance`, `/notifications` sub-resources)
- `POST|DELETE /push-subscriptions`, `POST /uploads/sign` — web-push/Cloudinary, not relevant to a TUI MVP.
- `GET|POST /admin/users`, `PATCH /admin/users/:userId`, `PATCH /admin/settings` — admin-only, out of scope for MVP.

### Channel shape

```jsonc
{
  "id": "uuid", "name": "string", "description": "string|null",
  "color": "#hex|null", "icon": "lucide-icon-name|null",
  "isPrivate": bool, "isArchived": bool, "archivedAt": "ts|null",
  "createdByUserId": "string", "createdAt": "ts", "updatedAt": "ts"
}
```

Per-channel role (`channelRole` enum): `owner|admin|user|viewer`. Membership
row also carries `isFavorite`, `lastReadMessageId`, `lastReadAt`.

`GET /channels/:id` additionally returns `membership` and a `capabilities`
object (`canPost`, `canManage`, `canManageMembers`) — the CLI should drive
which actions it offers off this rather than reimplementing the permission
matrix.

### Message shape

```jsonc
{
  "id": "uuid", "channelId": "uuid", "authorUserId": "string",
  "type": "user|system", "systemEvent": {...}|null,
  "threadRootId": "uuid|null", "body": "html string",
  "editedAt": "ts|null", "deletedAt": "ts|null",
  "createdAt": "ts", "updatedAt": "ts",
  "author": { "id", "name", "preferences": { "displayName","colorHue","avatarUrl" } },
  "attachments": [...], 
  "reactions": [{ "emoji", "count", "reactedByMe" }],
  "mentions": [{ "userId", "name", "colorHue" }]
}
```

`body` is **sanitized HTML** (Tiptap rich text), not plain text or Markdown —
the CLI will need an HTML→terminal-renderable conversion (strip tags, resolve
mentions, etc.), not just print it raw.

System messages (`type: "system"`) are join/leave/channel_updated
announcements — not editable, deletable, or reactable.

### Pagination

Two different strategies, deliberately:

- **Messages** (`GET /channels/:id/messages`): **keyset/cursor**, newest page
  first. Query params `beforeId` + `beforeCreatedAt` (both required together)
  page backwards. Page size fixed at **50** (`CHANNEL_PAGE_SIZE`). Response:
  `{ messages: [...], hasMore: bool }` — `hasMore` is `messages.length >= 50`,
  i.e. an approximation, not an exact count.
- **Gallery images** (`GET /channels/:id/images`): **offset**-based
  (`?offset=`), page size **60** (`GALLERY_PAGE_SIZE`), ascending/oldest-first.
  Response includes `hasMore` and a `total` count. (Irrelevant to MVP, which
  drops image galleries.)
- **Thread replies** (`GET /channels/:id/messages/:messageId/thread`): no
  pagination at all — returns the whole thread in one response.

### Real-time transport — confirmed: **SSE**, not polling/long-polling/WebSocket

`GET /api/v1/stream`, authenticated the same way as everything else. Backed by
Postgres `LISTEN/NOTIFY` fanned out from a single in-process broker (so it's
per-instance — fine for this deployment's scale).

- Sends an SSE `retry: 3000` directive immediately, then a `ready` event once
  subscribed, then a `heartbeat` comment every 25s (to keep proxies from
  closing the connection).
- Event `type` values: `ready`, `resync`, `message.created`,
  `message.updated`, `message.deleted`, `reaction.changed`, `mention`,
  `read.updated`, `channels.changed`, `users.changed`, `settings.changed`,
  `typing`, `presence`, `presence.snapshot`. Payloads carry whichever of
  `channelId`/`messageId`/`actorId`/`userId`/`onlineUserIds`/etc. are relevant
  — see `apps/web/lib/realtime/broker.ts` for the full union.
- `resync` fires after a broker reconnect to Postgres and means "you may have
  missed events — refetch anything you're tracking," not "reconnect the
  stream." The CLI should treat it as a cue to re-pull unread counts / visible
  channels rather than tearing down the SSE connection.
- Connection caps: 10 concurrent per user, 1000 total server-wide; over-cap
  returns `429 Too many concurrent connections` instead of opening the stream.
- No auto-reconnect is provided by the server beyond the `retry:` hint —
  that's an HTTP-client-library concern on the CLI side (Rust SSE clients
  generally handle `retry:` themselves, but confirm whatever crate is chosen
  does).

MVP transport choice for the CLI: **SSE**, matching the server. No polling
fallback needed unless a corporate/NAT environment blocks long-lived SSE
connections (not a concern for a family LAN/VPN use case).

## Status in Motte

- Done: epic **#3** and tasks **#14/#15/#16/#17/#18** rewritten around the
  bearer-token flow above.
- Ready to proceed: **#19** (define chat API domain types) and **#10**
  (crate/dependency choice) — this document is their input.
- Still open: nothing blocks *cutting a family-chat release and redeploying*
  before any live smoke-testing task (e.g. #45, #46) can actually run — that
  isn't tracked as its own Motte task yet since it's work in the other repo.
