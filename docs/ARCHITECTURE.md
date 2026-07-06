# Architecture

## Goals (in priority order)

1. **Lowest possible input lag.** This is the headline feature; every design
   choice defers to it.
2. **Cross-platform:** macOS ⇄ Windows (Linux best-effort).
3. **Secure by default:** authenticated encryption on every channel.
4. **Open source & self-hostable:** no accounts, no cloud, LAN-only.
5. **Feature-complete vs. paid tools:** clipboard + file transfer + seamless
   edge switching.

## The big picture

```
        SERVER (has the physical keyboard & mouse)      CLIENT (receives input)
   ┌──────────────────────────────────────┐        ┌──────────────────────────────┐
   │ capture (rdev grab)                    │        │ emit (enigo)                 │
   │   captures + can SWALLOW local input   │        │   injects received events    │
   │        │                               │        │        ▲                     │
   │        ▼  coalesced per tick           │  UDP   │        │                     │
   │   InputChannel ───────────────────────────────► │   InputChannel              │
   │   (encrypted, seq-numbered)            │ input  │   (decrypts, applies)        │
   │                                        │        │   cursor.rs tracks position  │
   │   Control (active? entry edge)         │◄────── │   → sends Leave to return    │
   ├────────────────────────────────────────┤  TCP   ├──────────────────────────────┤
   │ clipboard.rs (arboard) ─┐              │  bulk  │  ┌─ clipboard.rs (arboard)   │
   │ filexfer.rs (chunks)  ──┼─ BulkConn ───────────────┼─ filexfer.rs (received/)  │
   │                         └ (encrypted)  │        │  └                           │
   └────────────────────────────────────────┘        └──────────────────────────────┘
                 ▲ mDNS advertise                             ▲ mDNS discover
```

## Why two channels?

Input and bulk data have opposite requirements:

| | Input (mouse/keys) | Bulk (clipboard/files) |
|---|---|---|
| Needs | lowest latency | reliable, ordered delivery |
| Tolerates loss? | yes (next move supersedes) | no |
| Transport | **UDP** | **TCP** |
| Why | no head-of-line blocking; a dropped move is irrelevant a millisecond later | a lost clipboard byte corrupts the paste |

Mixing them would force the fast path to wait for retransmits of the slow path.
Keeping them separate is the single most important latency decision — see
[DECISIONS.md #1](./DECISIONS.md).

## Crate layout

```
crates/
  protocol/   # shared, dependency-light: wire types + crypto. No OS calls.
  app/        # the binary: transport, capture/emit, features, CLI, tray.
```

`protocol` is deliberately tiny and platform-agnostic so it can be unit-tested
anywhere and reused (e.g. a future mobile client). `app` holds everything that
touches the OS or the network.

## Module map (`crates/app/src`)

| Module | Responsibility | Native? |
|---|---|---|
| `main.rs` | CLI parsing (clap); dispatch; no-arg → tray | no |
| `transport.rs` | UDP `InputChannel`: framing, seq numbers, optional encryption | no |
| `bench.rs` | Loopback latency benchmark (the primary metric) | no |
| `bulk.rs` | TCP `BulkConn`: length-prefixed frames, handshake, encryption | no |
| `filexfer.rs` | Chunked file send + reassembly (`received/`) | no |
| `config.rs` | TOML settings + **monitor manager** (machine/edge layout) | no |
| `edge.rs` | Server-side screen-edge hit detection | no |
| `cursor.rs` | Client-side cursor integration for auto-return | no |
| `control.rs` | Shared control state (active? entry edge) | no |
| `capture.rs` | `rdev` global grab: capture + suppress local input | **yes** |
| `emit.rs` | `enigo` input injection | **yes** |
| `keymap.rs` | rdev ⇄ portable `Key` ⇄ enigo translation | **yes** |
| `clipboard.rs` | Bidirectional clipboard sync (text + image) | **yes** |
| `discovery.rs` | mDNS advertise/browse | **yes** |
| `run.rs` | Wires everything into `serve` / `connect` loops | **yes** |
| `tray.rs` | Menu-bar / system-tray GUI (`tray` feature) | **yes** (+`tray`) |

"Native?" means it needs OS input/clipboard APIs. Non-native modules build and
test with `--no-default-features`, which is what CI uses for fast, permissionless
unit tests.

## Feature flags

| Flag | Pulls in | Purpose |
|---|---|---|
| `native` (default) | enigo, rdev, arboard, mdns-sd | real input/clipboard/discovery |
| `tray` | native + tray-icon, tao | the GUI menu-bar/tray app |

The layering exists so the **testable core** (transport, protocol, crypto,
config, edge, cursor, file framing) never depends on a display or OS
permissions. That is why 23 unit tests run in CI headlessly.

## Control flow: how a keypress reaches the other machine

1. `capture.rs` (rdev grab) sees the key. If control is on the client, it maps
   the native key to a portable `Key`, forwards it, and **swallows** it locally.
2. `run.rs::run_server_input` coalesces the tick's events into one
   `InputMsg::Events`, and `transport.rs` seals + sends it over UDP.
3. On the client, `transport.rs` decrypts, drops stragglers by sequence number,
   and hands the batch to `run.rs::connect`.
4. `emit.rs` injects each event with enigo. `cursor.rs` integrates mouse deltas;
   if the cursor crosses back over the border edge it sends `InputMsg::Leave`.

## Control handoff (the "KVM" part)

Two ways control moves from server to client:

- **Automatic edge switch:** `edge.rs` detects the server cursor hitting a
  bordered screen edge (from the monitor-manager layout) → `Control.active =
  true` and the entry position is recorded.
- **F12 hotkey:** manual toggle, always available as a fallback.

Return is symmetric: the client's `cursor.rs` detects the cursor crossing back
and sends `Leave`; the server clears `active`. See
[DECISIONS.md #7](./DECISIONS.md).

## Where the latency actually goes

Measured on loopback with `shareclick bench --encrypted`:

- Transport (serialize + seal + socket + open + deserialize): **~6.5 µs one-way**
- Encryption adds **~20 ns** (ChaCha20-Poly1305 on tiny packets).

So on a real LAN, end-to-end input lag is dominated by the network RTT
(~0.2–1 ms) and OS event injection — not our code. This is the whole point of
the UDP + dedicated-blocking-socket + per-tick-coalescing design.
