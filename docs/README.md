# ShareClick documentation

This folder is the single source of truth for **what** ShareClick is, **why**
it is built the way it is, and **how** to develop, test, and release it. It is
written so that a new contributor (or an AI agent picking up the work later) can
get fully up to speed and continue without breaking anything.

## Start here

| If you want to… | Read |
|---|---|
| Understand the system at a high level | [ARCHITECTURE.md](./ARCHITECTURE.md) |
| Know exactly what goes over the wire | [PROTOCOL.md](./PROTOCOL.md) |
| Understand the encryption & threat model | [SECURITY.md](./SECURITY.md) |
| Build, run, test, or add a feature | [DEVELOPMENT.md](./DEVELOPMENT.md) |
| Cut a new version / publish installers | [RELEASING.md](./RELEASING.md) |
| Know *why* a design choice was made | [DECISIONS.md](./DECISIONS.md) |
| See what was built, phase by phase | [HISTORY.md](./HISTORY.md) |
| Rank the site / grow discoverability | [SEO.md](./SEO.md) |
| Install as an end user (past the unsigned-app warnings) | [INSTALL.md](./INSTALL.md) |
| Actually use it (set up + connect Mac ↔ Windows) | [USAGE.md](./USAGE.md) |
| Install overview | [../README.md](../README.md) |
| See the changelog | [../CHANGELOG.md](../CHANGELOG.md) |

## The one-paragraph summary

ShareClick is a low-latency, open-source **software KVM**: it shares one
keyboard, mouse, clipboard, and files between macOS and Windows machines over
the LAN. Input travels on an unreliable-but-fast **UDP** channel; clipboard and
files travel on a reliable **TCP** channel. Both are encrypted end-to-end with
X25519 + a pre-shared key + ChaCha20-Poly1305. The measured transport overhead
is ~6 µs one-way, so the code is never the bottleneck.

## Documentation rules (please keep these true)

1. **Every design decision that is non-obvious gets an entry in
   [DECISIONS.md](./DECISIONS.md).** If you change a decision, don't delete the
   old one — supersede it, so the reasoning history survives.
2. **Every user-visible change gets a [CHANGELOG.md](../CHANGELOG.md) entry**
   under `## [Unreleased]`.
3. **The wire protocol is versioned.** Any breaking change bumps
   `PROTOCOL_VERSION` in `crates/protocol/src/lib.rs` and is documented in
   [PROTOCOL.md](./PROTOCOL.md).
4. **Docs live next to the reason they exist.** If you add a module, add a line
   about it in [ARCHITECTURE.md](./ARCHITECTURE.md#module-map).
