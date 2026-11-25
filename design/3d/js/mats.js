// Procedural PBR-style materials: every texture is painted on a canvas (colour map, the same map doubles as a
// bump map). UVs are baked in metres, so each material has a "metres per texture tile" scale.
import * as THREE from 'three';

export const rng = (seed) => { let s = seed >>> 0; return () => ((s = (Math.imul(s, 1664525) + 1013904223) >>> 0) / 4294967296); };
const hex = (a) => Math.round(Math.max(0, Math.min(1, a)) * 255).toString(16).padStart(2, '0');

function tex(n, draw, o = {}) {
  const c = document.createElement('canvas'); c.width = c.height = n;
  const g = c.getContext('2d');
  draw(g, n, rng(o.seed || 7));
  const t = new THREE.CanvasTexture(c);
  t.wrapS = t.wrapT = THREE.RepeatWrapping; t.anisotropy = 8;
  t.colorSpace = o.linear ? THREE.NoColorSpace : THREE.SRGBColorSpace;
  return t;
}
function wrapAt(n, x, y, r, f) {
  for (const dx of [-n, 0, n]) for (const dy of [-n, 0, n]) {
    if (x + dx > -r && x + dx < n + r && y + dy > -r && y + dy < n + r) f(x + dx, y + dy);
  }
}
function blotch(g, n, rand, count, rmin, rmax, colors, a0, a1) {
  for (let i = 0; i < count; i++) {
    const x = rand() * n, y = rand() * n, r = rmin + rand() * (rmax - rmin);
    const col = colors[(rand() * colors.length) | 0], a = a0 + rand() * (a1 - a0);
    wrapAt(n, x, y, r, (X, Y) => {
      const gr = g.createRadialGradient(X, Y, 0, X, Y, r);
      gr.addColorStop(0, col + hex(a)); gr.addColorStop(1, col + '00');
      g.fillStyle = gr; g.beginPath(); g.arc(X, Y, r, 0, 7); g.fill();
    });
  }
}
function speckle(g, n, rand, count, colors, size, a0 = 0.2, a1 = 0.6) {
  for (let i = 0; i < count; i++) {
    const x = rand() * n, y = rand() * n, s = size * (0.4 + rand());
    g.fillStyle = colors[(rand() * colors.length) | 0] + hex(a0 + rand() * (a1 - a0));
    g.fillRect(x, y, s, s);
  }
}
function strokes(g, n, rand, count, colors, len, w, a0, a1, dirY = 1) {
  for (let i = 0; i < count; i++) {
    const x = rand() * n, y = rand() * n, l = len * (0.5 + rand());
    g.strokeStyle = colors[(rand() * colors.length) | 0] + hex(a0 + rand() * (a1 - a0)); g.lineWidth = w * (0.5 + rand());
    g.beginPath(); g.moveTo(x, y); g.lineTo(x + (rand() - 0.5) * 4, y + l * dirY); g.stroke();
  }
}

