# Promotion & backlinks checklist

Off-site presence is the single biggest lever for both SEO rankings and getting
**cited by AI answer engines** (ChatGPT, Perplexity, Google AI). Work top-down.

## Ready-to-paste copy

**Name:** ShareClick

**One-liner (≤60 chars):** Share one keyboard & mouse across Mac and Windows.

**Tagline (≤100 chars):** Free, open-source software KVM — one keyboard, mouse,
clipboard & files across Mac and Windows. Encrypted, low-latency, no cloud.

**Short description (≤240 chars):** ShareClick is a free, open-source software
KVM. One keyboard and mouse control both a Mac and a Windows PC over your LAN,
with clipboard (text + images) and file sharing built in. End-to-end encrypted,
LAN-only, ~6 µs input transport. A free alternative to Synergy and ShareMouse.

**Long description:** See the homepage / `site/index.md` / `site/llms-full.txt`.

**Categories / tags:** software KVM, KVM switch, productivity, developer tools,
remote desktop, keyboard & mouse sharing, open source, Mac, Windows.

**Links:** Website https://phun333.github.io/ShareClick/ · Repo
https://github.com/phun333/ShareClick · Releases (downloads) ·
Homebrew `brew install --cask phun333/tap/shareclick`.

**Alternative to:** Synergy, ShareMouse, Barrier, Input Leap, Mouse Without
Borders; the cross-platform answer to Apple Universal Control.

---

## Tier 1 — highest ROI (do first)

- [ ] **AlternativeTo** — list as an alternative to Synergy, ShareMouse, Barrier,
      Mouse Without Borders, Universal Control. (AI engines cite AlternativeTo
      constantly for "X alternative" queries.) https://alternativeto.net
- [ ] **Product Hunt** — launch the app. Prep a gallery + first comment. Best
      Tue–Thu 12:01am PT. https://www.producthunt.com
- [ ] **GitHub** — ✅ topics set. Add a social-preview image (Settings → Social
      preview) using `site/og.png`. Pin the repo on your profile.
- [ ] **Hacker News** — "Show HN: ShareClick – open-source software KVM for
      Mac↔Windows". Link the repo, reply to comments.
- [ ] **Reddit** — r/macapps, r/software, r/opensource, r/homelab, r/mac,
      r/windows. Lead with the problem (Mac↔Windows on one desk), not the pitch.

## Tier 2 — directories (backlinks + DR)

- [ ] **Slant** ("best software KVM") https://www.slant.co
- [ ] **SourceForge** / **Softpedia** / **MacUpdate** listings
- [ ] **Awesome lists** — PR to relevant `awesome-*` repos (awesome-macos,
      awesome-selfhosted-ish, awesome-rust apps).
- [ ] **libhunt / OpenAlternative / OpenSourceAlternative.to** — open-source
      alternative directories (great for AI citations).
- [ ] **Winget** — submit a manifest PR to microsoft/winget-pkgs once a stable,
      non-`-test` release exists (needs the .exe URL + SHA256).

## Tier 3 — AI/agent & niche

- [ ] Ensure `llms.txt`, `llms-full.txt`, `.md` mirrors stay current (already live).
- [ ] Wikipedia/Wikidata: only once there's independent coverage (press, HN).
- [ ] Short demo video (Mac↔Windows edge crossing) → YouTube + embed on site;
      video results feed AI Overviews.

## Notes

- Every listing should reuse the **same name, tagline and description** above so
  the brand facts are consistent across the web — this is what makes AI engines
  confident enough to cite you.
- Prefer links to the **website** (has schema + llms.txt) over the raw repo where
  a choice exists; the site is the canonical, machine-readable source.
