//! Loopback latency benchmark for the input channel.
//!
//! Spins up an echo responder on one UDP socket and a sender on another, then
//! measures round-trip time for `count` ping/pong pairs. This is our headline
//! metric: input lag is dominated by one-way latency (≈ RTT/2) plus OS event
//! injection. Keeping the transport RTT in the low tens of microseconds on
//! loopback proves the hot path adds negligible overhead.

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::{Duration, Instant};

use shareclick_protocol::crypto::{Handshake, Role};
use shareclick_protocol::InputMsg;

use crate::transport::InputChannel;

fn now_nanos(start: Instant) -> u64 {
    start.elapsed().as_nanos() as u64
}

/// Run `count` round trips over loopback and print latency statistics as
/// autoresearch-style `METRIC` lines.
pub fn run(count: usize, encrypted: bool) -> anyhow::Result<()> {
    let loopback = |port: u16| SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port));

    // Optionally establish a real X25519 + ChaCha20-Poly1305 session so the
    // measured RTT includes the full encrypt/decrypt cost on every packet.
    let (send_sess, recv_sess) = if encrypted {
        let psk = b"benchmark-preshared-key";
        let a = Handshake::new();
        let b = Handshake::new();
        let (a_pub, b_pub) = (a.public_bytes(), b.public_bytes());
        let sa = a.complete(b_pub, psk, Role::Initiator)?;
        let sb = b.complete(a_pub, psk, Role::Responder)?;
        (Some(Arc::new(sa)), Some(Arc::new(sb)))
    } else {
        (None, None)
    };

    // Responder: echoes Ping -> Pong.
    let mut responder = InputChannel::bind(loopback(0), None)?;
    if let Some(s) = recv_sess {
        responder = responder.with_cipher(s);
    }
    let responder_addr = responder.local_addr()?;
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stop_r = stop.clone();

    let handle = std::thread::spawn(move || {
        let mut buf = [0u8; 2048];
        responder
            .set_read_timeout(Some(Duration::from_millis(200)))
            .ok();
        loop {
            if stop_r.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            match responder.recv(&mut buf) {
                Ok(Some((pkt, from))) => {
                    if let InputMsg::Ping { nonce, echo_nanos } = pkt.msg {
                        let _ = responder.send_to(InputMsg::Pong { nonce, echo_nanos }, from);
                    }
                }
                Ok(None) => {}
                Err(_) => {} // timeout; loop to re-check stop flag
            }
        }
    });

    // Sender.
    let mut sender = InputChannel::bind(loopback(0), Some(responder_addr))?;
    if let Some(s) = send_sess {
        sender = sender.with_cipher(s);
    }
    sender.set_read_timeout(Some(Duration::from_millis(200)))?;
    let start = Instant::now();
    let mut buf = [0u8; 2048];

    // Warm up the path (page faults, socket buffers, branch predictors).
    for n in 0..64u64 {
        sender.send(InputMsg::Ping {
            nonce: n,
            echo_nanos: now_nanos(start),
        })?;
        let _ = sender.recv(&mut buf);
    }

    let mut rtts_ns: Vec<u64> = Vec::with_capacity(count);
    let mut lost = 0usize;
    for n in 0..count as u64 {
        let t0 = Instant::now();
        sender.send(InputMsg::Ping {
            nonce: n,
            echo_nanos: now_nanos(start),
        })?;
        match sender.recv(&mut buf) {
            Ok(Some((pkt, _))) => match pkt.msg {
                InputMsg::Pong { .. } => rtts_ns.push(t0.elapsed().as_nanos() as u64),
                _ => lost += 1,
            },
            _ => lost += 1,
        }
    }

    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    handle.join().ok();

    if rtts_ns.is_empty() {
        anyhow::bail!("no round trips completed");
    }
    rtts_ns.sort_unstable();
    let pct = |p: f64| -> f64 {
        let idx = ((rtts_ns.len() as f64 - 1.0) * p).round() as usize;
        rtts_ns[idx] as f64 / 1000.0 // ns -> µs
    };
    let mean_us = rtts_ns.iter().sum::<u64>() as f64 / rtts_ns.len() as f64 / 1000.0;
    let median_us = pct(0.50);
    let p99_us = pct(0.99);
    let owl_us = median_us / 2.0; // one-way latency estimate

    println!(
        "mode={} samples={} lost={}",
        if encrypted { "encrypted" } else { "plain" },
        rtts_ns.len(),
        lost
    );
    println!("METRIC rtt_median_us={median_us:.2}");
    println!("METRIC rtt_p99_us={p99_us:.2}");
    println!("METRIC rtt_mean_us={mean_us:.2}");
    println!("METRIC oneway_us={owl_us:.2}");
    Ok(())
}
