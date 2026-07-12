# Agent Rules for ShareClick

Rules for any AI agent (and human contributors) working in this repository.

## Style rules (hard requirements)

1. **NO EMOJI. EVER.** Do not use emoji or pictographic characters anywhere:
   - README.md, CHANGELOG.md, CONTRIBUTING.md, or any Markdown file
   - Issue templates, PR templates, workflow files, release notes
   - Commit messages, code comments, log output, CLI output
   - Badges (no emoji inside shields.io labels)
   - Use plain words instead: "Yes/No/Partial" in tables, "Note:"/"Warning:" for callouts.
2. Script/hook output uses plain ASCII: `OK`, `ERROR`, `->` (no checkmarks, crosses, or arrows from Unicode symbol blocks).
3. Keep prose factual and concise. No hype adjectives in user-facing docs.

## Git rules (hard requirements)

1. **NO `git commit` and NO `git push` without explicit user approval.** Always ask first, every time. Prepare changes and show a summary, then wait for the user to approve the commit.
2. Never force-push, rebase published history, or delete branches without explicit user approval.

## Project conventions

- Commit messages follow Conventional Commits (enforced by `.githooks/commit-msg` and CI).
- `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings` must pass before committing.
- Licensing is MIT OR Apache-2.0 (dual). Do not add code or assets with incompatible licenses.
- User-facing changes get a CHANGELOG.md entry.
- Do not commit files larger than 5 MB (pre-commit hook blocks this).
