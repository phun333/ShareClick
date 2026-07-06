//! File transfer over the reliable bulk channel.
//!
//! Wire flow: `FileBegin { id, name, size }` → many `FileChunk { id, offset,
//! data }` → `FileEnd { id }`. The receiver writes chunks at their offset so
//! transfers are resumable in principle and robust to reordering-free TCP.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};

use shareclick_protocol::BulkMsg;

use crate::bulk::BulkConn;

/// Chunk size on the wire. 64 KiB balances syscall overhead against latency.
const CHUNK: usize = 64 * 1024;

/// Connect to a listening peer's bulk port and stream `path` to it.
pub fn send_file(addr: SocketAddr, path: &Path) -> anyhow::Result<()> {
    let meta = fs::metadata(path)?;
    let size = meta.len();
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file.bin".into());
    let id = rand_id();

    let mut conn = BulkConn::new(TcpStream::connect(addr)?)?;
    conn.send(&BulkMsg::FileBegin { id, name: name.clone(), size })?;

    let mut file = File::open(path)?;
    let mut buf = vec![0u8; CHUNK];
    let mut offset = 0u64;
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        conn.send(&BulkMsg::FileChunk {
            id,
            offset,
            data: buf[..n].to_vec(),
        })?;
        offset += n as u64;
    }
    conn.send(&BulkMsg::FileEnd { id })?;
    tracing::info!(%name, size, "file sent");
    Ok(())
}

/// Reassembles incoming files into `dir`. Reused by the server's bulk reader
/// and by tests.
pub struct FileReceiver {
    dir: PathBuf,
    open: HashMap<u64, (File, String)>,
}

impl FileReceiver {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            open: HashMap::new(),
        }
    }

    /// Feed one bulk message. Non-file messages are ignored (returns `false`).
    /// Returns `true` if the message was a file message we handled.
    pub fn handle(&mut self, msg: &BulkMsg) -> anyhow::Result<bool> {
        match msg {
            BulkMsg::FileBegin { id, name, size } => {
                fs::create_dir_all(&self.dir)?;
                let dest = self.dir.join(sanitize(name));
                let file = File::create(&dest)?;
                file.set_len(*size)?;
                tracing::info!(name = %name, size, path = %dest.display(), "receiving file");
                self.open.insert(*id, (file, name.clone()));
                Ok(true)
            }
            BulkMsg::FileChunk { id, offset, data } => {
                if let Some((file, _)) = self.open.get_mut(id) {
                    file.seek(SeekFrom::Start(*offset))?;
                    file.write_all(data)?;
                }
                Ok(true)
            }
            BulkMsg::FileEnd { id } => {
                if let Some((file, name)) = self.open.remove(id) {
                    file.sync_all()?;
                    tracing::info!(%name, "file complete");
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

/// Strip any path components from a peer-supplied file name (path-traversal
/// safety — never let a remote pick where bytes land).
fn sanitize(name: &str) -> String {
    Path::new(name)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty() && s != "." && s != "..")
        .unwrap_or_else(|| "received.bin".into())
}

/// Small, dependency-free unique id from the system clock + a counter.
fn rand_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    t ^ (COUNTER.fetch_add(0x9E3779B97F4A7C15, Ordering::Relaxed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn file_transfers_over_bulk_channel() {
        // Prepare a source file with > 1 chunk of data.
        let tmp = std::env::temp_dir().join(format!("sc_send_{}", rand_id()));
        fs::create_dir_all(&tmp).unwrap();
        let src = tmp.join("payload.bin");
        let data: Vec<u8> = (0..(CHUNK * 2 + 123)).map(|i| (i % 251) as u8).collect();
        fs::write(&src, &data).unwrap();

        let recv_dir = tmp.join("in");
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let recv_dir_thread = recv_dir.clone();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut conn = BulkConn::new(stream).unwrap();
            let mut rx = FileReceiver::new(recv_dir_thread);
            loop {
                match conn.recv() {
                    Ok(msg) => {
                        let is_end = matches!(msg, BulkMsg::FileEnd { .. });
                        rx.handle(&msg).unwrap();
                        if is_end {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        send_file(addr, &src).unwrap();
        server.join().unwrap();

        let got = fs::read(recv_dir.join("payload.bin")).unwrap();
        assert_eq!(got, data, "received file must match source byte-for-byte");
        let _ = fs::remove_dir_all(&tmp);
    }
}