const T = {
  plasterOchre: () => tex(512, (g, n, r) => {
    g.fillStyle = '#c8a263'; g.fillRect(0, 0, n, n);
    blotch(g, n, r, 90, 30, 120, ['#d9b97c', '#b88f50', '#c9a874', '#a98448'], 0.10, 0.28);
    speckle(g, n, r, 5000, ['#8d6f3f', '#e6cf9b', '#6e5430'], 1.6, 0.1, 0.4);
    strokes(g, n, r, 120, ['#5a4a30', '#3d3a30'], 90, 5, 0.04, 0.14);
  }),
  plasterWhite: () => tex(512, (g, n, r) => {
    g.fillStyle = '#d6cfbf'; g.fillRect(0, 0, n, n);
    blotch(g, n, r, 80, 30, 110, ['#e8e2d2', '#bdb5a2', '#c8bfa8'], 0.12, 0.3);
    speckle(g, n, r, 4000, ['#9b927e', '#f1ece0'], 1.4, 0.1, 0.35);
    strokes(g, n, r, 60, ['#6a604c'], 70, 4, 0.03, 0.1);
  }, { seed: 11 }),
  plasterDark: () => tex(512, (g, n, r) => { // maze wing: damp stained stucco
    g.fillStyle = '#4d4f46'; g.fillRect(0, 0, n, n);
    blotch(g, n, r, 110, 25, 110, ['#3a3c34', '#5e6155', '#2e3a30', '#6a6a58'], 0.12, 0.35);
    strokes(g, n, r, 160, ['#1d211c', '#2f4430'], 110, 6, 0.06, 0.2);
    speckle(g, n, r, 4000, ['#202420', '#77786a'], 1.6, 0.1, 0.35);
  }, { seed: 13 }),
  concrete: () => tex(512, (g, n, r) => {
    g.fillStyle = '#8b8a84'; g.fillRect(0, 0, n, n);
    blotch(g, n, r, 100, 20, 90, ['#9b9a93', '#77766f', '#a5a49c'], 0.1, 0.3);
    speckle(g, n, r, 7000, ['#55544f', '#b5b3aa'], 1.8, 0.15, 0.5);
  }, { seed: 17 }),
  blocks: () => tex(512, (g, n, r) => { // cement block wall, 0.4 x 0.2 m
    g.fillStyle = '#6d6b66'; g.fillRect(0, 0, n, n);
    const bw = 128, bh = 64;
    for (let j = 0; j < n / bh; j++) for (let i = -1; i < n / bw + 1; i++) {
      const x = i * bw + (j % 2 ? bw / 2 : 0), y = j * bh, v = 150 + ((r() * 36) | 0);
      g.fillStyle = `rgb(${v},${v - 2},${v - 8})`; g.fillRect(x + 3, y + 3, bw - 6, bh - 6);
    }
    blotch(g, n, r, 60, 30, 100, ['#4a4a44', '#aaa79c'], 0.1, 0.26);
    speckle(g, n, r, 3000, ['#3c3c38', '#c0bdb2'], 1.6, 0.1, 0.4);
  }, { seed: 19 }),
  tileTerracotta: () => tex(512, (g, n, r) => {
    const k = 4, s = n / k;
    for (let i = 0; i < k; i++) for (let j = 0; j < k; j++) {
      const v = (r() * 28) | 0; g.fillStyle = `rgb(${168 + v},${96 + v / 2},${64 + v / 3})`; g.fillRect(i * s, j * s, s, s);
      blotch(g, n, r, 1, 20, 60, ['#8a4a2a', '#c98c5a'], 0.1, 0.2);
    }
    g.strokeStyle = '#4a4036'; g.lineWidth = 4; for (let i = 0; i <= k; i++) { g.beginPath(); g.moveTo(i * s, 0); g.lineTo(i * s, n); g.stroke(); g.beginPath(); g.moveTo(0, i * s); g.lineTo(n, i * s); g.stroke(); }
    speckle(g, n, r, 4000, ['#5a2f1c', '#d8a070'], 1.6, 0.1, 0.35);
  }, { seed: 23 }),
  marble: () => tex(512, (g, n, r) => {
    g.fillStyle = '#d9d7d0'; g.fillRect(0, 0, n, n);
    blotch(g, n, r, 60, 40, 140, ['#c4c2ba', '#eceae3', '#b5b3ad'], 0.15, 0.35);
    for (let i = 0; i < 14; i++) {
      g.strokeStyle = '#6f6d68' + hex(0.12 + r() * 0.25); g.lineWidth = 0.6 + r() * 2.2; g.beginPath();
      let x = r() * n, y = r() * n; g.moveTo(x, y);
      for (let s = 0; s < 9; s++) { x += (r() - 0.35) * 90; y += (r() - 0.5) * 90; g.lineTo(x, y); }
      g.stroke();
    }
    g.strokeStyle = '#78756e'; g.lineWidth = 3; const s = n / 2;
    for (let i = 0; i <= 2; i++) { g.beginPath(); g.moveTo(i * s, 0); g.lineTo(i * s, n); g.stroke(); g.beginPath(); g.moveTo(0, i * s); g.lineTo(n, i * s); g.stroke(); }
  }, { seed: 29 }),
  wood: () => tex(512, (g, n, r) => { // floor planks 0.12 m wide
    const ph = 64;
    for (let j = 0; j < n / ph; j++) {
      const v = (r() * 30) | 0; g.fillStyle = `rgb(${120 + v},${78 + v * 0.6},${46 + v * 0.3})`; g.fillRect(0, j * ph, n, ph);
      strokes(g, n, r, 18, ['#3b2414', '#a47a4a'], 120, 1.5, 0.1, 0.35, 0);
      for (let i = 0; i < 60; i++) { g.strokeStyle = '#2e1b0d' + hex(0.05 + r() * 0.12); g.lineWidth = 1; const y = j * ph + r() * ph; g.beginPath(); g.moveTo(0, y); g.lineTo(n, y + (r() - 0.5) * 6); g.stroke(); }
      g.fillStyle = '#1e1208'; g.fillRect(0, j * ph, n, 3);
      const cx = r() * n; g.fillRect(cx, j * ph, 3, ph);
    }
  }, { seed: 31 }),
  crate: () => tex(256, (g, n, r) => {
    g.fillStyle = '#9a7549'; g.fillRect(0, 0, n, n);
    for (let j = 0; j < 4; j++) { g.fillStyle = `rgb(${150 + (r() * 30 | 0)},${112 + (r() * 24 | 0)},${70})`; g.fillRect(0, j * 64 + 2, n, 60); }
    strokes(g, n, r, 120, ['#3d2810', '#c19868'], 100, 1.2, 0.1, 0.4, 0);
    g.fillStyle = '#4a3016'; g.fillRect(0, 0, 18, n); g.fillRect(n - 18, 0, 18, n);
  }, { seed: 37 }),
  poolTile: () => tex(512, (g, n, r) => {
    const k = 16, s = n / k;
    for (let i = 0; i < k; i++) for (let j = 0; j < k; j++) { const v = (r() * 24) | 0; g.fillStyle = `rgb(${110 + v},${196 + v / 2},${214 + v / 3})`; g.fillRect(i * s, j * s, s, s); }
    g.strokeStyle = '#dcebee'; g.lineWidth = 1.6; for (let i = 0; i <= k; i++) { g.beginPath(); g.moveTo(i * s, 0); g.lineTo(i * s, n); g.stroke(); g.beginPath(); g.moveTo(0, i * s); g.lineTo(n, i * s); g.stroke(); }
    blotch(g, n, r, 30, 30, 90, ['#3f8aa0', '#aee0ea'], 0.08, 0.2);
  }, { seed: 41 }),
  deck: () => tex(512, (g, n, r) => { // pool deck: pale stone slabs 0.6 m
    const k = 3, s = n / k;
    for (let i = 0; i < k; i++) for (let j = 0; j < k; j++) { const v = (r() * 24) | 0; g.fillStyle = `rgb(${196 + v},${186 + v},${164 + v})`; g.fillRect(i * s, j * s, s, s); }
    g.strokeStyle = '#7a7466'; g.lineWidth = 3; for (let i = 0; i <= k; i++) { g.beginPath(); g.moveTo(i * s, 0); g.lineTo(i * s, n); g.stroke(); g.beginPath(); g.moveTo(0, i * s); g.lineTo(n, i * s); g.stroke(); }
    blotch(g, n, r, 50, 20, 80, ['#8a826c', '#e6dfc8'], 0.1, 0.26); speckle(g, n, r, 5000, ['#6a6454', '#f0e9d4'], 1.6, 0.1, 0.4);
  }, { seed: 43 }),
  roofTar: () => tex(512, (g, n, r) => {
    g.fillStyle = '#4a4b4c'; g.fillRect(0, 0, n, n);
    blotch(g, n, r, 100, 25, 100, ['#3a3b3c', '#5d5e5e', '#6b6358'], 0.1, 0.3);
    speckle(g, n, r, 9000, ['#232424', '#8a8c8c', '#a89c88'], 2, 0.2, 0.6);
  }, { seed: 47 }),
  grass: () => tex(512, (g, n, r) => {
    g.fillStyle = '#486a2e'; g.fillRect(0, 0, n, n);
    blotch(g, n, r, 140, 20, 100, ['#5f8a38', '#3a5a26', '#7b9a44', '#4f7a30', '#6c7a38'], 0.12, 0.35);
    strokes(g, n, r, 9000, ['#2f4a1c', '#6e9a3a', '#86ad48', '#3d5e22', '#9aa856'], 9, 1.2, 0.3, 0.8);
  }, { seed: 53 }),
  dirt: () => tex(512, (g, n, r) => {
    g.fillStyle = '#74593b'; g.fillRect(0, 0, n, n);
    blotch(g, n, r, 120, 20, 100, ['#8a6c48', '#5e4630', '#7e6a50', '#6c5238'], 0.12, 0.3);
    speckle(g, n, r, 8000, ['#3a2a1a', '#a88a62', '#caa878'], 2.4, 0.2, 0.7);
    for (let i = 0; i < 160; i++) { const x = r() * n, y = r() * n; g.fillStyle = (r() < 0.5 ? '#9a8a74' : '#4a3a2a') + hex(0.5); g.beginPath(); g.ellipse(x, y, 1.5 + r() * 3, 1 + r() * 2, r() * 3, 0, 7); g.fill(); }
  }, { seed: 59 }),
  gravel: () => tex(512, (g, n, r) => {
    g.fillStyle = '#8a8476'; g.fillRect(0, 0, n, n);
    for (let i = 0; i < 2600; i++) { const v = 90 + (r() * 100 | 0); g.fillStyle = `rgb(${v},${v - 4},${v - 14})`; g.beginPath(); g.ellipse(r() * n, r() * n, 2 + r() * 4, 1.5 + r() * 3, r() * 3, 0, 7); g.fill(); }
    blotch(g, n, r, 30, 40, 100, ['#5a5448', '#b4ae9c'], 0.06, 0.16);
  }, { seed: 61 }),
  asphalt: () => tex(512, (g, n, r) => {
    g.fillStyle = '#35363a'; g.fillRect(0, 0, n, n);
    blotch(g, n, r, 80, 30, 110, ['#2a2b2e', '#46474b', '#3e3a36'], 0.1, 0.3);
    speckle(g, n, r, 9000, ['#1c1c1e', '#6d6e72', '#85807a'], 1.8, 0.2, 0.6);
  }, { seed: 67 }),
  metal: () => tex(256, (g, n, r) => { // corrugated sheet
    for (let i = 0; i < n; i += 16) { const gr = g.createLinearGradient(i, 0, i + 16, 0); gr.addColorStop(0, '#7e8288'); gr.addColorStop(0.5, '#c4c8cc'); gr.addColorStop(1, '#6a6e74'); g.fillStyle = gr; g.fillRect(i, 0, 16, n); }
    blotch(g, n, r, 24, 20, 60, ['#7a4a2a', '#5a3a20', '#9a6038'], 0.12, 0.35);
    strokes(g, n, r, 50, ['#4a3020'], 80, 3, 0.05, 0.2);
  }, { seed: 71 }),
  rust: () => tex(256, (g, n, r) => {
    g.fillStyle = '#6a4a38'; g.fillRect(0, 0, n, n);
    blotch(g, n, r, 50, 14, 60, ['#8a4a22', '#4a3226', '#a85a2a', '#2c2a28'], 0.2, 0.55);
    speckle(g, n, r, 2400, ['#2a1c14', '#b8703a'], 1.6, 0.2, 0.6);
  }, { seed: 73 }),
  carpet: () => tex(256, (g, n, r) => {
    g.fillStyle = '#6a1c22'; g.fillRect(0, 0, n, n);
    speckle(g, n, r, 14000, ['#4a1014', '#8a2a30', '#3a0c10'], 1.4, 0.3, 0.8);
    g.strokeStyle = '#c9a24a99'; g.lineWidth = 5; g.strokeRect(16, 16, n - 32, n - 32);
  }, { seed: 79 }),
  fabric: () => tex(128, (g, n, r) => { g.fillStyle = '#6c3b32'; g.fillRect(0, 0, n, n); speckle(g, n, r, 3000, ['#4a2620', '#8a5246'], 1.4, 0.3, 0.7); }, { seed: 83 }),
  paintRed: () => tex(128, (g, n, r) => { g.fillStyle = '#8c1a18'; g.fillRect(0, 0, n, n); blotch(g, n, r, 20, 10, 40, ['#5a0e0c', '#b02a24'], 0.15, 0.4); }, { seed: 89 }),
  paintGreen: () => tex(128, (g, n, r) => { g.fillStyle = '#3b4a36'; g.fillRect(0, 0, n, n); blotch(g, n, r, 20, 10, 40, ['#222c20', '#566450'], 0.15, 0.4); speckle(g, n, r, 600, ['#8a8a70'], 1.2, 0.2, 0.5); }, { seed: 97 }),
  darkMetal: () => tex(128, (g, n, r) => { g.fillStyle = '#2b2e32'; g.fillRect(0, 0, n, n); blotch(g, n, r, 20, 10, 40, ['#1a1c1e', '#44484e'], 0.2, 0.5); }, { seed: 101 }),
  bark: () => tex(128, (g, n, r) => { g.fillStyle = '#6d5b46'; g.fillRect(0, 0, n, n); strokes(g, n, r, 400, ['#2e251a', '#9a8668', '#4a3c2c'], 50, 3, 0.3, 0.8); }, { seed: 103 }),
  leaf: () => tex(128, (g, n, r) => { g.fillStyle = '#2f5a24'; g.fillRect(0, 0, n, n); blotch(g, n, r, 50, 8, 30, ['#4a7a30', '#1f3f18', '#6a9a3c'], 0.3, 0.7); strokes(g, n, r, 300, ['#8ab04a', '#1a3412'], 14, 1.4, 0.3, 0.8); }, { seed: 107 }),
};

