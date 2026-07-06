//! Bidirectional clipboard synchronization over the bulk channel (text +
//! images).
//!
//! Two loops run per peer connection:
//!  * **watch**: polls the local clipboard; on a genuine local change, sends it.
//!  * **apply**: receives remote clipboard messages and sets them locally.
//!
//! Echo suppression: whenever we *set* the clipboard from a remote message (or
//! send a local change), we remember a fingerprint of it so the watcher does
//! not bounce it straight back into an infinite loop.

#![cfg(feature = "native")]

use std::borrow::Cow;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use arboard::{Clipboard, ImageData};
use shareclick_protocol::{BulkMsg, ClipboardData};

/// A cheap fingerprint of the current clipboard, used to detect real changes
/// and to suppress echoes.
#[derive(Clone, PartialEq)]
pub(crate) enum Fingerprint {
    Text(String),
    Image(u64),
}

/// Shared "last known clipboard" used to suppress echoes.
pub(crate) type LastSeen = Arc<Mutex<Option<Fingerprint>>>;

fn hash_image(width: u32, height: u32, rgba: &[u8]) -> u64 {
    let mut h = DefaultHasher::new();
    width.hash(&mut h);
    height.hash(&mut h);
    rgba.hash(&mut h);
    h.finish()
}

/// Poll the local clipboard and forward genuine changes onto `out`.
pub(crate) fn watch(out: Sender<BulkMsg>, last: LastSeen) {
    let mut clipboard = match Clipboard::new() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "clipboard unavailable; sync disabled");
            return;
        }
    };
    loop {
        // Text takes priority; fall back to an image if there's no text.
        if let Ok(text) = clipboard.get_text() {
            if !text.is_empty() {
                let fp = Fingerprint::Text(text.clone());
                let mut guard = last.lock().unwrap();
                if guard.as_ref() != Some(&fp) {
                    *guard = Some(fp);
                    drop(guard);
                    let _ = out.send(BulkMsg::Clipboard(ClipboardData::Text(text)));
                }
                std::thread::sleep(Duration::from_millis(250));
                continue;
            }
        }
        if let Ok(img) = clipboard.get_image() {
            let (w, h) = (img.width as u32, img.height as u32);
            let rgba = img.bytes.into_owned();
            let fp = Fingerprint::Image(hash_image(w, h, &rgba));
            let mut guard = last.lock().unwrap();
            if guard.as_ref() != Some(&fp) {
                *guard = Some(fp);
                drop(guard);
                let _ = out.send(BulkMsg::Clipboard(ClipboardData::Image {
                    width: w,
                    height: h,
                    rgba,
                }));
            }
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// Apply remote clipboard messages arriving on `inbox` to the local clipboard.
pub(crate) fn apply(inbox: Receiver<ClipboardData>, last: LastSeen) {
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
                *last.lock().unwrap() = Some(Fingerprint::Text(text.clone()));
                if let Err(e) = clipboard.set_text(text) {
                    tracing::warn!(error = %e, "failed to set clipboard text");
                }
            }
            ClipboardData::Image { width, height, rgba } => {
                *last.lock().unwrap() = Some(Fingerprint::Image(hash_image(width, height, &rgba)));
                let img = ImageData {
                    width: width as usize,
                    height: height as usize,
                    bytes: Cow::Owned(rgba),
                };
                if let Err(e) = clipboard.set_image(img) {
                    tracing::warn!(error = %e, "failed to set clipboard image");
                }
            }
        }
    }
}

/// Convenience to build a fresh shared echo-suppression cell.
pub(crate) fn shared_last() -> LastSeen {
    Arc::new(Mutex::new(None))
}
