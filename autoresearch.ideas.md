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

## DONE (polish pass)
- [x] Client-side return edge (CursorTracker + Enter/Leave signalling; auto reclaim both directions)
- [x] Config `server_host` + zero-arg client + tray Start Client wired
- [x] Clipboard images (raw RGBA via arboard; unified fingerprint echo-suppression)
- [x] mDNS discovery (advertise + browse + `discover` CLI; PSK still authenticates)

## Next up (future / optional)
- **serve multi-client / reconnect:** currently one session at a time; make the UDP cipher swappable (arc-swap) so capture can keep running across reconnects and fan out to multiple clients.
- **Progress + backpressure** for large file transfers; zero-copy sendfile later.
- **Packaging:** macOS `.app` bundle (so Accessibility grant sticks) + Windows `.msi`; auto-launch at login.
- **GUI settings editor** in the tray instead of opening the TOML by hand.
- **Real end-to-end lag measurement** vs ShareMouse/Deskflow (photodiode / high-speed camera) to substantiate the "lowest lag" claim.

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

## Known issue: macOS local-cursor suppression (server)
- rdev `grab` on macOS does NOT suppress mouse-move even when the callback returns None, so while the client has control the Mac cursor also moves (mirrors instead of switching). Keyboard/clicks may suppress; mouse-move does not.
- Tried warp-to-anchor + CGAssociateMouseAndMouseCursorPosition(true): broke forwarding (likely the ~250ms post-warp event-suppression deadzone). Reverted.
- Proper fix: replace rdev mouse capture on macOS with a custom CGEventTap that (a) reads raw kCGMouseEventDeltaX/Y (relative deltas, valid even when the cursor is frozen), and (b) uses CGAssociateMouseAndMouseCursorPosition(false) to freeze the cursor + CGDisplayHideCursor to hide it. This is the standard remote-desktop/game capture technique. Keyboard can stay on rdev.
- Windows: rdev grab returning 1 (None) DOES suppress there, so Windows-as-server should switch correctly already.

## Deferred feature: monitor offset (realistic edge crossing, part 3)
Parts 1+2 done (client warps to entry on Enter; bidirectional Leave{entry} with
proportional fraction; tested). Part 3 = arbitrary vertical/horizontal OFFSET so
the cursor crosses at the exact placed position (not just proportional). Design:
- **Config:** add `offset: i32` (default 0) = the OTHER screen's top (left for
  vertical adjacency) relative to THIS screen's top, in this screen's pixels.
- **Server is the authority** (holds offset + the client's recorded size). Keep
  the client dumb (it only sends/receives local pixel coords):
  - Enter{edge, pos:i32}: server computes client-local entry
    `pos = clamp(exit_local - offset, 0, client_dim)`; client warps there.
  - Leave{pos:i32}: client sends its local exit pixel; server enters at
    `clamp(pos + offset, 0, server_dim)`.
  - This means switching Enter/Leave payload from fraction (f32) to pixel (i32)
    — a wire change; bump PROTOCOL_VERSION.
- **GUI:** allow free vertical/horizontal drag (not just centre-snap); store the
  resulting offset in config. Currently the arrangement centre-snaps, so there is
  no offset to honour yet — GUI offset placement is a prerequisite.
- Needs BOTH binaries updated + a 2-machine live test.
