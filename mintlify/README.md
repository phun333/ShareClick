# ShareClick Docs (Mintlify)

AI/LLM-friendly, human-beautiful documentation for ShareClick, built with
[Mintlify](https://mintlify.com).

## Structure

```
mintlify/
  docs.json              # site config: theme, colors, navigation, footer
  introduction.mdx       # landing page
  download.mdx           # download front door (auto-updated from releases)
  quickstart.mdx         # 60-second setup
  installation.mdx       # macOS + Windows install, unsigned-app fixes
  usage.mdx              # app UI + terminal walkthrough
  configuration.mdx      # every config.toml field
  file-transfer.mdx      # sending files + clipboard sync
  troubleshooting.mdx    # common problems and fixes
  concepts/              # how it works: architecture, protocol, security, latency
  develop/               # contribute: development, decisions (ADR), releasing
  compare/               # vs. Synergy, ShareMouse, Barrier, Input Leap, MWB, Universal Control
  logo/ favicon.svg      # brand assets
```

## Local preview

Mintlify's CLI needs a Node LTS version (Node 25+ is unsupported).

```bash
# from this directory
npx mint@latest dev            # live preview at http://localhost:3000
npx mint@latest broken-links   # validate internal links
```

If your default Node is too new, point at an LTS install first, e.g.:

```bash
export PATH="/opt/homebrew/opt/node@22/bin:$PATH"
```

## AI / LLM friendliness

Mintlify automatically serves machine-readable endpoints from the deployed site:

- `/llms.txt` — a structured index of every page for LLMs.
- `/llms-full.txt` — the entire docs as a single plain-text file.
- Each page is available as clean Markdown by appending `.md` to its URL, and
  the `contextual` options in `docs.json` add "Copy page" / "Open in ChatGPT" /
  "Open in Claude" buttons.

Clear frontmatter `title` + `description` on every page, semantic headings, and
tables make the content easy for both humans and models to parse.

## Downloads are managed from here

The [`download.mdx`](./download.mdx) page is the single front door for installers
— users don't need to browse GitHub. The version and direct asset links inside
the `{/* AUTO-DOWNLOAD */}` markers are updated **automatically**:

- **Script:** [`scripts/update-download-page.mjs`](../scripts/update-download-page.mjs)
  fetches the latest GitHub release and rewrites the block. Run it locally with
  `node scripts/update-download-page.mjs` (from the repo root).
- **CI:** [`.github/workflows/docs-download.yml`](../.github/workflows/docs-download.yml)
  runs on every `release` (published/edited) and on manual dispatch, then commits
  `download.mdx` if anything changed.

So the workflow is: cut a release → the download page updates itself. Everything
is driven from the repo; nobody edits links by hand.

## Deploy

Connect this repo to Mintlify (GitHub app) and set the docs directory to
`mintlify/`. Every push to the default branch redeploys. See
<https://mintlify.com/docs/deployment> for details.
