// gen-licenses-extra.mjs — append npm + fonts notices to the cargo-about HTML.
// Usage: node scripts/gen-licenses-extra.mjs <path-to-THIRD-PARTY-LICENSES.html>
//
// Walks the *production* npm dependency tree (what actually ships in the built
// frontend), reads each package's license text, and appends one section per
// package, plus an explicit SIL OFL note for the bundled Anton / Jost fonts.
import { readFileSync, writeFileSync, existsSync, readdirSync } from "node:fs";
import { execSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const REPO = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const OUT = process.argv[2];
if (!OUT) { console.error("usage: gen-licenses-extra.mjs <out.html>"); process.exit(2); }

const esc = (s) => String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
const LICENSE_FILES = ["LICENSE", "LICENSE.md", "LICENSE.txt", "LICENCE", "LICENCE.md", "LICENSE-MIT", "COPYING"];

function licenseText(dir) {
  for (const f of LICENSE_FILES) {
    const p = path.join(dir, f);
    if (existsSync(p)) return readFileSync(p, "utf8").trim();
  }
  // @fontsource and some packages keep the text under the package root with odd names
  try {
    const hit = readdirSync(dir).find((n) => /^licen[sc]e/i.test(n));
    if (hit) return readFileSync(path.join(dir, hit), "utf8").trim();
  } catch { /* ignore */ }
  return null;
}

// Production dependency directories (deduped). `npm ls` exits non-zero on benign
// warnings, so tolerate failure and parse whatever it printed.
let paths = "";
try {
  paths = execSync("npm ls --omit=dev --all --parseable", { cwd: REPO, encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] });
} catch (e) {
  paths = e.stdout || "";
}
const dirs = [...new Set(paths.split("\n").map((s) => s.trim()).filter((d) => d && d.includes("node_modules")))];

const pkgs = [];
for (const dir of dirs) {
  const pj = path.join(dir, "package.json");
  if (!existsSync(pj)) continue;
  let meta;
  try { meta = JSON.parse(readFileSync(pj, "utf8")); } catch { continue; }
  const license = typeof meta.license === "string" ? meta.license : (meta.license?.type || (meta.licenses?.[0]?.type) || "see package");
  pkgs.push({ name: meta.name, version: meta.version, license, text: licenseText(dir), author: meta.author?.name || meta.author || "" });
}
pkgs.sort((a, b) => a.name.localeCompare(b.name));

let html = `\n<h1 style="margin-top:3rem">Frontend (npm) packages</h1>\n`;
html += `<p class="intro">The following npm packages are bundled into the UsageOS frontend.</p>\n`;
for (const p of pkgs) {
  html += `<h2>${esc(p.name)} ${esc(p.version)} — ${esc(p.license)}</h2>\n`;
  if (p.text) html += `<pre>${esc(p.text)}</pre>\n`;
  else html += `<p class="crates">${esc(p.license)} licensed${p.author ? ` · © ${esc(p.author)}` : ""} (full text in the package).</p>\n`;
}

// Explicit OFL note for the display/body typefaces (bundled via @fontsource).
html += `\n<h1 style="margin-top:3rem">Fonts</h1>\n`;
html += `<h2>Anton &amp; Jost — SIL Open Font License 1.1</h2>\n`;
html += `<p class="crates">Anton (© The Anton Project Authors) and Jost (© indestructible type*) are bundled under the SIL Open Font License, Version 1.1. The license permits bundling and redistribution; the full OFL text ships with each font in the app and at <code>landing/public/fonts/</code>.</p>\n`;

const doc = readFileSync(OUT, "utf8");
writeFileSync(OUT, doc.replace(/<\/body>/i, `${html}</body>`));
console.log(`[licenses] appended ${pkgs.length} npm packages + fonts note`);
