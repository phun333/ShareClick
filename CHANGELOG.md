# Changelog

All notable changes to ShareClick are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project follows
[Semantic Versioning](https://semver.org/). See
[docs/RELEASING.md](./docs/RELEASING.md) for the release process.

## [Unreleased]

_Nothing yet._

## [0.1.0] - 2026-07-06

First release. A complete, low-latency, open-source software KVM.

### Added
- **Input sharing** — one keyboard & mouse across macOS ⇄ Windows, via rdev
  capture + enigo injection, with a portable cross-platform key mapping.
- **Low-latency transport** — hybrid UDP (input) + TCP (bulk) with sequence
  numbers and per-tick event coalescing; ~6 µs one-way loopback overhead.
- **Real control handoff** — rdev `grab` swallows local input while the client
  has control; F12 toggle plus automatic screen-edge switching in both
  directions (client auto-returns via cursor tracking).
- **Encryption** — X25519 + pre-shared-key handshake + ChaCha20-Poly1305 on both
  channels; measured ~20 ns latency cost.
- **Clipboard sync** — text and images (raw RGBA), with echo suppression.
- **File transfer** — chunked, offset-based, path-traversal-safe (`send-file`,
  received into `./received/`).
- **Settings + monitor manager** — TOML config describing the machine/edge
  layout, PSK, and port (`init-config`).
- **mDNS discovery** — zero-config peer finding (`discover`, or `connect` with no
  host).
- **GUI** — macOS menu-bar status item / Windows system tray (`--features tray`).
- **Installers** — universal macOS `.dmg` and Windows `.exe`, built and published
  by CI on tag. No Rust required for end users.
- **Docs** — full `docs/` set (architecture, protocol, security, development,
  releasing, decisions, history).

### Known limitations
- Builds are unsigned (Gatekeeper/SmartScreen prompt on first launch).
- One client at a time; no sliding-window UDP anti-replay yet.

[Unreleased]: https://github.com/ali/ShareClick/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/ali/ShareClick/releases/tag/v0.1.0
