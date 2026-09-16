# Architecture decision (as of 2026-09-16)

Covers Motte #10: Rust toolchain, crate layout, dependency set, feature
flags, target platforms, and test seams. Input: `docs/api-contract.md` (#48)
and the auth decision recorded on #3 (email+password bearer, no OAuth).

## Toolchain

Stable Rust, **edition 2024**. No published-MSRV policy — this is a personal
tool (per Chris: "not something we'll be distributing"), so "whatever stable
`rustc`/`cargo` is on the dev machine" is the floor. Dev machine currently has
1.98.1.

## One binary crate, not a workspace

A Cargo workspace buys nothing here — there's one deliverable, one team of
one, no shared library published elsewhere. A workspace would add
indirection (multiple `Cargo.toml`s, inter-crate path deps) for zero benefit
at this scale. Decision: **single crate**, `src/main.rs` as a thin entry
point over library-shaped modules underneath `src/`, so unit tests can still
target modules directly without needing a separate lib crate:

```
src/
  main.rs           # clap parsing → dispatch to `run()`, tracing init, terminal setup/teardown
  cli.rs            # clap arg/subcommand definitions
  config/           # file format + XDG-ish paths (owned by #34/#35, not decided here)
  auth/
    mod.rs
    login.rs        # email/password sign-in flow (#15)
    store.rs        # CredentialStore trait + keyring impl + in-memory test impl (#16)
  api/
    mod.rs
    client.rs       # reqwest-based ApiClient: base url + bearer token + JSON helpers
    types.rs        # domain types mirroring docs/api-contract.md (#19)
    stream.rs        # SSE client wrapper (reqwest-eventsource), maps to app events
    error.rs         # ApiError mirroring the server's {error:{message}} / 422 issues shape
  app/
    mod.rs
    state.rs        # pure state + reducer-style transitions (#12), no IO — easy to unit test
    event.rs        # the unified event enum (key input, SSE event, API response, tick)
  tui/
    mod.rs
    layout.rs
    widgets/        # channel list, message pane, composer, status/auth screens
  text/
    html.rs         # Tiptap HTML body → styled ratatui Text (uses `tl`)
```

`api/` and `auth/` are IO-boundary modules with trait seams (see below);
`app/state.rs` is pure and holds no IO, which is what makes #42 (unit tests
for state transitions) tractable without mocking anything.

## Dependency set

Verified by building a scratch crate with this exact set (`cargo check` +
`cargo tree -d`) rather than picking versions from memory:

| Purpose | Crate | Notes |
|---|---|---|
| Async runtime | `tokio` (`rt-multi-thread, macros, time, sync, io-util`) | The one and only executor. |
| Terminal backend | `crossterm` (`event-stream`) | `event-stream` makes `crossterm::event::EventStream` a `futures::Stream`, so key input merges into the same tokio-driven event loop as SSE/HTTP — no second executor, no polling thread. |
| TUI widgets | `ratatui` | Version compatible with the pinned `crossterm`. |
| HTTP client | `reqwest` (`json, rustls-tls, stream`, default-features off) | `rustls-tls` avoids an OpenSSL system-dependency for cross-compilation later (#41 is post-MVP but no reason to make it harder now). `stream` is needed for the SSE body. |
| SSE client | `reqwest-eventsource` | Wraps `reqwest`, handles reconnect/backoff and the `retry:` directive the server sends — matches `docs/api-contract.md`'s stream section instead of hand-rolling reconnect logic. |
| Serialization | `serde` (`derive`), `serde_json` | |
| CLI parsing | `clap` (`derive`) | `login`/`logout`/default-launches-TUI subcommands. |
| Credential storage | `keyring` | Cross-platform (macOS Keychain / Windows Credential Manager / Linux Secret Service) — see #16. |
| Timestamps | `chrono` (`serde`) | Server sends `timestamptz`/ISO 8601; `chrono::DateTime<Utc>` round-trips cleanly through serde. |
| Errors | `thiserror` (library-shaped errors in `api`/`auth`), `anyhow` (glue at `main`) | Standard split — typed errors where callers branch on them, `anyhow` where they're just surfaced. |
| Logging | `tracing`, `tracing-subscriber` (`env-filter`), `tracing-appender` | Must log to a **file**, never stdout/stderr — those are the TUI's terminal. |
| Rich-text rendering | `tl` | Message bodies are sanitized Tiptap HTML with a small, known tag vocabulary (`p`, `strong`, `em`, `a`, `br`, lists, mentions). `tl` is a minimal zero-copy-ish HTML tokenizer; deliberately **not** `scraper` (pulls in the full `html5ever`/`selectors`/`cssparser`/`string_cache`/`phf` CSS-selector stack for a job that needs no CSS selectors at all — confirmed by trying both: swapping `scraper` for `tl` measurably shrank the dependency tree with no loss of capability for this use case). |
| Async combinators | `futures-util` | For merging the crossterm event stream with SSE/timer streams in the main loop. |

### Dependency-tree verification

`cargo tree -d` on the full set above shows only build-time/proc-macro
duplication, no conflicting runtime stacks:

- `thiserror` v1 (pulled by `reqwest-eventsource`) alongside our own `thiserror` v2 —
  unavoidable (a transitive dep pins v1), harmless (derive-macro only, no
  runtime object crosses the boundary).
- `syn` v2 vs v3, `unicode-width` v0.1 vs v0.2 (the latter entirely inside
  `ratatui`'s own dependency graph, not something this crate's choices
  affect) — both build-time-only proc-macro/codegen duplication.

No duplicate TLS stack (rustls only, via `reqwest`'s `rustls-tls`), no
duplicate HTTP stack (hyper via `reqwest`/`tokio` only), no duplicate async
runtime (tokio only, `crossterm`'s `event-stream` rides on it rather than
bringing its own).

## Feature flags

None needed for MVP. Cross-platform keychain differences are handled by
`keyring`'s own `cfg(target_os)` gating internally, not by exposing Cargo
features on this crate. Revisit only if a genuine build-time optional
capability shows up post-MVP (e.g. #41's packaging work).

## Target platforms

Primary target: Linux x86_64 (the dev machine). macOS/Windows should build
(every chosen dependency — `keyring`, `crossterm`, `reqwest`/rustls — is
cross-platform) but are **not actively tested**; that's explicitly #41's
post-MVP scope, not this decision's problem to solve.

## Test seams

- **`api::client::ApiClient`**: constructed with a base URL + token, all HTTP
  calls go through it. Tests point it at a local mock server (`wiremock`, dev-
  dependency) instead of `https://chat.thevachonfamily.ca`.
- **`api::stream`**: same idea — the SSE URL is just another endpoint on the
  same base, so the same mock server covers reconnect/heartbeat/event-parsing
  tests (a raw `text/event-stream` body is easy to hand `wiremock` verbatim).
- **`auth::store::CredentialStore`** trait, with a `KeyringStore` (real) and an
  in-memory `FakeStore` (tests) impl — so #16/#43/#44's tests never touch the
  actual OS keychain.
- **`app::state`**: pure functions/reducers, zero IO, directly unit-testable
  (#42) without any of the above.

## Verified

`cargo check` and `cargo tree -d` run clean against this exact dependency set
in a scratch crate on 2026-09-16 (Rust 1.98.1). Reviewed against every MVP
capability in #1's plan: auth (#3 family), channels/messages/reactions/threads
(`reqwest` + `serde` + `api::types`), realtime (`reqwest-eventsource`),
credential storage (`keyring`), TUI (`ratatui`/`crossterm`), CLI entry
(`clap`) — nothing in the MVP scope is unaccounted for.
