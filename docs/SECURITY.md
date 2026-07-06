# Security & cryptography

Implementation: `crates/protocol/src/crypto.rs`. Design is WireGuard/Noise-
inspired and deliberately conservative — "very high quality, boring crypto."

## Primitives

| Purpose | Primitive | Crate |
|---|---|---|
| Key agreement | X25519 ECDH (ephemeral) | `x25519-dalek` |
| Authentication | Pre-shared key mixed into the KDF salt | — |
| Key derivation | HKDF-SHA256 | `hkdf`, `sha2` |
| Record encryption | ChaCha20-Poly1305 (AEAD) | `chacha20poly1305` |

## Handshake (per session)

1. Each peer generates a **fresh ephemeral** X25519 keypair.
2. Over the TCP bulk channel they exchange 32-byte public keys in the clear
   (safe for ECDH).
3. Both compute the shared secret `DH(our_sk, their_pk)`.
4. `HKDF-SHA256` derives key material with:
   - `salt = PSK` (the user's passphrase) → **authentication**,
   - `ikm = shared_secret`,
   - `info = "sc-v1-kd" || sorted(pubkey_a, pubkey_b)` → binds the transcript.
5. The 128-byte output is split into **four** keys:
   input-i2r, input-r2i, bulk-i2r, bulk-r2i (i2r = initiator→responder).

Each channel + each direction gets its own key, so counters/nonces can never
collide.

## Why this is secure

- **Confidentiality + integrity:** ChaCha20-Poly1305 is an AEAD; any tampering
  fails the Poly1305 tag and the packet is rejected.
- **Authentication / anti-MITM:** without the correct PSK, the two peers derive
  *different* keys and every `open()` fails. An attacker who can intercept and
  replace the ephemeral keys still cannot derive the session keys without the
  PSK. This is why **mDNS discovery is safe** — discovery only finds a candidate
  address; the PSK still proves identity.
- **Forward secrecy:** keys are ephemeral per session, so compromising one
  session's keys does not expose past or future sessions.
- **Nonce safety:** unique key per direction + a monotonic counter (UDP: the
  packet `seq`; TCP: an implicit ordered counter) guarantees no `(key, nonce)`
  pair is ever reused — the cardinal AEAD rule.

## Threat model

**In scope (defended):**
- Passive eavesdropper on the LAN/Wi-Fi.
- Active attacker who forges/replays/tampers packets (rejected by AEAD + counter).
- Imposter advertising the same mDNS service (rejected without the PSK).

**Out of scope (documented limitations):**
- A compromised endpoint (malware on either machine) — ShareClick injects input
  by design, so an attacker with code execution already wins.
- Denial of service by flooding UDP — mitigated only by being LAN-local.
- **Replay within a session:** the counter prevents accepting an *old* counter,
  but there is no sliding-window anti-replay cache yet for out-of-order UDP.
  Practically low-risk on a LAN; see [ROADMAP / future work](./DECISIONS.md).
- PSK strength is the user's responsibility (`config.rs` enforces ≥ 8 chars;
  users should pick a long random passphrase).

## Operational guidance

- Use a **long, random PSK**, identical on both machines. Never commit it.
- The config file holds the PSK in plaintext at
  `~/Library/Application Support/shareclick/config.toml` (macOS) /
  `%APPDATA%\shareclick\config.toml` (Windows). Protect it with normal file
  permissions.

## Tests

`crypto.rs` unit tests assert: round-trip both directions; wrong PSK fails;
tampered ciphertext fails; wrong counter fails; tag-only overhead; and that the
input and bulk channels use independent keys (a ciphertext from one cannot be
opened by the other).
