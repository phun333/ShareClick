# Wire protocol

All types live in `crates/protocol/src/lib.rs`. Encoding is
[postcard](https://docs.rs/postcard) — a compact, `serde`-based binary format
chosen because it is small and fast (important on the input hot path).

**Versioning:** `PROTOCOL_VERSION` (currently `1`). Bump it on any breaking wire
change and document the change here. The handshake exchanges versions so peers
can refuse mismatches in the future.

## Two channels

| Channel | Transport | Carries | Type |
|---|---|---|---|
| Input | UDP | mouse/keys, control signals, latency probes | `InputPacket` → `InputMsg` |
| Bulk | TCP | clipboard, files, handshake | `BulkMsg` |

## Input channel

### `InputPacket`
```
struct InputPacket { seq: u32, msg: InputMsg }
```
`seq` is a per-sender monotonic counter. The receiver drops any `Events` packet
whose seq is ≤ the highest seen (duplicate/straggler rejection **without**
blocking — the reason we use UDP). Control/ping messages bypass this check.

### `InputMsg`
```
enum InputMsg {
    Events(Vec<InputEvent>),   // a coalesced tick of input
    Enter { edge: Edge, entry: f32 },  // server → client: you have control
    Leave,                     // either direction: control returns to server
    Ping { nonce: u64, echo_nanos: u64 },
    Pong { nonce: u64, echo_nanos: u64 },
}
```

### `InputEvent`
```
enum InputEvent {
    MouseMove { dx: i32, dy: i32 },      // RELATIVE motion (see DECISIONS #4)
    MouseButton { button: MouseButton, pressed: bool },
    Scroll { dx: f32, dy: f32 },
    Key { key: Key, pressed: bool },     // portable key, see below
}
```

Events are **coalesced per capture tick** into one `Events(Vec<..>)` so that a
high mouse polling rate does not flood the network or cause the classic
"jumpiness" when polling exceeds display refresh.

### Portable `Key`
macOS and Windows use different raw keycodes, so we never send a raw scancode.
`keymap.rs` translates the native key to a portable `Key` enum on capture and
back to a native key on injection (the same approach Synergy/Deskflow use).
Unmappable keys become `Key::Unknown(u32)` and are dropped on injection.

## Bulk channel

### Framing
Each frame is `u32` big-endian length prefix + the (optionally encrypted)
postcard bytes. Max frame size is 64 MiB (guards against a hostile peer forcing
a huge allocation).

### `BulkMsg`
```
enum BulkMsg {
    Hello { version: u16, name: String, screen: (u32,u32) },  // reserved
    Welcome { version: u16, name: String },                   // reserved
    Clipboard(ClipboardData),
    FileBegin { id: u64, name: String, size: u64 },
    FileChunk { id: u64, offset: u64, data: Vec<u8> },
    FileEnd { id: u64 },
    Heartbeat,
}

enum ClipboardData {
    Text(String),
    Image { width: u32, height: u32, rgba: Vec<u8> },  // raw RGBA, no codec
}
```

### File transfer
`FileBegin` → many `FileChunk` (64 KiB each, written at `offset`) → `FileEnd`.
Offsets make it robust and resumable-in-principle. The receiver sanitizes the
filename (strips path components) to prevent path-traversal, and writes into
`./received/`.

## Encryption framing

See [SECURITY.md](./SECURITY.md) for the crypto design. Framing specifics:

- **Bulk (TCP):** the record is `seal(counter, aad=[], plaintext)`. The counter
  is *implicit* — TCP is ordered, so both peers keep in-sync send/recv counters
  starting at 0 and never transmit them.
- **Input (UDP):** the wire is `seq(4 bytes, cleartext) || seal(seq, aad=seq,
  ciphertext)`. The sequence number doubles as the nonce counter and is bound in
  as associated data, so it cannot be tampered with. Packets that fail
  authentication are silently dropped.

## Adding a new message

1. Add the variant to `InputMsg`/`BulkMsg`/`InputEvent` as appropriate.
2. Handle it on both send and receive sides (`run.rs`, `transport.rs`,
   `bulk.rs`).
3. If it changes the meaning of existing bytes, bump `PROTOCOL_VERSION` and add
   a note here.
4. Add a round-trip unit test (see `protocol` tests for the pattern).