// Water: tiling normal map from summed sines; caustic light net (voronoi edges)
export function waterNormal() {
  const n = 256, c = document.createElement('canvas'); c.width = c.height = n;
  const g = c.getContext('2d'), im = g.createImageData(n, n);
  const h = (x, y) => {
    const u = x / n * Math.PI * 2, v = y / n * Math.PI * 2;
    return Math.sin(u * 3 + v * 2) * 0.5 + Math.sin(u * 5 - v * 4 + 1.3) * 0.35 + Math.sin(u * 8 + v * 7 + 2.1) * 0.2 + Math.sin(-u * 11 + v * 9) * 0.12;
  };
  for (let y = 0; y < n; y++) for (let x = 0; x < n; x++) {
    const dx = h(x + 1, y) - h(x - 1, y), dy = h(x, y + 1) - h(x, y - 1), k = 0.9;
    const nx = -dx * k, ny = -dy * k, nz = 1, l = Math.hypot(nx, ny, nz), i = (y * n + x) * 4;
    im.data[i] = (nx / l * 0.5 + 0.5) * 255; im.data[i + 1] = (ny / l * 0.5 + 0.5) * 255; im.data[i + 2] = (nz / l * 0.5 + 0.5) * 255; im.data[i + 3] = 255;
  }
  g.putImageData(im, 0, 0);
  const t = new THREE.CanvasTexture(c); t.wrapS = t.wrapT = THREE.RepeatWrapping; t.anisotropy = 8; return t;
}
export function causticsTex() {
  const n = 256, c = document.createElement('canvas'); c.width = c.height = n;
  const g = c.getContext('2d'), im = g.createImageData(n, n), r = rng(5), pts = [];
  for (let i = 0; i < 28; i++) pts.push([r() * n, r() * n]);
  for (let y = 0; y < n; y++) for (let x = 0; x < n; x++) {
    let d1 = 1e9, d2 = 1e9;
    for (const [px, py] of pts) for (const ox of [-n, 0, n]) for (const oy of [-n, 0, n]) {
      const d = Math.hypot(x - px - ox, y - py - oy); if (d < d1) { d2 = d1; d1 = d; } else if (d < d2) d2 = d;
    }
    const e = Math.max(0, 1 - (d2 - d1) / 9), v = e * e * 255, i = (y * n + x) * 4;
    im.data[i] = v; im.data[i + 1] = v; im.data[i + 2] = v; im.data[i + 3] = 255;
  }
  g.putImageData(im, 0, 0);
  const t = new THREE.CanvasTexture(c); t.wrapS = t.wrapT = THREE.RepeatWrapping; return t;
}
export function labelTex(text, o = {}) {
  const c = document.createElement('canvas'); c.width = o.w || 512; c.height = o.h || 128;
  const g = c.getContext('2d'); g.fillStyle = o.bg || 'rgba(0,0,0,0)'; g.fillRect(0, 0, c.width, c.height);
  g.font = `bold ${o.size || 72}px Arial Black, Arial, sans-serif`; g.textAlign = 'center'; g.textBaseline = 'middle';
  g.shadowColor = o.glow || '#ff2a6a'; g.shadowBlur = o.blur ?? 18; g.fillStyle = o.fg || '#ffd6e6';
  g.fillText(text, c.width / 2, c.height / 2);
  const t = new THREE.CanvasTexture(c); t.colorSpace = THREE.SRGBColorSpace; return t;
}
export function frondTex() {
  const c = document.createElement('canvas'); c.width = 256; c.height = 64; const g = c.getContext('2d');
  g.strokeStyle = '#2a5a1e'; g.lineWidth = 3; g.beginPath(); g.moveTo(0, 32); g.lineTo(256, 32); g.stroke();
  const r = rng(3);
  for (let i = 0; i < 70; i++) {
    const x = 10 + i * 3.4, len = 26 * Math.sin(Math.PI * (i / 70)) + 6;
    for (const s of [-1, 1]) {
      g.strokeStyle = ['#2f6a22', '#3f7a2a', '#244e1a', '#4c8a30'][(r() * 4) | 0]; g.lineWidth = 2.2;
      g.beginPath(); g.moveTo(x, 32); g.lineTo(x + 7, 32 + s * len); g.stroke();
    }
  }
  const t = new THREE.CanvasTexture(c); t.colorSpace = THREE.SRGBColorSpace; return t;
}

