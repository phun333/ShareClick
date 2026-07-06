# Decision log (ADR)

Lightweight architecture decision records. Newest supersedes oldest — never
delete an entry; if a decision changes, add a new one that references it. This
is the "why" that keeps future changes from re-litigating settled questions.

Format: **status** · decision · rationale · consequences.

---

### 1. Hybrid transport: UDP for input, TCP for bulk
**Status:** accepted. **Rationale:** input tolerates loss and demands lowest
latency; a dropped mouse move is irrelevant a millisecond later, so UDP avoids
head-of-line blocking. Clipboard/files need reliable ordered delivery → TCP.
**Consequences:** two code paths and two sets of keys, but the fast path never
waits on the slow path. This is the core latency decision.

### 2. Rust
**Status:** accepted. **Rationale:** predictable low-latency (no GC pauses),
single static binary, easy cross-compilation, strong cross-platform input crates
(rdev/enigo/arboard). **Consequences:** contributors need Rust; end users get a
tiny dependency-free binary.

### 3. Dedicated blocking UDP socket, not async
**Status:** accepted. **Rationale:** an async runtime adds scheduler jitter on
the hot path. A dedicated thread with a blocking socket has the most predictable
latency. **Consequences:** `transport.rs` is intentionally synchronous; measured
~6 µs one-way overhead confirms it.

### 4. Relative mouse motion, not absolute
**Status:** accepted. **Rationale:** absolute coordinates couple the two
machines' screen geometries and break on differing resolutions/DPI. Relative
deltas "just work" and are tiny on the wire. **Consequences:** the client must
integrate deltas to know its cursor position (`cursor.rs`); capture converts
rdev's absolute positions to deltas.

### 5. Portable `Key` enum, never raw scancodes
**Status:** accepted. **Rationale:** macOS and Windows use different raw
keycodes; forwarding them raw would mistranslate keys. We map native → portable
`Key` → native (the Synergy/Deskflow approach). **Consequences:** `keymap.rs`
maintains the translation tables; unmapped keys degrade to `Unknown` and are
dropped.

### 6. Per-tick event coalescing
**Status:** accepted. **Rationale:** high mouse polling rates (1000+ Hz) would
flood the network and cause "jumpiness" when polling exceeds display refresh.
Coalescing a tick's events into one packet fixes both. **Consequences:**
`InputMsg::Events(Vec<..>)` carries a batch.

### 7. Control model: server-authoritative, Enter/Leave signalling
**Status:** accepted. **Rationale:** exactly one machine must own input at a time
to avoid feedback loops. The server holds the truth (`Control.active`), hands off
on edge/F12, and signals the client with `Enter`; the client auto-returns by
sending `Leave` when its cursor crosses back. **Consequences:** `rdev` grab must
*swallow* local input while the client is active — hence the `unstable_grab`
feature rather than passive `listen`.

### 8. WireGuard-style crypto: X25519 + PSK + ChaCha20-Poly1305
**Status:** accepted. **Rationale:** ephemeral ECDH gives forward secrecy; a PSK
mixed into the HKDF salt authenticates peers without a PKI (MITM-resistant);
ChaCha is fast in software so encryption costs ~20 ns per input packet. See
[SECURITY.md](./SECURITY.md). **Consequences:** users must share a passphrase;
no certificate infrastructure needed.

### 9. Nonce counters are implicit where possible
**Status:** accepted. **Rationale:** transmitting a nonce per packet wastes
bytes. TCP is ordered → both peers keep in-sync counters (transmit nothing). UDP
reuses the packet `seq` as the counter (already present). **Consequences:** never
reuse a `(key, nonce)` pair; per-direction keys guarantee this.

### 10. mDNS for discovery, PSK for identity
**Status:** accepted. **Rationale:** typing IPs is bad UX; mDNS finds peers
automatically. Security is unaffected because discovery only yields a candidate
address — the PSK handshake still authenticates. **Consequences:** an imposter
can advertise the service but cannot complete the handshake.

### 11. `native` / `tray` feature split
**Status:** accepted. **Rationale:** keep a testable, permissionless core that
CI can build and unit-test headlessly; isolate heavy GUI + OS deps behind
opt-in flags. **Consequences:** 23 unit tests run without a display; the GUI is
built only for releases.

### 12. No-argument launch opens the tray
**Status:** accepted. **Rationale:** a double-clicked `.app`/`.exe` passes no
CLI args; end users expect a GUI, not a help dump. **Consequences:** `main.rs`
routes no-subcommand → `tray::run()` (or help if built without `tray`).

### 13. Ship one-click installers, not source
**Status:** accepted. **Rationale:** end users won't install Rust. CI builds a
universal macOS `.dmg` and a Windows `.exe` installer on every tag and publishes
them to GitHub Releases. **Consequences:** see [RELEASING.md](./RELEASING.md);
unsigned until certificates are acquired.

---

## Known open questions / future decisions
- Sliding-window anti-replay for UDP (currently monotonic-counter only).
- Multiple simultaneous clients (needs swappable per-session UDP cipher).
- Code signing / notarization once certificates are available.
