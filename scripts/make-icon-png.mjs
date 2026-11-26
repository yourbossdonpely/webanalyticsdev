// Web Analytics Dev by Don PELY — renders assets/icon-1024.png from scratch.
// Pure Node (no native deps): draws the logo pixel-by-pixel, encodes PNG via zlib.
import { deflateSync } from 'node:zlib';
import { writeFileSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const SIZE = 1024;

const hex = (h) => [parseInt(h.slice(1, 3), 16), parseInt(h.slice(3, 5), 16), parseInt(h.slice(5, 7), 16)];
const BG_TOP = hex('#232f57');
const BG_BOT = hex('#121a36');
const TEAL = hex('#38e1c6');
const VIOLET = hex('#8b7bff');
const GREEN = hex('#3ddc84');

const buf = Buffer.alloc(SIZE * SIZE * 4);

const clamp01 = (v) => (v < 0 ? 0 : v > 1 ? 1 : v);
const smooth = (e0, e1, x) => {
  const t = clamp01((x - e0) / (e1 - e0));
  return t * t * (3 - 2 * t);
};
// Signed distance to rounded box centered at origin, half-extents b, radius r.
function sdRoundBox(px, py, bx, by, r) {
  const qx = Math.abs(px) - bx + r;
  const qy = Math.abs(py) - by + r;
  const ax = Math.max(qx, 0);
  const ay = Math.max(qy, 0);
  return Math.hypot(ax, ay) + Math.min(Math.max(qx, qy), 0) - r;
}
// Distance from point to segment.
function sdSeg(px, py, ax, ay, bx, by) {
  const dx = bx - ax;
  const dy = by - ay;
  const len2 = dx * dx + dy * dy;
  let t = len2 === 0 ? 0 : ((px - ax) * dx + (py - ay) * dy) / len2;
  t = clamp01(t);
  return Math.hypot(px - (ax + t * dx), py - (ay + t * dy));
}

// Pulse polyline (matches assets/logo.svg).
const PULSE = [[150, 560], [330, 560], [400, 380], [500, 700], [580, 470], [650, 530], [880, 530]];
const PULSE_W = 64;
const BARS = [
  [200, 620, 90, 200, TEAL, 0.35],
  [330, 540, 90, 280, TEAL, 0.55],
  [460, 450, 90, 370, TEAL, 0.75],
  [590, 360, 90, 460, VIOLET, 0.85],
];

for (let y = 0; y < SIZE; y++) {
  for (let x = 0; x < SIZE; x++) {
    const i = (y * SIZE + x) * 4;
    // Background rounded square (32..992, radius 230).
    const dBg = sdRoundBox(x - 512, y - 512, 480, 480, 230);
    const bgA = 1 - smooth(-1.5, 1.5, dBg);
    if (bgA <= 0) {
      buf[i + 3] = 0;
      continue;
    }
    const gt = y / (SIZE - 1);
    let r = BG_TOP[0] + (BG_BOT[0] - BG_TOP[0]) * gt;
    let g = BG_TOP[1] + (BG_BOT[1] - BG_TOP[1]) * gt;
    let b = BG_TOP[2] + (BG_BOT[2] - BG_TOP[2]) * gt;

    // Bars (rounded, alpha-blended).
    for (const [bx, by, bw, bh, col, alpha] of BARS) {
      const d = sdRoundBox(x - (bx + bw / 2), y - (by + bh / 2), bw / 2, bh / 2, 26);
      const a = (1 - smooth(-1.5, 1.5, d)) * alpha;
      r += (col[0] - r) * a;
      g += (col[1] - g) * a;
      b += (col[2] - b) * a;
    }

    // Pulse line with horizontal teal→violet gradient.
    let dPulse = Infinity;
    for (let s = 0; s < PULSE.length - 1; s++) {
      dPulse = Math.min(dPulse, sdSeg(x, y, PULSE[s][0], PULSE[s][1], PULSE[s + 1][0], PULSE[s + 1][1]));
    }
    // Round caps at the ends.
    dPulse = Math.min(dPulse, Math.hypot(x - PULSE[0][0], y - PULSE[0][1]));
    const last = PULSE[PULSE.length - 1];
    dPulse = Math.min(dPulse, Math.hypot(x - last[0], y - last[1]));
    const pa = 1 - smooth(PULSE_W / 2 - 1.5, PULSE_W / 2 + 1.5, dPulse);
    if (pa > 0) {
      const t = clamp01((x - 150) / (880 - 150));
      const pr = TEAL[0] + (VIOLET[0] - TEAL[0]) * t;
      const pg = TEAL[1] + (VIOLET[1] - TEAL[1]) * t;
      const pb = TEAL[2] + (VIOLET[2] - TEAL[2]) * t;
      r += (pr - r) * pa;
      g += (pg - g) * pa;
      b += (pb - b) * pa;
    }

    // Live dot + glow ring.
    const dDot = Math.hypot(x - 800, y - 250);
    const dotA = 1 - smooth(50.5, 53.5, dDot);
    r += (GREEN[0] - r) * dotA;
    g += (GREEN[1] - g) * dotA;
    b += (GREEN[2] - b) * dotA;
    const ringD = Math.abs(dDot - 66);
    const ringA = (1 - smooth(8, 14, ringD)) * 0.45;
    r += (GREEN[0] - r) * ringA;
    g += (GREEN[1] - g) * ringA;
    b += (GREEN[2] - b) * ringA;

    buf[i] = Math.round(r);
    buf[i + 1] = Math.round(g);
    buf[i + 2] = Math.round(b);
    buf[i + 3] = Math.round(bgA * 255);
  }
}

// ---- minimal PNG encoder (RGBA8, filter 0) ----
const crcTable = new Int32Array(256);
for (let n = 0; n < 256; n++) {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  crcTable[n] = c;
}
function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, 'ascii'), data]);
  let c = 0xffffffff;
  for (const byte of body) c = crcTable[(c ^ byte) & 0xff] ^ (c >>> 8);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE((c ^ 0xffffffff) >>> 0);
  return Buffer.concat([len, body, crc]);
}
const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(SIZE, 0);
ihdr.writeUInt32BE(SIZE, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // RGBA
const raw = Buffer.alloc((SIZE * 4 + 1) * SIZE);
for (let y = 0; y < SIZE; y++) {
  raw[y * (SIZE * 4 + 1)] = 0;
  buf.copy(raw, y * (SIZE * 4 + 1) + 1, y * SIZE * 4, (y + 1) * SIZE * 4);
}
const png = Buffer.concat([
  Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
  chunk('IHDR', ihdr),
  chunk('IDAT', deflateSync(raw, { level: 9 })),
  chunk('IEND', Buffer.alloc(0)),
]);
mkdirSync(join(root, 'assets'), { recursive: true });
const out = join(root, 'assets', 'icon-1024.png');
writeFileSync(out, png);
console.log(`wrote ${out} (${(png.length / 1024).toFixed(0)} KB)`);
