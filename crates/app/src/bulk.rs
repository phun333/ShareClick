//! Reliable "bulk" channel (TCP) for clipboard sync and file transfer.
//!
//! Frames are length-prefixed: `u32` big-endian byte count followed by the
//! postcard-encoded [`BulkMsg`]. Ordering and delivery matter here more than
//! microseconds, so TCP is the right tool (unlike the input path).

use std::io::{Read, Write};
use std::net::TcpStream;

use shareclick_protocol::BulkMsg;

/// Max frame size we will accept, to avoid unbounded allocations from a peer.
const MAX_FRAME: u32 = 64 * 1024 * 1024; // 64 MiB

/// A framed connection over a TCP stream.
pub struct BulkConn {
    stream: TcpStream,
}

impl BulkConn {
    pub fn new(stream: TcpStream) -> std::io::Result<Self> {
        stream.set_nodelay(true)?; // clipboard/file latency: don't Nagle-buffer
        Ok(Self { stream })
    }

    /// Clone the underlying stream into an independent framed handle so one
    /// thread can read while another writes.
    pub fn try_clone(&self) -> std::io::Result<Self> {
        Ok(Self {
            stream: self.stream.try_clone()?,
        })
    }

    pub fn send(&mut self, msg: &BulkMsg) -> anyhow::Result<()> {
        let bytes = msg.encode()?;
        let len = bytes.len() as u32;
        self.stream.write_all(&len.to_be_bytes())?;
        self.stream.write_all(&bytes)?;
        self.stream.flush()?;
        Ok(())
    }

    pub fn recv(&mut self) -> anyhow::Result<BulkMsg> {
        let mut len_buf = [0u8; 4];
        self.stream.read_exact(&mut len_buf)?;
        let len = u32::from_be_bytes(len_buf);
        if len > MAX_FRAME {
            anyhow::bail!("bulk frame too large: {len} bytes");
        }
        let mut buf = vec![0u8; len as usize];
        self.stream.read_exact(&mut buf)?;
        Ok(BulkMsg::decode(&buf)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shareclick_protocol::ClipboardData;
    use std::net::{TcpListener, TcpStream};

    #[test]
    fn bulk_frames_roundtrip_over_tcp() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut conn = BulkConn::new(stream).unwrap();
            // Echo back three framed messages.
            for _ in 0..3 {
                let msg = conn.recv().unwrap();
                conn.send(&msg).unwrap();
            }
        });

        let mut client = BulkConn::new(TcpStream::connect(addr).unwrap()).unwrap();
        let msgs = vec![
            BulkMsg::Clipboard(ClipboardData::Text("hello world".into())),
            BulkMsg::Heartbeat,
            BulkMsg::FileBegin { id: 7, name: "a.txt".into(), size: 12 },
        ];
        for m in &msgs {
            client.send(m).unwrap();
            assert_eq!(&client.recv().unwrap(), m);
        }
        server.join().unwrap();
    }
}
