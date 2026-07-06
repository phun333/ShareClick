# SEO / discoverability playbook

Goal: when someone searches for a way to share a mouse & keyboard between Mac and
Windows — or for a free/open-source Synergy/ShareMouse alternative — **ShareClick
shows up at the top.** Being free + open source + genuinely better is our unfair
advantage; this doc is how we turn that into rankings.

SEO is **on-page** (done in code) + **off-page** (links & mentions you build) +
**patience** (Google takes weeks to trust a new site). Do the on-page once, then
grind the off-page list.

---

## Target keywords (what real people type)

Primary (high intent):
- `share mouse and keyboard between mac and windows`
- `software kvm` / `software kvm free`
- `synergy alternative` / `sharemouse alternative` / `barrier alternative`
- `control two computers with one keyboard and mouse`
- `share clipboard between mac and windows`
- `universal control for windows`

Long-tail (easier to win, high conversion):
- `free open source software kvm mac windows`
- `share keyboard mouse mac windows without hardware`
- `move mouse between two computers over network`

The page title, meta description, H1/tagline, feature list, and FAQ all target
these naturally — **never keyword-stuff**; Google punishes it.

---

## On-page SEO — DONE (in `site/index.html`)

- ✅ Keyword-rich `<title>` and `<meta description>` (front-loaded keywords).
- ✅ **`SoftwareApplication` JSON-LD** with `offers` price `0` → eligible for the
  "Free" rich result in Google. `operatingSystem: macOS, Windows`,
  `applicationCategory: UtilitiesApplication` (Google enum), `downloadUrl`,
  `featureList`, `sameAs` (GitHub + X).
- ✅ **`FAQPage` JSON-LD** matching the visible FAQ → eligible for FAQ rich
  results / "People also ask". (The FAQ text must stay visible on the page.)
- ✅ Open Graph + Twitter Card + `og.png` (1200×630) → rich link previews when
  shared anywhere.
- ✅ `canonical`, `robots`, `sitemap.xml`, `robots.txt`, `favicon.svg`.
- ✅ Fast, single static file, mobile-responsive, HTTPS via GitHub Pages → strong
  Core Web Vitals (a real ranking factor).

**Validate after deploy:**
- Rich Results Test → https://search.google.com/test/rich-results (paste the URL;
  expect SoftwareApplication + FAQPage detected, no errors).
- Facebook debugger → https://developers.facebook.com/tools/debug/ (refresh OG).
- PageSpeed Insights → https://pagespeed.web.dev/ (aim for 95+).

---

## Off-page SEO — the actual work (do these in order)

### 1. Get indexed (week 1)
- [ ] **Google Search Console** (https://search.google.com/search-console):
  add the property `https://phun333.github.io/ShareClick/`, verify (HTML tag or
  DNS), then **submit `sitemap.xml`** and use **URL Inspection → Request
  indexing**.
- [ ] **Bing Webmaster Tools** (https://www.bing.com/webmasters): add + submit
  sitemap (also feeds DuckDuckGo/ChatGPT search).

### 2. High-authority backlinks & listings (weeks 1–4)
Each of these is a do-follow-ish mention Google trusts, and a traffic source:
- [ ] **AlternativeTo.net** — add ShareClick as an alternative to Synergy,
  ShareMouse, Barrier, Input Leap. (Huge for "X alternative" searches.)
- [ ] **awesome-lists PRs** — submit to `awesome-selfhosted`, `awesome-rust`,
  `awesome-macos`, `awesome-sysadmin`, any "awesome KVM / remote" list. A merged
  PR = a backlink from a very high-authority repo.
- [ ] **Product Hunt** launch (schedule for a Tue–Thu). Tagline + the og image +
  a short demo GIF.
- [ ] **Hacker News** — "Show HN: ShareClick — free, open-source software KVM
  (Mac/Windows), lower latency than Synergy". Post the repo, be present in
  comments.
- [ ] **Reddit** — r/opensource, r/macapps, r/software, r/sysadmin, r/rust
  (native Rust angle), r/homelab. Value-first, not spammy.
- [ ] **Slant.co / SaaSHub / libhunt** — add the product.
- [ ] **Wikipedia** "Comparison of remote desktop / KVM software" tables — add a
  row if it fits the criteria.

### 3. GitHub as an SEO asset (ongoing)
The repo itself ranks and feeds trust to the site:
- [ ] Repo **topics** (already set) + a crisp **About** with keywords + the site
  URL in the "Website" field.
- [ ] README first paragraph = the primary keyword sentence ("free, open-source
  software KVM to share mouse & keyboard between Mac and Windows…").
- [ ] Link the site from the README; link the repo from the site (mutual).
- [ ] Ship releases regularly + get **stars** — star count correlates with
  ranking and credibility. Ask HN/Reddit/PH visitors to star.

### 4. Content that wins long-tail (weeks 2+)
Publish 2–4 short pages/posts targeting exact queries (blog on the repo wiki, or
extra pages under `site/`):
- [ ] "ShareClick vs Synergy" and "ShareClick vs ShareMouse" comparison pages —
  these intercept people comparing paid tools. Be fair and factual.
- [ ] "How to share a mouse and keyboard between Mac and Windows (free)" tutorial.
- [ ] A short YouTube demo (video ranks + embeds + backlink). Title with the
  primary keyword.

### 5. Optional but strong
- [ ] **Custom domain** (e.g. `shareclick.app`) → set it in GitHub Pages + a
  `CNAME` file in `site/`; update `canonical`, `og:url`, `sitemap.xml`. A branded
  domain outranks a `github.io` path over time.

---

## Why we can actually beat the incumbents

- **"free" + "open source"** are exactly the modifiers people add when the paid
  tools (Synergy, ShareMouse) annoy them with licenses — and we own those words
  honestly.
- The paid tools rank their *marketing* pages; review aggregators rank for
  "best". We emit the same structured signals (price, platform, features) **and**
  have a real free product, so we can win the "alternative" and "free" queries
  they can't.
- Lower latency + encryption + clipboard + files is a genuinely competitive
  feature set — the content is true, which is what sustains rankings.

## Keep it honest

No fake reviews/ratings, no cloaking, no keyword stuffing, no bought links.
Google's spam systems punish all of these and it's not needed — a real free tool
with real structured data and real community mentions wins on its own.
