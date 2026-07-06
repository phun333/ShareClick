# ShareClick — ideas & optimization backlog

## Research summary (competitors)
- **Deskflow** = official OSS successor to Synergy v1.15; most mature free option; clipboard ✅, TLS by default. File drag&drop only partial.
- **Input Leap** = living fork of Barrier; ~feature parity with Deskflow.
- **Barrier** = dead, migrate off.
- **Lan Mouse (Rust)** = highest refresh rate + most stable, but input-only (no clipboard/files). Best low-latency reference architecture.
- **ShareMouse/Synergy** = paid; their real differentiator vs free tools is polished file drag&drop.
- **Apple Universal Control** = Mac/iPad only, no Windows.
- Known industry bug: "jumpiness" when mouse polling rate > display refresh rate → mitigate by coalescing events per tick (done).

## Architecture decisions (locked)
- Hybrid transport: UDP for input (latency), reliable channel for clipboard/files.
- Dedicated blocking sockets on the hot path (no async scheduler jitter).
- Portable `Key` enum (Synergy-style) — never forward raw OS keycodes.
- Per-tick event coalescing.

## DONE (Phase 1 + 2)
- [x] Local input suppression + F12 handoff (rdev unstable_grab)
- [x] Clipboard sync with echo suppression (arboard)
- [x] File transfer (chunked, offset writes, path-traversal safe)
- [x] Encryption: ephemeral X25519 + PSK (HKDF salt) + ChaCha20-Poly1305 on BOTH channels; per-channel + per-direction keys; ~20ns measured cost
- [x] Settings + monitor-manager config (TOML)
- [x] Automatic edge-switching (server->client) via neighbour graph
- [x] macOS menu-bar / Windows tray UI (tray-icon + tao, `tray` feature)

## Next up (polish)
- **Client-side return edge:** client tracks injected cursor position from relative deltas + its screen size; when it hits the return edge, send InputMsg::Leave so the server flips control back automatically (today F12 reclaims). Enter/Leave already exist in the protocol.
- **Tray Start Client:** needs a `server_host` setting; wire the menu action to actually dial it.
- **Clipboard images:** ClipboardData::Image already in protocol; add arboard image get/set.
- **mDNS discovery:** advertise `_shareclick._udp` so peers find each other without typing IPs.
- **serve multi-client / reconnect:** currently one session at a time; make the UDP cipher swappable (arc-swap) so capture can keep running across reconnects.
- **Progress + backpressure** for large file transfers; zero-copy sendfile later.

## Latency optimization ideas (measure with `shareclick bench`)
- Try QUIC for the bulk channel only (keep UDP raw for input) — encryption + reliability without HOL-blocking input.
- Busy-poll vs sleep on the server send loop: current 500µs sleep adds up to 0.5ms; consider adaptive spin when a burst is in flight.
- `SO_BUSY_POLL` / real-time thread priority for the capture + send threads.
- Pack multiple relative-move deltas by summing them within a tick (already coalesced as a Vec; could sum consecutive MouseMove into one to shrink packets further).
- Consider `recvmmsg`/`sendmmsg` batching if we ever fan out to multiple clients.
- Measure real end-to-end input lag (photodiode / high-speed camera) vs ShareMouse/Deskflow to prove the "lowest lag" claim — add a repeatable methodology doc.

## Cross-platform gaps to close
- enigo `Insert` key missing on macOS → currently dropped; revisit with raw keycodes.
- Windows keymap: verify rdev `Key` coverage for non-US layouts; may need scancode-based path.
- Wayland: enigo/rdev experimental; document X11-first for Linux like the rest of the ecosystem.
