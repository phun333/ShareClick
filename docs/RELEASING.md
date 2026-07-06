# Releasing

This is the exact, repeatable process to ship a new version. Following it means
end users always get working one-click installers and the history stays clean.

## Versioning policy (SemVer)

`MAJOR.MINOR.PATCH`:

- **PATCH** — bug fixes, no wire or config changes.
- **MINOR** — new features, backward-compatible wire/config (old configs still
  load thanks to `#[serde(default)]`; `PROTOCOL_VERSION` unchanged).
- **MAJOR** — breaking wire/config changes → also bump `PROTOCOL_VERSION` in
  `crates/protocol/src/lib.rs` and document the migration in the changelog.

The crate version lives once in the workspace: `Cargo.toml → [workspace.package]
version`. All crates inherit it.

## Release checklist

1. **Green build & tests**
   ```bash
   cargo test                       # native
   cargo test --no-default-features # core
   cargo build --release --features tray
   cargo run --release -- bench --encrypted   # no latency regression
   ```
2. **Bump the version** in `Cargo.toml` (`[workspace.package] version`).
   Run `cargo build` once so `Cargo.lock` updates.
3. **Update [CHANGELOG.md](../CHANGELOG.md):** move `## [Unreleased]` items under
   a new `## [X.Y.Z] - YYYY-MM-DD` heading; start a fresh empty `Unreleased`.
4. **Commit** on `main`:
   ```bash
   git add -A && git commit -m "release: vX.Y.Z"
   ```
5. **Tag & push** — this triggers the CI release workflow:
   ```bash
   git tag vX.Y.Z
   git push origin main --tags
   ```
6. **Wait for CI.** `.github/workflows/release.yml` builds and attaches:
   - `ShareClick-X.Y.Z.dmg` (macOS universal, arm64 + Intel)
   - `ShareClick-Setup-X.Y.Z.exe` (Windows installer)
   to a GitHub Release with auto-generated notes.
7. **Smoke-test the artifacts** on both OSes (install, launch, connect once).
8. **Announce** — the Release page is the download link for users.

## What CI does (`.github/workflows/release.yml`)

- Trigger: pushing a tag matching `v*` (or manual `workflow_dispatch`).
- `macos` job (macos-14): adds both Apple targets, runs
  `packaging/macos/build-app.sh $VERSION` → universal `.app` + `.dmg`.
- `windows` job (windows-latest): `cargo build --release --features tray`, then
  `choco install innosetup` and `ISCC /DMyAppVersion=$VERSION shareclick.iss` →
  `.exe` installer.
- `release` job: downloads both artifacts and publishes the GitHub Release.

## Building installers locally (optional)

```bash
# macOS (produces dist/ShareClick.app + dist/ShareClick-<ver>.dmg)
bash packaging/macos/build-app.sh 0.1.0

# Windows (in a Windows shell, after cargo build --release --features tray)
iscc /DMyAppVersion=0.1.0 packaging\windows\shareclick.iss
```

## Code signing & notarization (current status)

Builds are **unsigned** today (no paid developer certificates), so:

- **macOS:** Gatekeeper blocks first launch. Users right-click the app → **Open**
  once. To remove this friction we would need an Apple Developer ID ($99/yr) and
  a notarization step in the macOS CI job (`codesign` with the Developer ID +
  `xcrun notarytool submit`). The build script already ad-hoc signs so it runs
  locally.
- **Windows:** SmartScreen shows an "unknown publisher" warning; users click
  **More info → Run anyway**. An EV/OV code-signing certificate would remove it.

When certificates are available, add the signing secrets to the repo and the
signing steps to the CI jobs — see the TODO markers in `release.yml`.

## Rollback

If a release is broken, delete the GitHub Release + tag, fix, and re-tag with a
new PATCH version. Never re-use a published version number.
