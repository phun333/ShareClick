# ShareClick

**A low-latency, open-source software KVM.** Move one keyboard & mouse — plus
the clipboard and files — between your macOS and Windows machines over the LAN,
with the lowest input lag we can squeeze out.

Think ShareMouse / Synergy / Deskflow, but free, open, and built for latency
first.

> Status: **early MVP.** Transport + native input capture/injection work and are
> benchmarked. Clipboard, file transfer, and seamless edge-switching are on the
> roadmap below.

---

## Why another one?

We surveyed everything in this space before writing a line of code:

| Tool | License | Mouse/KB | Clipboard | Files | Notes |
|------|---------|:--------:|:---------:|:-----:|-------|
| **ShareMouse** | 💰 Paid | ✅ | ✅ | ✅ | Nice UX, but licensed |
| **Synergy** | 💰 Paid | ✅ | ✅ | ✅ | Built on Deskflow |
| **Deskflow** | 🟢 GPLv2 | ✅ | ✅ | ⚠️ | Most mature free option |
| **Input Leap** | 🟢 GPLv2 | ✅ | ✅ | ⚠️ | Living Barrier fork |
| **Barrier** | 🟢 | ✅ | ✅ | ❌ | ⛔ Unmaintained |
| **Lan Mouse** | 🟢 | ✅ | ❌ | ❌ | Fastest/cleanest, but input-only |
| **ShareClick** | 🟢 MIT/Apache | ✅ | 🛠️ | 🛠️ | Latency-first, taking the best of all |

**What we took from each:**

- **Lan Mouse** → UDP input path + active/inactive state model → lowest latency.
- **Deskflow/Synergy** → portable key IDs, clipboard & file transfer features.
- **ShareMouse** → the UX goal: drag-and-drop files should "just work".

## Architecture

Two logical channels, each optimized for its job:

```
        macOS (server)                         Windows (client)
   ┌───────────────────────┐   UDP input    ┌───────────────────────┐
   │ rdev capture          │ ─────────────▶ │ enigo injection       │
   │  → portable Key/mouse  │  (tiny, seq-#, │  ← portable Key/mouse  │
   │  → coalesced per tick  │   dedup'd)     │                       │
   ├───────────────────────┤  TCP bulk      ├───────────────────────┤
   │ clipboard / files      │ ◀────────────▶ │ clipboard / files      │
   └───────────────────────┘  (reliable)    └───────────────────────┘
```

- **Input channel (UDP):** dedicated blocking socket (no async scheduler
  jitter), postcard-encoded packets, monotonic sequence numbers so late/dup
  packets are dropped instead of blocking. Events are **coalesced per poll
  tick** to avoid the classic "jumpiness" when mouse polling rate exceeds the
  display refresh rate.
- **Bulk channel (reliable):** clipboard sync and chunked file transfer, where
  ordering matters more than microseconds.
- **Portable keys:** macOS and Windows use different raw keycodes, so we
  translate native keys into a portable `Key` enum on capture and back on
  injection (same idea as Synergy's key IDs).

### Measured latency

Transport overhead is negligible — the OS event path and LAN dominate:

```
$ shareclick bench --count 20000
METRIC rtt_median_us=~12     # loopback round-trip
METRIC oneway_us=~6          # one-way transport overhead
```

~6 µs one-way transport overhead means our code is not the bottleneck; real
input lag will be LAN RTT (~0.2–1 ms) + OS injection. We keep the bench in the
repo so latency regressions get caught immediately.

## Build & run

Requires [Rust](https://rustup.rs). Native input needs OS permission:
**macOS** → System Settings ▸ Privacy & Security ▸ Accessibility (add your
terminal / the binary). **Windows** → run once, allow through the firewall.

```bash
# Build
cargo build --release

# Benchmark the input transport (no permissions needed)
./target/release/shareclick bench --count 20000

# On the machine whose keyboard & mouse you want to share:
./target/release/shareclick serve --bind 0.0.0.0:24800
#   → press F12 to hand control to the client; F12 again to reclaim it.
#     While the client has control, local input on the server is suppressed.

# On the other machine:
./target/release/shareclick connect 192.168.1.20:24800

# Send a file to a peer (clipboard syncs automatically once connected):
./target/release/shareclick send-file 192.168.1.20:24800 ./report.pdf
#   → lands in ./received on the peer.
```

### Settings, encryption & the monitor manager

```bash
# Create an editable config (settings + monitor-manager layout):
./target/release/shareclick init-config
#   → set a strong `psk` (identical on both machines — it authenticates the
#     peers and derives the ChaCha20-Poly1305 session keys) and describe which
#     machine borders which screen edge. serve/connect then require this file.
```

The **monitor manager** is the `[[machines]]` layout: each machine lists its
screen size and which peer sits on each edge. When `auto_edge_switch` is on,
pushing the cursor into a bordered edge hands control to that neighbour.

### Menu-bar / tray app

```bash
# Build with the GUI front-end and launch it:
cargo build --release --features tray
./target/release/shareclick tray
```

* **macOS:** a status item appears in the top-right menu bar (no dock icon).
* **Windows:** an icon appears in the system tray.

The menu exposes Start Server / Start Client, **Settings & Monitor Manager…**
(opens `config.toml`), and Quit.

Build the portable core without native deps (for CI / headless):

```bash
cargo build --release -p shareclick --no-default-features
```

## Roadmap

- [x] Hybrid UDP/reliable transport with sequence numbers
- [x] Latency benchmark harness
- [x] Native input capture (rdev) + injection (enigo)
- [x] Portable cross-platform key mapping
- [x] Control handoff hotkey (F12) + local input suppression (`rdev` grab)
- [x] Clipboard sync (text) over the bulk channel
- [x] File transfer (`send-file`, chunked, resumable-by-offset)
- [x] Encryption: X25519 + PSK handshake + ChaCha20-Poly1305 on both channels
      (measured ~20 ns extra latency — see the encrypted benchmark)
- [x] Settings + monitor manager (`config.toml`)
- [x] Automatic edge-switching (cursor crosses a bordered screen edge)
- [x] Menu-bar (macOS) / system-tray (Windows) app (`--features tray`)
- [ ] Client-side return-edge detection (auto reclaim without F12)
- [ ] Clipboard images
- [ ] mDNS auto-discovery
- [ ] Multi-monitor / multi-client layouts
- [ ] Auto-discovery (mDNS) so you don't type IPs
- [ ] Tray app / GUI

See [`autoresearch.ideas.md`](./autoresearch.ideas.md) for deeper technical
notes and optimization ideas.

## License

Dual-licensed under MIT or Apache-2.0.
