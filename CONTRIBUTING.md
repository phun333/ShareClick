# Contributing to ShareClick

Thanks for your interest in contributing! 🎉

## Getting started

```sh
git clone https://github.com/phun333/ShareClick
cd ShareClick
./scripts/setup-hooks.sh   # install git hooks (do this once!)
cargo build
```

The setup script enables shared git hooks that check formatting and commit
messages **before** you push — so CI never surprises you.

## Commit messages — Conventional Commits

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
type(scope): short imperative summary
```

| Type | When to use |
|------|-------------|
| `feat` | New user-facing feature |
| `fix` | Bug fix |
| `docs` | Documentation only |
| `perf` | Performance improvement |
| `refactor` | Code change that is neither a feat nor a fix |
| `test` | Adding or fixing tests |
| `build` | Build system, dependencies, packaging |
| `ci` | CI configuration |
| `chore` | Maintenance that doesn't touch src or tests |
| `revert` | Reverting a previous commit |

**Common scopes:** `pair`, `input`, `net`, `protocol`, `app`, `docs`, `packaging`

Examples:

```
feat(pair): add multi-monitor edge detection
fix(input): stop cursor jitter at screen border
docs: update pairing guide
feat(protocol)!: bump wire format to v5   ← "!" marks a breaking change
```

The `commit-msg` hook and CI both enforce this. Breaking changes get a `!`
after the type/scope and a `BREAKING CHANGE:` footer in the body.

## Code style

- `cargo fmt --all` before committing (the pre-commit hook checks this)
- `cargo clippy --workspace --all-targets` must be warning-free (CI enforces `-D warnings`)
- Add tests for protocol or logic changes where practical

## Pull requests

1. Fork and create a branch: `feat/multi-monitor` or `fix/border-jitter`
2. Keep PRs focused — one logical change per PR
3. Fill in the PR template; if behavior changed, mention how you tested it
   (ShareClick is a two-machine app, real hardware testing matters!)
4. Update `CHANGELOG.md` for user-facing changes

## Reporting bugs

Use the [bug report template](https://github.com/phun333/ShareClick/issues/new/choose).
Since ShareClick runs on **two machines**, always include the OS and version of
both sides, plus `RUST_LOG=debug` output when possible.

## License

ShareClick is dual-licensed under [MIT](./LICENSE-MIT) or
[Apache-2.0](./LICENSE-APACHE), at your option.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in this project by you, as defined in the
Apache-2.0 license, shall be dual-licensed as above, without any additional
terms or conditions.

## Questions?

Open a [Discussion](https://github.com/phun333/ShareClick/discussions) — issues
are reserved for confirmed bugs and planned features.
