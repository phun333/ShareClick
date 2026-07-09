# ShareClick — share one keyboard & mouse across Mac and Windows

> Free, open-source software KVM. One keyboard, mouse, clipboard and files
> across a Mac and a Windows PC over your LAN. Encrypted, low-latency, no cloud.

ShareClick is a free, open-source **software KVM**. It lets one keyboard and mouse
control both a Mac and a Windows PC over your local network, with clipboard (text
and images) and file sharing built in. Every channel is end-to-end encrypted
(X25519 key exchange + ChaCha20-Poly1305) and it runs LAN-only — no hardware, no
cloud, no account. It is a free alternative to Synergy, ShareMouse, Barrier and
Input Leap, built for the lowest input lag (~6 µs transport overhead).

- **Price:** Free — open source (MIT / Apache-2.0), unlimited machines
- **Platforms:** macOS and Windows
- **Download:** https://github.com/phun333/ShareClick/releases
- **Source:** https://github.com/phun333/ShareClick

## Features

- Share one keyboard and mouse across Mac and Windows
- Clipboard sync (text and images)
- File transfer over the LAN
- End-to-end encryption (X25519 + ChaCha20-Poly1305), authenticated by a shared passphrase
- Automatic screen-edge switching — push the cursor to the border to switch machines (plus a hotkey)
- mDNS zero-config discovery — no IP addresses to type
- Lowest input lag: ~6 µs transport overhead on a UDP input path

## FAQ

**What is ShareClick?**
ShareClick is a free, open-source software KVM. It lets one keyboard and mouse
control both a Mac and a Windows PC over your local network, with clipboard and
file sharing built in. Everything is end-to-end encrypted and runs LAN-only — no
hardware, no cloud, no account.

**Is ShareClick free?**
Yes. It is completely free and open source (MIT / Apache-2.0). No account, no
subscription, no cloud.

**Does it share a mouse and keyboard between Mac and Windows?**
Yes. ShareClick is a software KVM: one keyboard and mouse control both your Mac
and Windows PC over the local network. Move the cursor to a screen edge to switch
machines.

**Is ShareClick a free alternative to Synergy and ShareMouse?**
Yes. It does what paid tools like Synergy and ShareMouse do — input sharing,
clipboard sync and file transfer — but it is free, open source, and built for
lower input lag.

**Does it sync the clipboard and transfer files?**
Yes. The clipboard (text and images) syncs automatically, and you can send files
straight across the encrypted connection.

**Is it secure and does it need the internet?**
It runs on your local network only — no cloud. Every channel is end-to-end
encrypted with X25519 key exchange and ChaCha20-Poly1305, authenticated by a
shared passphrase.

## More

- [Setup guide](https://phun333.github.io/ShareClick/how-to-share-mouse-keyboard-mac-windows.md)
- [ShareClick vs Synergy](https://phun333.github.io/ShareClick/vs-synergy.md)
- [ShareClick vs ShareMouse](https://phun333.github.io/ShareClick/vs-sharemouse.md)
- [ShareClick vs Barrier](https://phun333.github.io/ShareClick/vs-barrier.md)
- [Pricing](https://phun333.github.io/ShareClick/pricing.md)
- [Install help](https://github.com/phun333/ShareClick/blob/main/docs/INSTALL.md)
