# Build history — what was built, why, and how

This is the narrative record of how ShareClick came to be, written so anyone
(human or AI) resuming the project understands the intent behind each stage. The
machine-readable version — every experiment, its metric, and its rationale — is
in `autoresearch.jsonl` at the repo root; this file is the human summary.

The project was built by an AI agent in an autonomous loop that tracks input
latency as its primary metric (`shareclick bench`). Every change was compiled,
unit-tested where verifiable, and checked for latency regression.

## Phase 0 — Research (before any code)

Surveyed the whole "software KVM" space (ShareMouse, Synergy, Deskflow, Input
Leap, Barrier, Lan Mouse, Universal Control). Findings that shaped the design:
- Lan Mouse proved a UDP input path + active/inactive model gives the lowest
  latency and stability.
- Deskflow/Synergy showed portable key IDs and the value of clipboard + file
  transfer (the paid tools' real differentiator is polished file drag-and-drop).
- "Jumpiness" is a known industry bug when mouse polling rate exceeds display
  refresh → mitigate with per-tick coalescing.

Decision: build in Rust, take the best of each, and make latency the headline.

## Phase 1 — Working MVP

Goal: a real KVM (not a mirror) with input + clipboard + files.

1. **Baseline transport** (`transport.rs`, `bench.rs`): UDP `InputChannel` with
   postcard framing, sequence numbers, dedicated blocking socket. Measured
   **~6 µs** one-way on loopback → proved the transport is not the bottleneck and
   established the benchmark we defend forever after.
2. **Input layer** (`capture.rs`, `emit.rs`, `keymap.rs`): rdev capture → enigo
   injection, with the portable `Key` enum bridging macOS/Windows keycodes.
3. **Bulk channel + clipboard** (`bulk.rs`, `clipboard.rs`): length-prefixed TCP
   frames; bidirectional text sync with echo suppression.
4. **File transfer** (`filexfer.rs`): chunked, offset-based, path-traversal-safe;
   verified byte-for-byte over loopback.
5. **Real control handoff** (`capture.rs` → rdev `grab`): switched from passive
   `listen` to `grab` so local input is *swallowed* while the client has control
   — the difference between a mirror and a true KVM. F12 toggles.

Result: a functional MVP; all verifiable paths unit-tested; latency unchanged.

## Phase 2 — Quality: encryption, settings, seamless switching, GUI

1. **Encryption** (`crypto.rs`): X25519 + PSK-authenticated handshake +
   ChaCha20-Poly1305 on **both** channels. The key experiment measured the
   latency cost: **~20 ns** (6.46 → 6.48 µs) — "very high quality encryption,
   basically free." See [SECURITY.md](./SECURITY.md).
2. **Settings + monitor manager** (`config.rs`): TOML config with the machine/
   edge-neighbour layout, PSK, port. `init-config` writes a starter file.
3. **Automatic edge-switching** (`edge.rs`): the server hands off when the cursor
   hits a bordered screen edge — no hotkey needed.
4. **Menu-bar / tray UI** (`tray.rs`, `tray` feature): macOS status item (top-
   right, no dock icon) and Windows system tray via `tray-icon` + `tao`.

## Phase 3 — Polish

1. **Client-side auto-return** (`cursor.rs`, `control.rs`): the client integrates
   relative motion and returns control when the cursor crosses back over the
   border — closing the round-trip so F12 is never required.
2. **Zero-arg client** (`config.rs` `server_host`): `connect` with no argument
   reads the server from config; the tray "Start Client" works.
3. **Clipboard images** (`clipboard.rs`): raw RGBA via arboard, unified
   fingerprint echo-suppression for text + images.
4. **mDNS discovery** (`discovery.rs`): peers find each other without typing IPs;
   the PSK still authenticates, so it stays secure.

## Phase 4 — Distribution (make it user-friendly)

The realization that end users won't install Rust drove packaging:
- No-argument launch opens the tray (double-click UX).
- `packaging/macos/build-app.sh`: universal (arm64 + Intel) menu-bar `.app` +
  `.dmg`.
- `packaging/windows/shareclick.iss`: one-click Inno Setup installer.
- `.github/workflows/release.yml`: on every `vX.Y.Z` tag, CI builds and publishes
  both installers to a GitHub Release. See [RELEASING.md](./RELEASING.md).

## Invariants to preserve going forward

- The `bench --encrypted` number is the north star — don't regress it.
- The testable core stays free of OS/display dependencies (`native`/`tray`
  feature split).
- Wire changes are versioned and documented in [PROTOCOL.md](./PROTOCOL.md).
- Every decision's reasoning lives in [DECISIONS.md](./DECISIONS.md).
