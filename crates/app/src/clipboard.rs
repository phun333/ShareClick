//! Bidirectional clipboard synchronization over the bulk channel.
//!
//! Two loops run per peer connection:
//!  * **watch**: polls the local clipboard; on a genuine local change, sends it.
//!  * **apply**: receives remote clipboard messages and sets them locally.
//!
//! Echo suppression: whenever we *set* the clipboard from a remote message (or
//! send a local change), we remember that text so the watcher does not bounce
//! it straight back and cause an infinite loop.

#![cfg(feature = "native")]

use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use arboard::Clipboard;
use shareclick_protocol::{BulkMsg, ClipboardData};

/// Shared "last known clipboard text" used to suppress echoes.
type LastSeen = Arc<Mutex<Option<String>>>;

/// Poll the local clipboard and forward genuine changes onto `out`.
pub fn watch(out: Sender<BulkMsg>, last: LastSeen) {
    let mut clipboard = match Clipboard::new() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "clipboard unavailable; sync disabled");
            return;
        }
    };
    loop {
        if let Ok(text) = clipboard.get_text() {
            let mut guard = last.lock().unwrap();
            if guard.as_deref() != Some(text.as_str()) {
                *guard = Some(text.clone());
                drop(guard);
                let _ = out.send(BulkMsg::Clipboard(ClipboardData::Text(text)));
            }
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// Apply remote clipboard messages arriving on `inbox` to the local clipboard.
pub fn apply(inbox: Receiver<ClipboardData>, last: LastSeen) {
    let mut clipboard = match Clipboard::new() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "clipboard unavailable; sync disabled");
            return;
        }
    };
    while let Ok(data) = inbox.recv() {
        match data {
            ClipboardData::Text(text) => {
                // Record before setting so our own watcher ignores this change.
                *last.lock().unwrap() = Some(text.clone());
                if let Err(e) = clipboard.set_text(text) {
                    tracing::warn!(error = %e, "failed to set clipboard");
                }
            }
            ClipboardData::Image { .. } => {
                tracing::debug!("image clipboard sync not implemented yet");
            }
        }
    }
}

/// Convenience to build a fresh shared echo-suppression cell.
pub fn shared_last() -> LastSeen {
    Arc::new(Mutex::new(None))
}
