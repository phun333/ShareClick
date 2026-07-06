# Product marketing context — ShareClick

Shared context for the marketing skills. Read this before asking the user
questions.

## What it is
ShareClick is a **free, open-source software KVM**: it shares one keyboard,
mouse, clipboard and files between a Mac and a Windows PC over the local network.
No KVM hardware, no cloud, no account. Encrypted end-to-end and built for the
lowest input lag (~6 µs transport overhead). Menu-bar app on macOS, system tray
on Windows.

## Category & positioning
- Category: software KVM / input sharing / keyboard-mouse sharing.
- One-liner: "One keyboard & mouse across your Mac and Windows PC — free, open, encrypted."
- Wedge words we own honestly: **free**, **open source**, **lower input lag**,
  **encrypted by default**, **no license**.

## ICP / audiences
- Developers & power users with both a Mac and a Windows PC on one desk.
- People frustrated by paid licenses of Synergy / ShareMouse.
- Homelab / self-hosted enthusiasts (LAN-only, no cloud is a selling point).
- Rust community (native Rust project).

## Primary keywords
- share mouse and keyboard between mac and windows
- software KVM (free)
- synergy alternative / sharemouse alternative / barrier alternative
- control two computers with one keyboard and mouse
- share clipboard between mac and windows
- universal control for windows

## Competitors (be honest about them)
- **Synergy** — paid; mature; open core (Deskflow); great Linux support.
- **ShareMouse** — paid (free for 2 personal PCs); very polished drag-and-drop.
- **Barrier** — free/open but unmaintained (archived); users migrating away.
- **Deskflow / Input Leap / Lan Mouse** — open source; Lan Mouse is the fast
  Rust input-only reference.

## Differentiators
- Free & open source (MIT / Apache-2.0), no license for any number of machines.
- Encrypted by default (X25519 + ChaCha20-Poly1305), authenticated by a PSK.
- Lowest input lag focus (UDP input path, ~6 µs transport, per-tick coalescing).
- Clipboard text + images, file transfer, automatic edge switching, mDNS
  discovery (no IPs to type).

## Assets & links
- Site: https://phun333.github.io/ShareClick/
- Repo: https://github.com/phun333/ShareClick
- X / Twitter: https://x.com/wiredaddict (handle @wiredaddict)
- Comparison pages live: /vs-synergy.html, /vs-sharemouse.html, /vs-barrier.html
- Setup guide: /how-to-share-mouse-keyboard-mac-windows.html
- OG image: /og.png (1200×630)
- License: MIT / Apache-2.0. Pricing = Free (no pricing page; "free" counts).

## Asset gaps to create (needed for directory/PH launch)
- 5–8 real screenshots (1920×1080) of the app in use.
- A 60–90s demo video (screen recording of edge-switching + clipboard + file).
- Square 1024×1024 logo + PNG/SVG logo set (we have favicon.svg + og.png).

## Constraints / tone
- Honest, factual, no hype, no fake reviews/ratings, no keyword stuffing.
- Minimal, monospace, black/white + one blue accent visual brand.
- LAN-only, privacy-first (no tracking/analytics on the site).
