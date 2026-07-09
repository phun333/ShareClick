# Installing ShareClick

ShareClick is **free and open source**, and the builds are **not signed with a
paid Apple/Microsoft certificate** (that costs money we're not spending). The
app is completely safe — it's the same code you can read in this repo — but your
OS will show a scary warning the first time because it can't verify a paid
developer identity. Here's how to get past it. It only happens **once**.

---

## Package managers (fastest)

- **macOS (Homebrew):**
  ```bash
  brew install --cask phun333/tap/shareclick
  ```
  If macOS still blocks it: `xattr -cr "/Applications/ShareClick.app"`.
- **Windows (Scoop):**
  ```powershell
  scoop install https://raw.githubusercontent.com/phun333/ShareClick/main/packaging/scoop/shareclick.json
  ```

Prefer the manual installers below if you don't use a package manager.

---

## macOS

### 1. Install
1. Download `ShareClick-*.dmg` from the
   [Releases page](https://github.com/phun333/ShareClick/releases).
2. Open the `.dmg` and drag **ShareClick** onto the **Applications** folder.

### 2. First launch (get past Gatekeeper)

Try the simple way first, then the guaranteed way.

**Simple way — right-click Open:**
1. Open the **Applications** folder (Finder → Go → Applications).
2. **Right-click** (or Control-click) **ShareClick** → **Open**.
3. In the dialog, click **Open** again.

**If macOS says the app "is damaged" or "can't be opened", or there's no
Open button** (common on macOS Sequoia / newer), use the guaranteed Terminal
one-liner. This just removes the "downloaded from the internet" quarantine flag:

```bash
xattr -cr /Applications/ShareClick.app
```

Then open the app normally (double-click). That's it — you won't see the warning
again.

> **Where's Terminal?** Press `Cmd`+`Space`, type `Terminal`, Enter. Paste the
> line above, press Enter, then launch ShareClick.

**Alternative (no Terminal) via System Settings:**
1. Double-click ShareClick — it gets blocked.
2. Open **System Settings → Privacy & Security**.
3. Scroll down to the **Security** section; you'll see
   *"ShareClick was blocked…"* → click **Open Anyway**.
4. Confirm with Touch ID / password.

### 3. Grant input permissions (required for a KVM)
ShareClick moves your mouse & keyboard, so macOS requires two permissions.
On first run it will prompt, or add them manually:

**System Settings → Privacy & Security →**
- **Accessibility** → enable **ShareClick**
- **Input Monitoring** → enable **ShareClick**

Quit and relaunch ShareClick after granting them. The menu-bar icon appears in
the top-right.

---

## Windows

### 1. Install
1. Download `ShareClick-Setup-*.exe` from the
   [Releases page](https://github.com/phun333/ShareClick/releases).
2. Run it. **No administrator rights needed** (it installs just for you).

### 2. Get past SmartScreen
Windows may show **"Windows protected your PC"**:
1. Click **More info**.
2. Click **Run anyway**.

This appears because the installer isn't signed with a paid certificate — it's
the same warning every small open-source app gets. It only shows once.

### 3. Allow through the firewall
On first `serve`/`connect`, **Windows Defender Firewall** asks to allow
ShareClick. Tick **Private networks** and click **Allow access** (port 24800).

---

## Why the warnings? (and is it safe?)

- Apple and Microsoft charge for the certificates that make these warnings go
  away (Apple: $99/yr + notarization; Windows: ~$200+/yr EV certificate).
- ShareClick is open source: every line is in this repo and every release is
  built by public CI ([`.github/workflows/release.yml`](../.github/workflows/release.yml)),
  so you can verify exactly what you're running.
- The steps above are the standard, safe way to run trusted unsigned open-source
  software. You are only telling your OS "yes, I trust this app I chose to
  download."

## Prefer to build it yourself?

If you'd rather not use a prebuilt binary at all, you can compile from source —
no warnings, full control. See [DEVELOPMENT.md](./DEVELOPMENT.md).

## Uninstall

- **macOS:** drag `ShareClick.app` from Applications to the Trash. Config lives
  at `~/Library/Application Support/shareclick/`.
- **Windows:** Settings → Apps → ShareClick → Uninstall. Config lives at
  `%APPDATA%\shareclick\`.
