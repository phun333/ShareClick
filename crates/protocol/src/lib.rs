//! ShareClick wire protocol.
//!
//! Two logical channels:
//!  * **Input channel** (UDP): latency-critical, tiny, delta-encoded events.
//!    Packets carry a monotonic sequence number so the receiver can drop
//!    duplicates and out-of-order stragglers without waiting (no head-of-line
//!    blocking — that is the whole point of using UDP here).
//!  * **Bulk channel** (TCP/reliable): clipboard + file transfer, where
//!    ordering and delivery matter more than microseconds.

use serde::{Deserialize, Serialize};

/// Protocol version. Bump on breaking wire changes.
pub const PROTOCOL_VERSION: u16 = 1;

/// Screen edge a cursor can cross to hand control to a neighbour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

/// Mouse buttons we forward.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    /// Extra buttons (back/forward, etc.) addressed by index.
    Other(u8),
}

/// A single low-level input event. Kept as small as possible on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum InputEvent {
    /// Relative pointer motion (preferred — no absolute coordinate coupling).
    MouseMove { dx: i32, dy: i32 },
    /// Button transition.
    MouseButton { button: MouseButton, pressed: bool },
    /// Scroll wheel deltas (high-resolution / pixel units when available).
    Scroll { dx: f32, dy: f32 },
    /// Keyboard key transition, addressed by an OS-independent scancode.
    Key { code: u32, pressed: bool },
}

/// Messages carried on the **input** (UDP) channel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InputMsg {
    /// A batch of input events captured in one poll tick (coalescing reduces
    /// packet count at high polling rates and avoids the classic "jumpiness"
    /// when mouse rate exceeds display refresh).
    Events(Vec<InputEvent>),
    /// Control handed to this client because the cursor crossed `edge`.
    /// `entry` is the normalized 0.0..1.0 position along the crossed edge.
    Enter { edge: Edge, entry: f32 },
    /// Control returned to the server; client should release/hide its cursor.
    Leave,
    /// Latency probe. `echo_nanos` mirrors the sender's monotonic clock.
    Ping { nonce: u64, echo_nanos: u64 },
    /// Reply to a [`InputMsg::Ping`].
    Pong { nonce: u64, echo_nanos: u64 },
}

/// A framed input packet with a sequence number for dedup/reordering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputPacket {
    pub seq: u32,
    pub msg: InputMsg,
}

/// Messages carried on the **bulk** (reliable) channel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BulkMsg {
    /// Handshake: identify a peer and negotiate capabilities.
    Hello {
        version: u16,
        name: String,
        screen: (u32, u32),
    },
    /// Handshake acknowledgement.
    Welcome { version: u16, name: String },
    /// Clipboard contents changed on the sender.
    Clipboard(ClipboardData),
    /// Begin a file transfer.
    FileBegin { id: u64, name: String, size: u64 },
    /// A chunk of a file identified by `id`.
    FileChunk { id: u64, offset: u64, data: Vec<u8> },
    /// File transfer finished.
    FileEnd { id: u64 },
    /// Keep-alive so peers can detect drops.
    Heartbeat,
}

/// Clipboard payloads we understand.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClipboardData {
    Text(String),
    /// Raw image bytes plus a MIME hint (e.g. "image/png").
    Image { mime: String, bytes: Vec<u8> },
}

/// Errors from (de)serialization.
#[derive(Debug, thiserror::Error)]
pub enum ProtoError {
    #[error("encode failed: {0}")]
    Encode(postcard::Error),
    #[error("decode failed: {0}")]
    Decode(postcard::Error),
}

impl InputPacket {
    pub fn encode(&self) -> Result<Vec<u8>, ProtoError> {
        postcard::to_allocvec(self).map_err(ProtoError::Encode)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, ProtoError> {
        postcard::from_bytes(bytes).map_err(ProtoError::Decode)
    }
}

impl BulkMsg {
    pub fn encode(&self) -> Result<Vec<u8>, ProtoError> {
        postcard::to_allocvec(self).map_err(ProtoError::Encode)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, ProtoError> {
        postcard::from_bytes(bytes).map_err(ProtoError::Decode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_packet_roundtrips() {
        let pkt = InputPacket {
            seq: 42,
            msg: InputMsg::Events(vec![
                InputEvent::MouseMove { dx: -3, dy: 7 },
                InputEvent::MouseButton { button: MouseButton::Left, pressed: true },
            ]),
        };
        let bytes = pkt.encode().unwrap();
        assert_eq!(InputPacket::decode(&bytes).unwrap(), pkt);
    }

    #[test]
    fn mouse_move_packet_is_tiny() {
        // A single relative move must stay small to keep the input path fast.
        let pkt = InputPacket {
            seq: 1,
            msg: InputMsg::Events(vec![InputEvent::MouseMove { dx: 1, dy: 1 }]),
        };
        assert!(pkt.encode().unwrap().len() <= 12, "move packet too large");
    }

    #[test]
    fn bulk_clipboard_roundtrips() {
        let msg = BulkMsg::Clipboard(ClipboardData::Text("hello".into()));
        let bytes = msg.encode().unwrap();
        assert_eq!(BulkMsg::decode(&bytes).unwrap(), msg);
    }
}