// name: [texture, metres per tile, material props]
const SPEC = {
  plasterOchre: ['plasterOchre', 4, { roughness: 0.92, bumpScale: 1.2 }],
  plasterWhite: ['plasterWhite', 4, { roughness: 0.9, bumpScale: 1 }],
  plasterDark: ['plasterDark', 4, { roughness: 0.95, bumpScale: 1.4 }],
  concrete: ['concrete', 3, { roughness: 0.95, bumpScale: 1.5 }],
  blocks: ['blocks', 3.2, { roughness: 0.96, bumpScale: 2 }],
  tile: ['tileTerracotta', 1.6, { roughness: 0.55, bumpScale: 0.8 }],
  marble: ['marble', 2.4, { roughness: 0.18, metalness: 0.0, bumpScale: 0.2 }],
  wood: ['wood', 2.4, { roughness: 0.6, bumpScale: 0.6 }],
  crate: ['crate', 1.2, { roughness: 0.85, bumpScale: 1 }],
  poolTile: ['poolTile', 2, { roughness: 0.15, bumpScale: 0.5 }],
  deck: ['deck', 2.4, { roughness: 0.75, bumpScale: 1 }],
  roofTar: ['roofTar', 2.4, { roughness: 0.97, bumpScale: 1.4 }],
  grass: ['grass', 3, { roughness: 1, bumpScale: 2.2 }],
  dirt: ['dirt', 3, { roughness: 1, bumpScale: 2 }],
  gravel: ['gravel', 2.4, { roughness: 0.98, bumpScale: 2.5 }],
  asphalt: ['asphalt', 3, { roughness: 0.9, bumpScale: 1.4 }],
  metal: ['metal', 1.6, { roughness: 0.55, metalness: 0.7, bumpScale: 2 }],
  rust: ['rust', 1.5, { roughness: 0.8, metalness: 0.5, bumpScale: 1.2 }],
  carpet: ['carpet', 2, { roughness: 1, bumpScale: 1 }],
  fabric: ['fabric', 1, { roughness: 0.95, bumpScale: 0.6 }],
  paintRed: ['paintRed', 1, { roughness: 0.5, metalness: 0.4 }],
  paintGreen: ['paintGreen', 1, { roughness: 0.6, metalness: 0.2 }],
  darkMetal: ['darkMetal', 1, { roughness: 0.4, metalness: 0.8 }],
  bark: ['bark', 1, { roughness: 1, bumpScale: 2 }],
  leaf: ['leaf', 1, { roughness: 0.85 }],
};
export const SCALE = Object.fromEntries(Object.entries(SPEC).map(([k, v]) => [k, v[1]]));
const cache = {};
const texCache = {};
export function mat(name) {
  if (cache[name]) return cache[name];
  const s = SPEC[name]; if (!s) throw new Error('no material ' + name);
  const map = texCache[s[0]] || (texCache[s[0]] = T[s[0]]());
  const m = new THREE.MeshStandardMaterial({ map, bumpMap: map, ...s[2] });
  return (cache[name] = m);
}
export const plain = (color, o = {}) => new THREE.MeshStandardMaterial({ color, roughness: 0.7, ...o });
export const EXTRA = {
  glass: new THREE.MeshPhysicalMaterial({ color: 0xbfd8dc, roughness: 0.04, metalness: 0, transparent: true, opacity: 0.22, side: THREE.DoubleSide, depthWrite: false, envMapIntensity: 1.6 }),
  glassFloor: new THREE.MeshPhysicalMaterial({ color: 0xa8cbd0, roughness: 0.06, transparent: true, opacity: 0.30, side: THREE.DoubleSide, depthWrite: false, envMapIntensity: 1.4 }),
  frame: plain(0x1d1f20, { roughness: 0.45, metalness: 0.7 }),
  frameWhite: plain(0xe7e2d6, { roughness: 0.6 }),
  shutter: plain(0x2f5a3c, { roughness: 0.75 }),
  steel: plain(0x6b6f74, { roughness: 0.35, metalness: 0.9 }),
  rubber: plain(0x151515, { roughness: 0.9 }),
  white: plain(0xe8e8e6, { roughness: 0.6 }),
  black: plain(0x0c0c0d, { roughness: 0.5 }),
  red: plain(0xa01c1c, { roughness: 0.5, metalness: 0.2 }),
  redGlow: new THREE.MeshStandardMaterial({ color: 0xff2a2a, emissive: 0xff1a1a, emissiveIntensity: 1.2, roughness: 0.4 }),
  lamp: new THREE.MeshStandardMaterial({ color: 0xfff0c8, emissive: 0xffd9a0, emissiveIntensity: 1.5, roughness: 0.5 }),
  candle: new THREE.MeshStandardMaterial({ color: 0xf1e6cf, emissive: 0xffb050, emissiveIntensity: 0.5, roughness: 0.7 }),
  flame: new THREE.MeshBasicMaterial({ color: 0xffc060 }),
  vial: (c) => new THREE.MeshPhysicalMaterial({ color: c, emissive: c, emissiveIntensity: 0.6, transmission: 0, transparent: true, opacity: 0.85, roughness: 0.1 }),
  heliGreen: plain(0x3d4a3a, { roughness: 0.45, metalness: 0.5 }),
  heliGlass: new THREE.MeshPhysicalMaterial({ color: 0x88a0aa, roughness: 0.03, metalness: 0.1, transparent: true, opacity: 0.3, side: THREE.DoubleSide, depthWrite: false, envMapIntensity: 2 }),
};
