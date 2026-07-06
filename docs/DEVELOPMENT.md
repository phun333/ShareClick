# Development guide

## Prerequisites

- [Rust](https://rustup.rs) stable (1.80+).
- macOS: Xcode command-line tools. Windows: MSVC build tools.
- To run `serve`/`connect` live you need OS permission (see below).

## Everyday commands

```bash
# Fast, permissionless: builds the testable core only.
cargo build --no-default-features
cargo test  --no-default-features        # unit tests, headless-safe

# Full native build (input capture/injection, clipboard, mDNS).
cargo build --release

# With the GUI menu-bar / tray app.
cargo build --release --features tray

# The headline metric: input-channel latency.
cargo run --release -- bench --count 20000            # plaintext
cargo run --release -- bench --count 20000 --encrypted
```

`cargo test` runs 23 unit tests across `protocol` (wire + crypto) and `app`
(bulk framing, file transfer, config, edge, cursor). None require a display or
special permissions — keep it that way so CI stays green.

## OS permissions (for live testing)

- **macOS:** System Settings → Privacy & Security →
  - **Accessibility** (rdev capture + enigo injection), and
  - **Input Monitoring** (rdev).
  Add your terminal, or the installed `ShareClick.app`. Permissions are tied to
  the *bundle identity*, so a packaged `.app` remembers the grant; a bare binary
  run from Terminal ties the grant to Terminal.
- **Windows:** allow `shareclick.exe` through the firewall on first run
  (port 24800, TCP + UDP).

## Running a real two-machine test

```bash
# Machine A (has the keyboard/mouse):
shareclick init-config      # edit psk + [[machines]] layout
shareclick serve

# Machine B (same psk, name = its own machine):
shareclick init-config      # set name, server_host or rely on mDNS
shareclick connect          # or: shareclick connect <host>
```

Move the cursor into a bordered edge to hand off; F12 toggles manually.

## Project conventions

- **Keep the hot path allocation-light and lock-free.** `transport.rs` uses a
  dedicated blocking socket and immutable cipher state precisely to avoid
  scheduler jitter and locks per packet. Don't add a `Mutex` to the send/recv
  path.
- **Native code goes behind `#[cfg(feature = "native")]`.** Pure logic
  (protocol, framing, geometry) stays testable without it.
- **Every new wire message gets a round-trip test.** Every geometry/state helper
  (`edge.rs`, `cursor.rs`) gets unit tests — they encode the UX rules.
- **Latency is a tracked metric.** Before/after a change on the input path, run
  `bench --encrypted` and confirm no regression. The autoresearch log
  (`autoresearch.jsonl`) records the history.

## Adding a feature (checklist)

1. Decide: pure logic (testable) vs. native. Prefer pushing logic into a pure,
   tested module.
2. If it adds a wire message → update [PROTOCOL.md](./PROTOCOL.md) + add a test +
   consider `PROTOCOL_VERSION`.
3. Wire it into `run.rs` (`serve`/`connect`) and, if user-facing, the tray menu.
4. Add a config field in `config.rs` if it needs settings (with a `#[serde(
   default)]` so old configs still load).
5. Update [ARCHITECTURE.md module map](./ARCHITECTURE.md#module-map) and
   [CHANGELOG.md](../CHANGELOG.md).
6. Run `cargo test` and `bench --encrypted`.

## Repository layout

```
crates/protocol/   # wire types + crypto (no OS deps)
crates/app/        # binary: transport, features, CLI, tray
packaging/macos/   # build-app.sh → ShareClick.app + .dmg
packaging/windows/ # shareclick.iss → Inno Setup installer
.github/workflows/ # CI + release automation
docs/              # this documentation
autoresearch.*     # experiment log + backlog (latency history)
```
