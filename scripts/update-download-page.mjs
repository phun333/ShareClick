#!/usr/bin/env node
/**
 * Updates the auto-generated download block in mintlify/download.mdx with the
 * latest GitHub release: version, publish date, and direct asset links for the
 * macOS .dmg and Windows .exe installers.
 *
 * Run locally:   node scripts/update-download-page.mjs
 * In CI:         GITHUB_TOKEN=... node scripts/update-download-page.mjs
 *
 * It is idempotent — running it without a new release leaves the file byte-for-
 * byte identical, so the CI "commit if changed" step becomes a no-op.
 */

import { readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const REPO = process.env.SHARECLICK_REPO || "phun333/ShareClick";
const __dirname = dirname(fileURLToPath(import.meta.url));
const PAGE = resolve(__dirname, "..", "mintlify", "download.mdx");
const START = "{/* AUTO-DOWNLOAD:START";
const END = "{/* AUTO-DOWNLOAD:END */}";
const RELEASES_LATEST = `https://github.com/${REPO}/releases/latest`;

async function fetchLatestRelease() {
  const headers = { "User-Agent": "shareclick-docs", Accept: "application/vnd.github+json" };
  if (process.env.GITHUB_TOKEN) headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;

  // Prefer the newest non-draft release (falls back through the list so a
  // prerelease still works if that's all there is).
  const res = await fetch(`https://api.github.com/repos/${REPO}/releases?per_page=20`, { headers });
  if (!res.ok) throw new Error(`GitHub API ${res.status} ${res.statusText}`);
  const list = await res.json();
  const rel = list.find((r) => !r.draft) || list[0];
  if (!rel) throw new Error("no releases found");
  return rel;
}

function pickAsset(assets, ...patterns) {
  for (const p of patterns) {
    const hit = assets.find((a) => p.test(a.name));
    if (hit) return hit;
  }
  return null;
}

function formatDate(iso) {
  if (!iso) return "";
  return new Date(iso).toLocaleDateString("en-US", { year: "numeric", month: "long", day: "numeric" });
}

function renderBlock(rel) {
  const assets = rel.assets || [];
  const dmg = pickAsset(assets, /\.dmg$/i);
  const exe = pickAsset(assets, /setup.*\.exe$/i, /\.exe$/i);
  const version = (rel.tag_name || "").replace(/^v/, "");
  const date = formatDate(rel.published_at);
  const macHref = dmg ? dmg.browser_download_url : RELEASES_LATEST;
  const winHref = exe ? exe.browser_download_url : RELEASES_LATEST;
  const versionSuffix = version ? ` Version ${version}.` : "";

  const meta = version
    ? `  **Latest version:** \`v${version}\`${date ? ` · released ${date}` : ""}`
    : `  **Latest version:** the links below always point to the most recent release.`;

  return `${START} — this block is updated automatically on each release. Do not edit by hand. */}

<Info>
${meta}
</Info>

<CardGroup cols={2}>
  <Card title="Download for macOS" icon="apple" href="${macHref}">
    Universal \`.dmg\` — Apple Silicon + Intel.${versionSuffix}
  </Card>
  <Card title="Download for Windows" icon="windows" href="${winHref}">
    \`.exe\` installer — no administrator rights needed.${versionSuffix}
  </Card>
</CardGroup>

${END}`;
}

async function main() {
  const original = await readFile(PAGE, "utf8");
  const startIdx = original.indexOf(START);
  const endIdx = original.indexOf(END);
  if (startIdx === -1 || endIdx === -1) {
    throw new Error(`markers not found in ${PAGE}`);
  }

  let rel;
  try {
    rel = await fetchLatestRelease();
  } catch (err) {
    console.warn(`⚠️  could not fetch latest release: ${err.message}`);
    console.warn("    leaving download.mdx unchanged.");
    return;
  }

  const block = renderBlock(rel);
  const before = original.slice(0, startIdx);
  const after = original.slice(endIdx + END.length);
  const updated = before + block + after;

  if (updated === original) {
    console.log(`✓ download.mdx already up to date (${rel.tag_name}).`);
    return;
  }
  await writeFile(PAGE, updated, "utf8");
  console.log(`✓ updated download.mdx → ${rel.tag_name}`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
