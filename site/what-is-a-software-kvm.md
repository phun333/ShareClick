# What is a software KVM?

> A software KVM lets one keyboard and mouse control several computers over the
> network — no hardware box. Here's how it works, how it differs from a hardware
> KVM, and the best free option for Mac and Windows.

A **software KVM** is an application that lets **one keyboard and mouse control
several computers** over a network, switching between them by moving the cursor to
a screen edge — with **no hardware box or cables**. It usually shares the clipboard
and can transfer files too.

## What does KVM stand for?

KVM stands for **Keyboard, Video and Mouse**. The term comes from hardware "KVM
switches" that let you drive several computers from a single keyboard, monitor and
mouse. A *software* KVM keeps the idea but drops the hardware: it shares the
keyboard and mouse (and typically the clipboard and files) between machines over
your local network, while each computer keeps using its own display.

## How does a software KVM work?

- You install a small app on each computer and put them on the same network (LAN).
- You lay out the screens — e.g. "the PC is to the right of the Mac".
- When you push the mouse past a shared screen edge, control jumps to the next
  computer; your keystrokes and mouse movements are sent to it over the network.
- The clipboard syncs automatically, so you can copy on one machine and paste on
  the other, and many tools (including ShareClick) can send files across too.

## Software KVM vs hardware KVM

| | Software KVM | Hardware KVM |
| --- | --- | --- |
| Extra hardware | **None** | Physical switch + cables |
| Shares the display | No — each PC uses its own screen | Yes (switches monitors) |
| Clipboard & files | **Yes (synced over network)** | No |
| Cost | **Often free** | Buy the device |
| Best for | Two+ computers on one desk sharing input | Servers / headless machines |

## Is there a free software KVM for Mac and Windows?

Yes. **ShareClick** is a free, open-source software KVM that shares one keyboard,
mouse, clipboard and files between a **Mac and a Windows PC** over the local
network. Every channel is end-to-end encrypted (X25519 + ChaCha20-Poly1305), it
runs LAN-only (no cloud), and it's built for the lowest input lag (~6 µs transport
overhead). It's a free alternative to Synergy, ShareMouse, Barrier and Input Leap.

In short: a software KVM turns two computers on your desk into one seamless
workspace — one keyboard, one mouse, one clipboard — without any extra hardware.

- [Download ShareClick (free)](https://github.com/phun333/ShareClick/releases)
- [Setup guide](https://phun333.github.io/ShareClick/how-to-share-mouse-keyboard-mac-windows.md)
