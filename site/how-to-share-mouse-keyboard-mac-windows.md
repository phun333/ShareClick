# How to share a mouse & keyboard between Mac and Windows

> Step-by-step guide to share one mouse, keyboard, clipboard and files between a
> Mac and a Windows PC for free with ShareClick, an open-source software KVM.

You can control both your Mac and your Windows PC with a single keyboard and
mouse — for free, no KVM hardware, no cloud. Here's how with **ShareClick**, an
open-source software KVM. It takes about three minutes.

## 1. Install on both machines

Download **ShareClick** from the [releases page](https://github.com/phun333/ShareClick/releases):
the **.dmg** on your Mac and the **.exe** on your Windows PC. Both launch to the
menu bar (macOS) / system tray (Windows). First launch blocked? See the
[install help](https://github.com/phun333/ShareClick/blob/main/docs/INSTALL.md).

## 2. Set one shared passphrase & layout

Open **Settings & Monitor Manager** on both machines. Enter the **same
passphrase** (it authenticates and encrypts the connection), then say which
machine sits on which screen edge — e.g. the PC is to the right of the Mac.

## 3. Grant permissions

- **macOS:** System Settings → Privacy & Security → enable **Accessibility** and **Input Monitoring** for ShareClick.
- **Windows:** allow ShareClick through the firewall when prompted.

Both machines must be on the same Wi-Fi / LAN.

## 4. Just move your mouse

Slide the cursor into the shared screen edge — your keyboard and mouse now drive
the other computer. Push back to return. Copy on one machine and paste on the
other; the **clipboard syncs automatically**, and you can send files too.

That's it. One keyboard, one mouse, one clipboard across your Mac and Windows PC —
**encrypted, low-latency, and free**.

- [Download ShareClick (free)](https://github.com/phun333/ShareClick/releases)
- [Back to home](https://phun333.github.io/ShareClick/index.md)
