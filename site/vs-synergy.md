# ShareClick vs Synergy

> A free, open-source Synergy alternative to share mouse, keyboard, clipboard and
> files between Mac and Windows. Encrypted, low-latency.

Looking for a **free, open-source Synergy alternative**? ShareClick shares one
keyboard, mouse, clipboard and files between your Mac and Windows PC — with no
license, encrypted by default, and built for the lowest input lag.

| Feature | ShareClick | Synergy |
| --- | --- | --- |
| Price | **Free** | Paid license |
| Open source | Yes (MIT / Apache) | Core (Deskflow) yes; app proprietary |
| Mac & Windows | Yes | Yes |
| Linux | Work in progress | Yes |
| Encryption | X25519 + ChaCha20-Poly1305 | TLS |
| Clipboard (text + images) | Yes | Yes |
| File transfer | Yes | Yes |
| Auto edge switching | Yes (+ hotkey) | Yes |
| Input latency focus | **~6 µs transport (UDP)** | Good |
| Support | Community / GitHub | Commercial support |

## Which should you pick?

- **Choose ShareClick** if you want it **free and open source**, encrypted by
  default, with the lowest input lag and no license to manage.
- **Choose Synergy** if you need first-class **Linux** support and paid
  commercial support, and don't mind the license fee.

Synergy is a mature, respected product — its open core (Deskflow) is excellent.
ShareClick is the newer, free-and-open option focused on Mac↔Windows with a UDP
input path (~6 µs transport overhead), encryption on every channel, and
zero-config mDNS discovery.

- [Download ShareClick (free)](https://github.com/phun333/ShareClick/releases)
- [Setup guide](https://phun333.github.io/ShareClick/how-to-share-mouse-keyboard-mac-windows.md)
