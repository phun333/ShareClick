# Using ShareClick

How to actually share your keyboard, mouse, clipboard and files once ShareClick
is installed on both machines. Two ways: the **app UI** (easiest) or the
**terminal** (best for a first test, because you can see the logs).

First, if the app won't open at all (macOS "damaged", Windows SmartScreen) or you
haven't installed yet, see [INSTALL.md](./INSTALL.md).

---

## Key idea (read this first)

- The machine whose **physical keyboard & mouse** you use is the **server**.
- The machine you want to **control** is the **client**.
- Both machines need the **same passphrase** (it authenticates + encrypts).
- Both machines must be on the **same Wi‑Fi / network**.

Example used below: **Mac = server**, **Windows PC = client**. (You can swap the
roles — just swap which one runs "server" vs "client".)

---

## Option A — the app UI (recommended for normal use)

### On BOTH machines: open Settings and set the passphrase

1. Launch ShareClick. It lives in the **menu bar** (macOS, top‑right) or the
   **system tray** (Windows, bottom‑right).
2. Click the icon → **Settings & Monitor Manager**. This opens a text file
   (`config.toml`).
3. Edit it (see the fields below), **save**, and close the editor.

**On the Mac (server)** set:
```toml
name = "mac"                    # this machine's name
psk  = "pick-a-long-secret"     # SAME on both machines
port = 24800
auto_edge_switch = true

[[machines]]
name = "mac"
screen = [1470, 956]            # this Mac's resolution
right  = "windows"              # the PC is to the right of the Mac

[[machines]]
name = "windows"
screen = [1920, 1080]           # the PC's resolution
left   = "mac"
```

**On the Windows PC (client)** set the *same* file, but change `name` and add
`server_host` (the Mac's IP address):
```toml
name = "windows"
psk  = "pick-a-long-secret"     # EXACTLY the same as the Mac
port = 24800
auto_edge_switch = true
server_host = "192.168.1.20"    # the Mac's IP (see "Find the server's IP")

[[machines]]
name = "mac"
screen = [1470, 956]
right  = "windows"

[[machines]]
name = "windows"
screen = [1920, 1080]
left   = "mac"
```

### Then start it

1. **On the Mac:** grant permissions the first time — System Settings → Privacy &
   Security → enable **Accessibility** and **Input Monitoring** for ShareClick.
   Then tray icon → **Start Server**.
2. **On the Windows PC:** tray icon → **Start Client**. Allow it through the
   **firewall** when Windows asks (Private networks → Allow).

### Use it

- Push your mouse into the **shared screen edge** (in the example: the Mac's
  right edge) → your keyboard & mouse now control the PC.
- Push back to the opposite edge to return. Or press **F12** on the server to
  toggle manually.
- **Copy** on one machine, **paste** on the other — the clipboard syncs.

---

## Option B — the terminal (best for the first test / debugging)

Running from a terminal shows live logs, which makes the first setup much easier
to diagnose.

### macOS (server)
```bash
# from the app, or the built binary:
shareclick init-config                 # writes the config once
open -e ~/Library/Application\ Support/shareclick/config.toml   # edit + save
shareclick serve                       # start the server
```
Grant **Accessibility** + **Input Monitoring** to the Terminal (or the app) in
System Settings → Privacy & Security, then run `serve` again.

### Windows (client)
Open **PowerShell** and find the installed exe:
```powershell
$exe = Get-ChildItem "$env:LOCALAPPDATA\Programs\ShareClick","$env:ProgramFiles\ShareClick" `
       -Filter shareclick.exe -Recurse -ErrorAction SilentlyContinue |
       Select-Object -First 1 -ExpandProperty FullName
& $exe init-config
notepad "$env:APPDATA\shareclick\config.toml"    # edit (name=windows, same psk, server_host=Mac IP) + save
& $exe connect                                    # allow through the firewall when asked
```

You should see **"client authenticated (encrypted session established)"** on the
Mac. Then test the edge switch, clipboard, and:
```bash
# send a file to the other machine (use the OTHER machine's IP:port)
shareclick send-file 192.168.1.30:24800 ./report.pdf   # lands in ./received there
```

---

## Find the server's IP

- **macOS:** System Settings → Wi‑Fi → **Details** → IP address, or in Terminal:
  `ipconfig getifaddr en0`
- **Windows:** in PowerShell: `ipconfig` → look for **IPv4 Address** under your
  Wi‑Fi/Ethernet adapter.

Tip: with `auto_edge_switch` on and both on the same LAN, the client can also
find the server automatically over mDNS — running `connect` with no
`server_host` will search for it.

---

## Config fields

| Field | Meaning |
|---|---|
| `name` | This machine's name. Must match one entry under `[[machines]]`. |
| `psk` | Shared passphrase (≥ 8 chars). Identical on both machines. Authenticates + encrypts. |
| `port` | Network port (default 24800). Same on both. |
| `auto_edge_switch` | Hand control over when the cursor hits a bordered edge. |
| `server_host` | (Client only) the server's IP, e.g. `192.168.1.20`. Omit to auto‑discover. |
| `[[machines]]` | The layout: each machine's `screen = [w, h]` and which peer is on each edge (`left`/`right`/`top`/`bottom`). |

---

## Troubleshooting

| Symptom | Fix |
|---|---|
| Mouse doesn't move (Mac server) | Enable **Accessibility** *and* **Input Monitoring**; quit and reopen after granting. |
| `handshake/auth failed` | The `psk` isn't identical on both machines. |
| Client can't connect | Same Wi‑Fi? Firewall allowed on Windows? Correct `server_host` IP? Try `shareclick discover`. |
| `name ... is not present` | `name` must match a `[[machines]]` entry. |
| Nothing happens at the edge | Check the layout edges (`right`/`left`) and that `auto_edge_switch = true`. F12 always works as a fallback. |
| Windows: no tray icon after launching | New icons hide under the **"^" (show hidden icons)** arrow by the clock — drag ShareClick onto the taskbar. For a first test you can skip the tray entirely and run `shareclick.exe connect` from PowerShell. |

Still stuck? Open an issue with the terminal output from both machines:
<https://github.com/phun333/ShareClick/issues>.
