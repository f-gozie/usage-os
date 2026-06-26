// Generates the 24 hourly menubar tray frames: the mono Contexts mark + a "now" triangle that
// points at the hour. Black template PNGs (macOS tints them for the light/dark menu bar).
// Run from the repo root:  node design/logo/assets/gen-tray-icons.mjs   (needs rsvg-convert)
import { writeFileSync, mkdirSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { tmpdir } from "node:os";
import { join } from "node:path";

const OUT = "src-tauri/icons/tray";
mkdirSync(OUT, { recursive: true });

// Mono mark: balanced Contexts arcs (gap 18°, r33, stroke 10) — same geometry as mark-mono.svg.
const ARCS = [
  "M55.16 17.41 A33 33 0 0 1 80.81 61.83",
  "M75.65 70.77 A33 33 0 0 1 24.35 70.77",
  "M19.19 61.83 A33 33 0 0 1 44.84 17.41",
];
const INK = "#000000";
const CX = 50, CY = 50, R = 33, SW = 10;
const rad = (d) => (d * Math.PI) / 180;

// A triangle just outside the ring, tip toward centre, at the hour angle (0 = midnight, top).
function nowTriangle(hour) {
  const t = (hour / 24) * 360;
  const dir = [Math.sin(rad(t)), -Math.cos(rad(t))];
  const perp = [Math.cos(rad(t)), Math.sin(rad(t))];
  const at = (radius, off) => [
    CX + dir[0] * radius + perp[0] * off,
    CY + dir[1] * radius + perp[1] * off,
  ];
  const p = (xy) => `${xy[0].toFixed(2)},${xy[1].toFixed(2)}`;
  const tip = at(R + SW / 2 + 1, 0);
  const b1 = at(R + SW / 2 + 10, 5);
  const b2 = at(R + SW / 2 + 10, -5);
  return `<polygon points="${p(tip)} ${p(b1)} ${p(b2)}" fill="${INK}"/>`;
}

function svg(hour) {
  const arcs = ARCS.map(
    (d) => `<path d="${d}" fill="none" stroke="${INK}" stroke-width="${SW}"/>`,
  ).join("");
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">${arcs}${nowTriangle(hour)}</svg>`;
}

for (let h = 0; h < 24; h++) {
  const tmp = join(tmpdir(), `usageos-tray-${h}.svg`);
  writeFileSync(tmp, svg(h));
  const out = join(OUT, `now-${String(h).padStart(2, "0")}.png`);
  execFileSync("rsvg-convert", ["-w", "44", "-h", "44", tmp, "-o", out]);
}
console.log(`Generated 24 tray frames → ${OUT}/now-00.png … now-23.png`);
