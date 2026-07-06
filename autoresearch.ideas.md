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

## Next up (roadmap detail)
- **Edge-switching + local suppression:** server must *grab*/suppress local input while the remote client is active, and release cursor at screen edges. macOS: CGEventTap with tap that can swallow events (rdev `unstable_grab` feature). Windows: low-level hook returning 1 to consume. This is the biggest UX gap right now — currently `serve` mirrors input rather than handing it off.
- **Encryption:** X25519 ECDH handshake → ChaCha20-Poly1305 AEAD per packet. Nonce = seq counter. Handshake over the reliable channel, derive input-channel key from it. Reference: InputSync repo.
- **Clipboard sync:** arboard poll (or native change notifications) → BulkMsg::Clipboard. Debounce to avoid ping-pong loops; tag origin so you don't echo back what you just received.
- **File transfer:** FileBegin/Chunk/End already in protocol. Add backpressure + progress + resume-by-offset. Consider zero-copy sendfile later.

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
