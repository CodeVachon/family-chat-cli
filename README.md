# family-chat-cli

A terminal client for the family's self-hosted chat server — login, channels,
real-time messages, and rich-text rendering, all from a single Rust binary.
Personal tool, not published anywhere (see `docs/architecture.md`).

## Building

Requires a stable Rust toolchain (install via [rustup](https://rustup.rs) if
you don't have one).

```sh
cargo build --release   # or: make release
```

The binary lands at `target/release/family-chat-cli`. A debug build
(`cargo build`, or `make build`) is faster to compile and fine for regular
use — the release profile trades build time for a smaller, faster binary
(LTO, single codegen unit, stripped symbols; see `Cargo.toml`), which mostly
matters if you're copying the binary somewhere else to run it.

## Installing

There's no installer — copy the built binary wherever you keep personal
tools on your `PATH`, e.g.:

```sh
cp target/release/family-chat-cli ~/.local/bin/
```

or build directly into a `PATH` directory with `cargo install`:

```sh
cargo install --path . --root ~/.local
```

## Running

```sh
family-chat-cli
```

The server URL defaults to the family's production instance. Point it
elsewhere with `--server <url>`, the `FAMILY_CHAT_URL` environment variable,
or a config file's `server` field (`src/config/mod.rs`) — in that order of
precedence. A concise login/everyday-use walkthrough isn't written up yet
(tracked as Motte #39).

Logs go to `$XDG_STATE_HOME/family-chat-cli/` (or `~/.local/state/...`) —
never the terminal itself, since the TUI owns that.

## Configuring

Non-secret settings (`server`, `profile`, `default-channel`) live in a TOML
file, editable without opening it by hand:

```sh
family-chat-cli config get                  # every value
family-chat-cli config get server           # just one
family-chat-cli config set server https://chat.example.com
family-chat-cli config unset profile
family-chat-cli config path                 # where the file lives
```

`set`/`unset` validate before saving — an invalid value (e.g. a non-`https`
URL against a real host) is rejected and never written to disk.

## Developing

```sh
make preflight   # fmt-check + clippy + test — what CI runs
```

Other `Makefile` targets: `fmt`, `clippy`, `test`, `build`, `run`.
`docs/architecture.md`, `docs/api-contract.md`, and `docs/tui-interaction.md`
cover how the pieces fit together.
