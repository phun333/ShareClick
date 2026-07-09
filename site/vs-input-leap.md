# ShareClick vs Input Leap

> A free, open-source Input Leap alternative: encrypted by default, clipboard
> images, file transfer and mDNS discovery for Mac and Windows.

Input Leap is the community-maintained fork of Barrier — a solid, cross-platform
open-source KVM. ShareClick covers the same core idea for **Mac↔Windows**, but is
**encrypted by default**, ships **file transfer** and clipboard images, and uses
zero-config discovery so you never type an IP.

| Feature | ShareClick | Input Leap |
| --- | --- | --- |
| Price | Free & open source | Free & open source |
| Actively maintained | Yes | Yes |
| Encryption | **On by default (X25519 + ChaCha20)** | Optional (manual TLS) |
| Clipboard images | Yes | Text mainly |
| File transfer | **Yes** | Limited / No |
| Auto discovery | **mDNS (no IPs)** | Manual IP setup |
| Input transport | **UDP, ~6 µs** | TCP |
| Linux | Work in progress | Yes |
| Mac & Windows | Yes | Yes |

## Which should you pick?

- **Choose ShareClick** for **Mac↔Windows** if you want encryption on by default,
  file transfer, clipboard images and mDNS discovery out of the box, with the
  lowest input lag.
- **Choose Input Leap** if you need broad **Linux** support and a very mature,
  multi-platform codebase.

Input Leap (and Barrier before it) proved how useful a software KVM is. ShareClick
is the newer, security-first take focused on Mac↔Windows: a UDP input path (~6 µs
transport overhead), encryption on every channel, clipboard images, file transfer,
and mDNS discovery.

- [Download ShareClick (free)](https://github.com/phun333/ShareClick/releases)
- [Setup guide](https://phun333.github.io/ShareClick/how-to-share-mouse-keyboard-mac-windows.md)
