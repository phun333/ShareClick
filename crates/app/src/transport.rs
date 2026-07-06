//! Low-latency input transport over UDP.
//!
//! Design goals:
//!  * One dedicated blocking socket per direction — no async scheduler jitter
//!    on the hot path.
//!  * Monotonic sequence numbers so the receiver drops duplicates and late
//!    stragglers instead of blocking (no head-of-line blocking).
//!  * Small packets (postcard-encoded) to minimize serialization + wire time.

use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use shareclick_protocol::crypto::Session;
use shareclick_protocol::{InputMsg, InputPacket};

/// A UDP endpoint for the input channel.
pub struct InputChannel {
    socket: UdpSocket,
    seq: AtomicU32,
    /// Highest sequence number seen so far, for straggler rejection.
    last_seen: AtomicU32,
    /// When set, every packet is ChaCha20-Poly1305 sealed. The 32-bit sequence
    /// number travels in the clear as the AEAD nonce counter (and is bound in
    /// as associated data). Immutable after construction so the hot path never
    /// locks.
    cipher: Option<Arc<Session>>,
}

impl InputChannel {
    /// Bind to `bind` and (optionally) connect to `peer` so `send` needs no
    /// per-call address lookup.
    pub fn bind(bind: SocketAddr, peer: Option<SocketAddr>) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(bind)?;
        if let Some(peer) = peer {
            socket.connect(peer)?;
        }
        Ok(Self {
            socket,
            seq: AtomicU32::new(0),
            last_seen: AtomicU32::new(0),
            cipher: None,
        })
    }

    /// Enable encryption for this endpoint using an established session.
    pub fn with_cipher(mut self, session: Arc<Session>) -> Self {
        self.cipher = Some(session);
        self
    }

    /// Serialize + (optionally) encrypt one packet into wire bytes.
    fn frame(&self, seq: u32, msg: InputMsg) -> anyhow::Result<Vec<u8>> {
        let body = InputPacket { seq, msg }.encode()?;
        match &self.cipher {
            Some(session) => {
                let seq_bytes = seq.to_le_bytes();
                let ct = session.seal(seq as u64, &seq_bytes, &body);
                let mut out = Vec::with_capacity(4 + ct.len());
                out.extend_from_slice(&seq_bytes);
                out.extend_from_slice(&ct);
                Ok(out)
            }
            None => Ok(body),
        }
    }

    /// Decode wire bytes back into a packet, decrypting if needed. Returns
    /// `None` for packets that fail authentication (dropped, loop continues).
    fn deframe(&self, bytes: &[u8]) -> Option<InputPacket> {
        match &self.cipher {
            Some(session) => {
                if bytes.len() < 4 {
                    return None;
                }
                let seq = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                let pt = session.open(seq as u64, &bytes[..4], &bytes[4..]).ok()?;
                InputPacket::decode(&pt).ok()
            }
            None => InputPacket::decode(bytes).ok(),
        }
    }

    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.socket.local_addr()
    }

    /// Send a message to the connected peer, assigning the next sequence number.
    pub fn send(&self, msg: InputMsg) -> anyhow::Result<()> {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        let bytes = self.frame(seq, msg)?;
        self.socket.send(&bytes)?;
        Ok(())
    }

    /// Send a message to a specific address (used before a peer is fixed).
    pub fn send_to(&self, msg: InputMsg, addr: SocketAddr) -> anyhow::Result<()> {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        let bytes = self.frame(seq, msg)?;
        self.socket.send_to(&bytes, addr)?;
        Ok(())
    }

    /// Block for the next packet. Returns the decoded packet and its source.
    /// Late stragglers (seq <= last_seen for the *Events* stream) are dropped.
    pub fn recv(&self, buf: &mut [u8]) -> anyhow::Result<Option<(InputPacket, SocketAddr)>> {
        let (n, from) = self.socket.recv_from(buf)?;
        let pkt = match self.deframe(&buf[..n]) {
            Some(p) => p,
            None => return Ok(None), // undecodable or failed authentication
        };

        // Only enforce monotonicity on the continuous event stream; control
        // and ping/pong messages must always pass through.
        if matches!(pkt.msg, InputMsg::Events(_)) {
            let prev = self.last_seen.load(Ordering::Relaxed);
            if pkt.seq != 0 && pkt.seq <= prev {
                return Ok(None); // duplicate or straggler
            }
            self.last_seen.store(pkt.seq, Ordering::Relaxed);
        }
        Ok(Some((pkt, from)))
    }

    pub fn set_read_timeout(&self, dur: Option<std::time::Duration>) -> std::io::Result<()> {
        self.socket.set_read_timeout(dur)
    }
}
