// Outdoor landscape: terrain with hills, the 60 x 60 m compound (wall, razor wire, gate, yard, lawn, house,
// pergola, sniper outbuilding, supply truck, fuel store, food shed, crates), grass blades, palms and trees,
// the road, the helipad and the helicopter with its cockpit. World axes: x east, z south (game north = -z), y up.
import * as THREE from 'three';
import { mergeGeometries } from 'three/addons/utils/BufferGeometryUtils.js';
import { Builder } from './build.js';
import { mat, plain, EXTRA, rng, labelTex, frondTex } from './mats.js';

// ---------------------------------------------------------------- noise and terrain height
const hash = (x, y) => { let h = Math.imul(x | 0, 374761393) + Math.imul(y | 0, 668265263); h = Math.imul(h ^ (h >>> 13), 1274126177); return ((h ^ (h >>> 16)) >>> 0) / 4294967296; };
const vnoise = (x, y) => {
  const xi = Math.floor(x), yi = Math.floor(y), xf = x - xi, yf = y - yi, u = xf * xf * (3 - 2 * xf), v = yf * yf * (3 - 2 * yf);
  return hash(xi, yi) * (1 - u) * (1 - v) + hash(xi + 1, yi) * u * (1 - v) + hash(xi, yi + 1) * (1 - u) * v + hash(xi + 1, yi + 1) * u * v;
};
const fbm = (x, y) => vnoise(x, y) * 0.5 + vnoise(x * 2.1, y * 2.1) * 0.25 + vnoise(x * 4.3, y * 4.3) * 0.125 + vnoise(x * 8.7, y * 8.7) * 0.0625;
const smooth = (a, b, x) => { const t = Math.min(1, Math.max(0, (x - a) / (b - a))); return t * t * (3 - 2 * t); };
export function terrainY(x, z) {
  const d = Math.hypot(x + 15, z + 25);
  const ridge = smooth(105, 300, d) * (16 + fbm(x * 0.01 + 3, z * 0.01) * 70);
  const micro = (fbm(x * 0.08, z * 0.08) - 0.4) * 0.5 * smooth(70, 130, d);
  // gentle shoulder next to the road so it does not float
  return ridge + micro;
}
export const inCompound = (x, z, m = 0) => x > -30.6 - m && x < 30.6 + m && z > -30.6 - m && z < 30.6 + m;
const inMansionZone = (x, z, m = 0) => x > -76 - m && x < 35 + m && z > -80 - m && z < -24 + m;

export function buildWorld() {
  const B = new Builder('SITE'), upd = [], R = rng(2025);
  const W = { builder: B, update: (t) => upd.forEach(f => f(t)), terrainY, extras: new THREE.Group(), tint: null };
  const ex = W.extras;

  // ----------------------------------------------------------- terrain
  {
    const N = 220, S = 760, g = new THREE.PlaneGeometry(S, S, N, N); g.rotateX(-Math.PI / 2); g.translate(-15, 0, -25);
    const p = g.attributes.position, uv = g.attributes.uv, col = new Float32Array(p.count * 3);
    for (let i = 0; i < p.count; i++) {
      const x = p.getX(i), z = p.getZ(i), y = terrainY(x, z); p.setY(i, y); uv.setXY(i, x / 3, z / 3);
      const n = fbm(x * 0.05, z * 0.05), dry = smooth(0.35, 0.65, fbm(x * 0.02 + 9, z * 0.02 + 4)), hill = smooth(0, 40, y);
      const gr = 0.82 + n * 0.3; col[i * 3] = gr * (0.95 + dry * 0.35 + hill * 0.1); col[i * 3 + 1] = gr * (1 - dry * 0.12); col[i * 3 + 2] = gr * (0.85 - dry * 0.25);
    }
    g.setAttribute('color', new THREE.BufferAttribute(col, 3)); g.computeVertexNormals();
    // cut the terrain out under the mansion and the maze wing (the pool basin goes below it)
    { const holes = [[-30.2, -70.2, 30.2, -29.8], [-70.6, -70.2, -30.4, -29.8]], idx = g.index.array, keep = [];
      const inH = (i) => holes.some(h => p.getX(i) > h[0] && p.getX(i) < h[2] && p.getZ(i) > h[1] && p.getZ(i) < h[3]);
      for (let i = 0; i < idx.length; i += 3) if (!(inH(idx[i]) && inH(idx[i + 1]) && inH(idx[i + 2]))) keep.push(idx[i], idx[i + 1], idx[i + 2]);
      g.setIndex(keep); }
    const m = new THREE.MeshStandardMaterial({ map: mat('grass').map, bumpMap: mat('grass').map, bumpScale: 3, roughness: 1, vertexColors: true });
    const ter = new THREE.Mesh(g, m); ter.receiveShadow = true; ter.name = 'terrain'; ex.add(ter);
  }

  // ----------------------------------------------------------- ground surfaces (thin slabs just above y = 0)
  B.bx('dirt', -30.3, 0, -30, 30.3, 0.03, 30.3);                       // the yard
  B.bx('gravel', -2.2, 0.03, -19, 2.2, 0.05, 30.6);                    // gravel path gate -> house
  B.bx('gravel', -9, 0.03, 11, 9, 0.05, 13);                           // cross path
  B.bx('concrete', -10, 0.03, -19.4, 10, 0.07, -14);                   // apron / terrace
  B.bx('asphalt', -3.6, 0.0, 30.3, 3.6, 0.04, 330);                    // the road south
  B.bx('dirt', -7, 0.0, 30.3, -3.6, 0.03, 330); B.bx('dirt', 3.6, 0.0, 30.3, 7, 0.03, 330);
  B.bx('concrete', -8, 0.0, 33, 8, 0.1, 47);                           // helipad
  B.bx('grass', -16, 0.03, -5, 24, 0.12, 17);                          // the lawn, raised kerb
  B.bx('concrete', -16.1, 0.03, -5.1, 24.1, 0.14, -4.9); B.bx('concrete', -16.1, 0.03, 16.9, 24.1, 0.14, 17.1);
  B.bx('concrete', -16.1, 0.03, -5.1, -15.9, 0.14, 17.1); B.bx('concrete', 23.9, 0.03, -5.1, 24.1, 0.14, 17.1);
  // road markings
  for (let z = 34; z < 320; z += 8) B.bx(EXTRA.white, -0.08, 0.04, z, 0.08, 0.05, z + 3.5);
  {
    const pad = new THREE.Mesh(new THREE.PlaneGeometry(10, 10), new THREE.MeshStandardMaterial({ map: padTex(), transparent: true, roughness: 0.8 }));
    pad.rotation.x = -Math.PI / 2; pad.position.set(0, 0.115, 40); pad.receiveShadow = true; ex.add(pad);
  }

  // ----------------------------------------------------------- perimeter wall, pillars, razor wire, gate
  const HW = 2.6, tw = 0.6;
  B.wall('blocks', -30.3, -30, -30.3, 30.3, tw, 0, HW, [], { mat: mat('plasterOchre'), t: 0.05, side: -1 });
  B.wall('blocks', 30.3, -30, 30.3, 30.3, tw, 0, HW, [], { mat: mat('plasterOchre'), t: 0.05, side: 1 });
  B.wall('blocks', -30.3, 30.3, 30.3, 30.3, tw, 0, HW, [{ c: 0, w: 4.8, kind: 'gap' }], { mat: mat('plasterOchre'), t: 0.05, side: 1 });
  for (const [px, pz] of [[-30.3, -30], [30.3, -30], [-30.3, 30.3], [30.3, 30.3], [-30.3, 10], [30.3, 10], [-30.3, -10], [30.3, -10], [-15, 30.3], [15, 30.3], [-2.6, 30.3], [2.6, 30.3]]) {
    B.solid('concrete', px - 0.4, 0, pz - 0.4, px + 0.4, HW + 0.25, pz + 0.4); B.bx('concrete', px - 0.5, HW + 0.25, pz - 0.5, px + 0.5, HW + 0.35, pz + 0.5);
  }
  {
    const wire = [], t = new THREE.TorusGeometry(0.2, 0.011, 5, 14);
    const place = (x, z, ry) => { const m = new THREE.Matrix4().compose(new THREE.Vector3(x, HW + 0.55, z), new THREE.Quaternion().setFromEuler(new THREE.Euler(0, ry + Math.PI / 2, 0)), new THREE.Vector3(1, 1, 1)); wire.push(m); };
    for (let i = -30; i <= 30; i += 0.28) { place(-30.3, i, 0); place(30.3, i, 0); if (Math.abs(i) > 2.5) place(i, 30.3, Math.PI / 2); }
    const im = new THREE.InstancedMesh(t, EXTRA.steel, wire.length); wire.forEach((m, i) => im.setMatrixAt(i, m)); im.castShadow = false; ex.add(im);
    for (const [x0, z0, x1, z1] of [[-30.3, -30, -30.3, 30.3], [30.3, -30, 30.3, 30.3]]) B.bx(EXTRA.steel, x0 - 0.01, HW + 0.52, z0, x1 + 0.01, HW + 0.56, z1);
  }
  // gate: two open leaves swung inwards
  for (const s of [-1, 1]) {
    const leaf = new THREE.Group();
    const frame = new THREE.Mesh(new THREE.BoxGeometry(2.3, 2.3, 0.07), EXTRA.darkMetalMat || plain(0x2a2d31, { metalness: 0.8, roughness: 0.4 })); leaf.add(frame);
    for (let i = 0; i < 12; i++) { const b = new THREE.Mesh(new THREE.BoxGeometry(0.04, 2.1, 0.05), EXTRA.steel); b.position.x = -1.05 + i * 0.19; leaf.add(b); }
    leaf.position.set(s * 2.4, 1.2, 30.3 - 1.2); leaf.rotation.y = s * 1.15; leaf.children.forEach(c => { c.castShadow = true; }); ex.add(leaf);
  }
  // corner floodlight towers
  for (const [x, z] of [[-27, 27], [27, 27], [-27, -27], [27, -27]]) {
    B.solid(EXTRA.steel, x - 0.12, 0, z - 0.12, x + 0.12, 6.5, z + 0.12); B.bx(EXTRA.lamp, x - 0.45, 6.4, z - 0.2, x + 0.45, 6.55, z + 0.2); B.bx(EXTRA.steel, x - 0.5, 6.55, z - 0.25, x + 0.5, 6.62, z + 0.25);
    B.lights.push({ x, y: 6.2, z, color: 0xffe2b0, intensity: 90, dist: 40, tag: 'night' });
  }

  // ----------------------------------------------------------- the low brown house, pergola, sign
  const brown = mat('plasterOchre').clone(); brown.color = new THREE.Color(0xa88868); brown.userData.uvScale = 4; W.tint = brown;
  const HX0 = -9, HX1 = 9, HZ0 = -26.5, HZ1 = -19.5, HH = 3.6, HT = 0.4;
  B.slab('concrete', HX0, HZ0, HX1, HZ1, -0.2, 0.15);
  B.floor('tile', HX0 + 0.2, HZ0 + 0.2, HX1 - 0.2, HZ1 - 0.2, 0.15);
  const sk = (side) => ({ mat: brown, t: 0.05, side });
  B.wall('plasterWhite', HX0, HZ0, HX0, HZ1, HT, 0.15, HH, [], sk(-1));
  B.wall('plasterWhite', HX1, HZ0, HX1, HZ1, HT, 0.15, HH, [], sk(1));
  B.wall('plasterWhite', HX0, HZ0, HX1, HZ0, HT, 0.15, HH, [{ c: 0, w: 1.6, head: 2.4, kind: 'door' }], sk(-1));
  B.wall('plasterWhite', HX0, HZ1, HX1, HZ1, HT, 0.15, HH, [
    { c: -6.5, w: 3.2, sill: 0.9, head: 2.4, kind: 'window', shutters: 1 }, { c: 0, w: 1.6, head: 2.4, kind: 'gate' }, { c: 6.5, w: 3.2, sill: 0.9, head: 2.4, kind: 'window', shutters: 1 }], sk(1));
  B.slab('concrete', HX0 - 1, HZ0 - 1.2, HX1 + 1, HZ1 + 1.2, HH, HH + 0.35);
  B.bx('concrete', HX0 - 1, HH + 0.35, HZ0 - 1.2, HX1 + 1, HH + 0.6, HZ0 - 1.1); B.bx('concrete', HX0 - 1, HH + 0.35, HZ1 + 1.1, HX1 + 1, HH + 0.6, HZ1 + 1.2);
  B.bx('concrete', HX0 - 1, HH + 0.35, HZ0 - 1.2, HX0 - 0.9, HH + 0.6, HZ1 + 1.2); B.bx('concrete', HX1 + 0.9, HH + 0.35, HZ0 - 1.2, HX1 + 1, HH + 0.6, HZ1 + 1.2);
  B.light(0, 2.7, -23, 0xffc890, 30, 12);
  // lobby furniture
  B.solid('wood', -7, 0.15, -25.6, 1, 1.1, -24.8); B.solid('fabric', 4, 0.15, -25.8, 8, 0.55, -24.6); B.bx('fabric', 4, 0.55, -26, 8, 1.0, -25.8);
  B.solid('wood', -8.4, 0.15, -23.4, -7.4, 1.0, -21); B.solid('crate', 5.4, 0.15, -23, 6.4, 1.0, -22);
  // pergola
  for (let x = -8; x <= 8; x += 4) for (const z of [-17.3]) { B.solid('wood', x - 0.12, 0, z - 0.12, x + 0.12, 3.1, z + 0.12); }
  B.bx('wood', -8.3, 3.1, -17.45, 8.3, 3.3, -17.15); B.bx('wood', -8.3, 3.1, -19.7, 8.3, 3.3, -19.4);
  for (let x = -8.2; x <= 8.2; x += 0.55) B.bx('wood', x - 0.05, 3.3, -19.7, x + 0.05, 3.4, -17.15);
  { // string lights
    const bulbs = new THREE.SphereGeometry(0.06, 6, 5), pts = [];
    for (const z of [-19.55, -18.5, -17.3]) for (let x = -8.2; x <= 8.2; x += 0.45) pts.push(new THREE.Vector3(x, 3.08 - Math.sin(((x + 8.2) % 4) / 4 * Math.PI) * -0.0 - 0.16 * Math.sin(((x + 8.2) % 4) / 4 * Math.PI), z));
    const im = new THREE.InstancedMesh(bulbs, new THREE.MeshBasicMaterial({ color: 0xffe2a0 }), pts.length);
    pts.forEach((p, i) => im.setMatrixAt(i, new THREE.Matrix4().setPosition(p))); ex.add(im); W.stringLights = im;
    B.lights.push({ x: -5, y: 2.7, z: -18.5, color: 0xffd596, intensity: 30, dist: 11, tag: 'night' }, { x: 5, y: 2.7, z: -18.5, color: 0xffd596, intensity: 30, dist: 11, tag: 'night' });
  }
  { // neon OPEN sign + hanging sign
    const sign = new THREE.Mesh(new THREE.PlaneGeometry(1.5, 0.45), new THREE.MeshBasicMaterial({ map: labelTex('OPEN', { w: 384, h: 128, size: 90 }), transparent: true, blending: THREE.AdditiveBlending, depthWrite: false }));
    sign.position.set(-6.5, 2.75, -19.27); sign.rotation.y = 0; ex.add(sign); W.neon = sign;
    B.lights.push({ x: -6.5, y: 2.5, z: -18.3, color: 0xff2a6a, intensity: 18, dist: 8, tag: 'night' });
  }

  // ----------------------------------------------------------- sniper outbuilding with sandbag nest
  B.slab('concrete', -27.8, -14.3, -22.2, -9.7, -0.2, 0.15);
  const ob = (a, b, c, d, o = []) => B.wall('blocks', a, b, c, d, 0.3, 0.15, 3.0, o);
  ob(-27.5, -14, -22.5, -14); ob(-27.5, -10, -22.5, -10, [{ c: -25, w: 1.0, head: 2.1, kind: 'door' }]); ob(-27.5, -14, -27.5, -10); ob(-22.5, -14, -22.5, -10, [{ c: -12, w: 0.8, sill: 1.2, head: 2.0, kind: 'window' }]);
  B.slab('concrete', -27.9, -14.4, -22.1, -9.6, 3.0, 3.25);
  for (let i = 0; i < 12; i++) { const a = i < 6 ? -27.6 + i * 0.9 : -22.4; const z = i < 6 ? -14.3 : -14.3 + (i - 5) * 0.75; B.solid('crate', a, 3.25, z, a + 0.8, 3.6, z + 0.35); }
  B.solid('concrete', -27.6, 3.25, -14.3, -22.4, 3.9, -14.1);
  for (let i = 0; i < 5; i++) { const y = 0.15 + i * 0.35; B.solid('wood', -22.1 + 0, y, -13.4 + i * 0.02, -21.9, y + 0.3, -12.3 + i * 0.02); }   // ladder-side rack
  B.solid('steel' in {} ? 'steel' : EXTRA.steel, -22.1, 0.15, -13.2, -22.0, 3.1, -13.1); B.solid(EXTRA.steel, -22.1, 0.15, -12.4, -22.0, 3.1, -12.3);
  for (let y = 0.4; y < 3.0; y += 0.35) B.bx(EXTRA.steel, -22.1, y, -13.2, -22.0, y + 0.03, -12.3);
  B.light(-25, 2.2, -12, 0xffb070, 12, 7);

  // ----------------------------------------------------------- supply truck (flat rear tyre)
  {
    const t = new THREE.Group(), body = EXTRA.heliGreen;
    const add = (g, m, x, y, z) => { const o = new THREE.Mesh(g, m); o.position.set(x, y, z); o.castShadow = o.receiveShadow = true; t.add(o); return o; };
    add(new THREE.BoxGeometry(2.4, 0.3, 7.4), EXTRA.black, 0, 0.9, -0.3);
    add(new THREE.BoxGeometry(2.3, 2.5, 4.6), mat('paintGreen'), 0, 2.3, 1.2);              // cargo box
    add(new THREE.BoxGeometry(2.2, 1.7, 1.9), mat('paintGreen'), 0, 1.9, -2.7);             // cab
    add(new THREE.BoxGeometry(2.0, 0.9, 0.06), EXTRA.heliGlass, 0, 2.25, -3.68).rotation.x = -0.25;
    add(new THREE.BoxGeometry(2.3, 0.35, 0.4), EXTRA.steel, 0, 0.65, -3.8);
    for (const x of [-0.8, 0.8]) add(new THREE.BoxGeometry(0.35, 0.2, 0.08), EXTRA.lamp, x, 1.0, -3.86);
    const tyre = new THREE.CylinderGeometry(0.55, 0.55, 0.38, 20), rim = new THREE.CylinderGeometry(0.28, 0.28, 0.4, 12);
    [[-1.2, -2.5], [1.2, -2.5], [-1.2, 0.9], [1.2, 0.9], [-1.2, 2.2], [1.2, 2.2]].forEach(([x, z], i) => {
      const flat = i === 5, w = add(tyre, EXTRA.rubber, x, flat ? 0.4 : 0.55, z); w.rotation.z = Math.PI / 2; if (flat) w.scale.set(0.72, 1, 1.0);
      const r = add(rim, EXTRA.steel, x, flat ? 0.4 : 0.55, z); r.rotation.z = Math.PI / 2;
    });
    t.position.set(-16, 0, -1.8); t.rotation.y = 0.04; ex.add(t);
    B.collide(-17.3, -5.7, -14.7, 1.5, 0, 3.5);
  }
  // ----------------------------------------------------------- fuel store: canopy, drums, pipes
  for (const [x, z] of [[14.5, -12], [22, -12], [14.5, -5], [22, -5]]) B.solid(EXTRA.steel, x - 0.1, 0, z - 0.1, x + 0.1, 3.0, z + 0.1);
  B.slab('metal', 14, -12.3, 22.5, -4.7, 3.0, 3.1, [], false);
  B.bx('concrete', 14.2, 0.0, -12, 22.3, 0.12, -5.2);
  for (const [x, z, c] of [[18, -8, 'rust'], [19, -8.9, 'paintRed'], [17.8, -9.3, 'rust'], [19.2, -7.6, 'paintGreen'], [16.5, -10.5, 'rust'], [20.6, -10.2, 'darkMetal']]) {
    B.box(mat(c), x, 0.57, z, 0.9, 0.9, 0.9);                                    // squared drums read fine at this scale
    const d = new THREE.Mesh(new THREE.CylinderGeometry(0.42, 0.42, 0.9, 16), mat(c)); d.position.set(x, 0.57 + 0.45, z); d.castShadow = true; ex.add(d); B.collide(x - 0.45, z - 0.45, x + 0.45, z + 0.45, 0, 1.2);
    const rb = new THREE.Mesh(new THREE.TorusGeometry(0.42, 0.025, 6, 20), EXTRA.steel); rb.position.set(x, 1.02, z); rb.rotation.x = Math.PI / 2; ex.add(rb);
  }
  B.bx(EXTRA.steel, 15.5, 0.6, -6.4, 20, 0.7, -6.25); B.solid(EXTRA.redGlow, 16.8, 0.15, -6.9, 17.2, 0.9, -6.5);
  B.light(18, 2.7, -8, 0xffd0a0, 25, 10, 'night');
  // ----------------------------------------------------------- food store
  B.slab('concrete', 11.2, 15.45, 16.8, 19.55, -0.2, 0.12);
  B.wall('blocks', 11.5, 15.75, 16.5, 15.75, 0.3, 0.12, 2.8, [{ c: 14, w: 1.2, head: 2.1, kind: 'door' }]);
  B.wall('blocks', 11.5, 19.25, 16.5, 19.25, 0.3, 0.12, 2.8); B.wall('blocks', 11.5, 15.75, 11.5, 19.25, 0.3, 0.12, 2.8);
  B.wall('blocks', 16.5, 15.75, 16.5, 19.25, 0.3, 0.12, 2.8, [{ c: 17.5, w: 0.7, sill: 1.4, head: 2.0, kind: 'window' }]);
  { const r = new THREE.Mesh(new THREE.BoxGeometry(6.2, 0.1, 4.6), mat('metal')); r.position.set(14, 2.95, 17.5); r.rotation.x = 0.1; r.castShadow = true; ex.add(r); }
  B.solid('crate', 11.4, 0.12, 15.8, 12.4, 0.9, 16.8); B.solid('crate', 12.5, 0.12, 15.8, 13.2, 0.7, 16.5); B.solid('crate', 15.0, 0.12, 17.8, 16.2, 1.0, 19.0);
  B.light(14, 2.2, 17.5, 0xffe0b0, 10, 6);
  // ----------------------------------------------------------- crates and sandbags in the yard
  for (const [x, z, sx, sz] of [[-4, 16, 1.6, 1.6], [-2.2, 17.2, 1.6, 1.6], [8, 6, 1.8, 1.2], [-9, 9, 1.4, 1.4], [5, -8, 1.4, 1.4], [-12, -10, 1.6, 1.2], [22, 2, 1.6, 1.6], [-3.3, 16.4, 1.2, 1.2]]) {
    const h = 1.1 + ((x * 7 + z * 3) % 3) * 0.1; B.solid('crate', x - sx / 2, 0.03, z - sz / 2, x + sx / 2, 0.03 + h, z + sz / 2);
  }
  B.solid('crate', -3.2, 1.13, 15.6, -2.2, 2.2, 16.6);

  // ----------------------------------------------------------- bushes
  const bushG = (() => { const g = new THREE.IcosahedronGeometry(1, 2), p = g.attributes.position; for (let i = 0; i < p.count; i++) { const k = 0.8 + vnoise(p.getX(i) * 2.2 + 5, p.getY(i) * 2.2 + p.getZ(i) * 2.0) * 0.45; p.setXYZ(i, p.getX(i) * k, p.getY(i) * k * 0.8, p.getZ(i) * k); } g.computeVertexNormals(); return g; })();
  const leafM = new THREE.MeshStandardMaterial({ map: mat('leaf').map, bumpMap: mat('leaf').map, bumpScale: 2, roughness: 0.9, color: 0x9ab87a });
  const bush = (x, z, r) => { const b = new THREE.Mesh(bushG, leafM); b.position.set(x, r * 0.55, z); b.scale.setScalar(r); b.rotation.y = R() * 6; b.castShadow = b.receiveShadow = true; ex.add(b); };
  for (const [x, z] of [[-3, 8], [10, -1], [-8, -12], [2, -10], [22, -18], [-22, 12], [4, 20]]) bush(x, z, 1.0);
  for (let x = -26; x <= 26; x += 3.4) { bush(x + R(), 28.2 + R() * 0.4, 0.55 + R() * 0.3); }
  for (let z = -26; z <= 26; z += 4.2) { bush(-28.2 + R() * 0.4, z, 0.5 + R() * 0.3); bush(28.2 + R() * 0.4, z + 1, 0.5 + R() * 0.3); }
  for (const x of [-7.5, -2.5, 2.5, 7.5]) bush(x, -14.4, 0.4);

  // ----------------------------------------------------------- grass blades
  {
    const bg = new THREE.BufferGeometry();
    const v = [-0.03, 0, 0, 0.03, 0, 0, -0.022, 0.55, 0.0, 0.022, 0.55, 0.0, 0, 1, 0]; const idx = [0, 1, 2, 1, 3, 2, 2, 3, 4];
    const c = [0.06, 0.12, 0.03, 0.06, 0.12, 0.03, 0.2, 0.34, 0.1, 0.2, 0.34, 0.1, 0.5, 0.66, 0.22];
    bg.setAttribute('position', new THREE.Float32BufferAttribute(v, 3)); bg.setAttribute('color', new THREE.Float32BufferAttribute(c, 3));
    bg.setAttribute('normal', new THREE.Float32BufferAttribute([0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0], 3)); bg.setIndex(idx);
    const gm = new THREE.MeshStandardMaterial({ vertexColors: true, side: THREE.DoubleSide, roughness: 0.9 });
    const uTime = { value: 0 };
    gm.onBeforeCompile = (sh) => {
      sh.uniforms.uTime = uTime;
      sh.vertexShader = 'uniform float uTime;\n' + sh.vertexShader.replace('#include <begin_vertex>',
        `vec3 transformed = vec3(position);
         float ph = instanceMatrix[3].x * 0.7 + instanceMatrix[3].z * 0.5;
         transformed.x += (sin(uTime * 1.7 + ph) * 0.5 + 0.5) * 0.28 * position.y * position.y;
         transformed.z += cos(uTime * 1.3 + ph * 1.3) * 0.12 * position.y * position.y;`);
    };
    upd.push((t) => { uTime.value = t; });
    const spots = [];   // [x0,z0,x1,z1,density per m2, hMin,hMax]
    spots.push([-16, -5, 24, 17, 46, 0.35, 0.7]);                                      // lawn
    spots.push([-29.5, -29.5, -10, -20, 6, 0.2, 0.45], [10, -29.5, 29.5, -20, 6, 0.2, 0.45], [-29.5, 18, -9, 29.5, 5, 0.2, 0.4], [9, 18, 29.5, 29.5, 5, 0.2, 0.4]);
    spots.push([-26, -20, -10, -5, 4, 0.15, 0.35], [-4, 17, 4, 29.5, 3, 0.15, 0.3]);
    spots.push([32, -76, 62, -24, 3.2, 0.3, 0.8], [-110, -26, -76, 30, 2.4, 0.3, 0.8], [32, -24, 70, 28, 2.4, 0.3, 0.8], [-76, -26, -32, 30, 2.8, 0.3, 0.8], [-76, -120, 35, -76, 2, 0.3, 0.8], [32, 28, 90, 90, 1.4, 0.35, 0.9], [-90, 28, -32, 90, 1.4, 0.35, 0.9]);
    let total = 0; for (const s of spots) total += (s[2] - s[0]) * (s[3] - s[1]) * s[4];
    const im = new THREE.InstancedMesh(bg, gm, Math.round(total)); const m4 = new THREE.Matrix4(), q = new THREE.Quaternion(), e = new THREE.Euler(), col = new THREE.Color();
    let k = 0;
    for (const [x0, z0, x1, z1, d, h0, h1] of spots) {
      const n = Math.round((x1 - x0) * (z1 - z0) * d);
      for (let i = 0; i < n && k < im.count; i++) {
        const x = x0 + R() * (x1 - x0), z = z0 + R() * (z1 - z0);
        if (inMansionZone(x, z, 0) && !(x < -32 || x > 32)) continue;
        if (Math.abs(x) < 7.5 && z > 30) continue; // road
        if (x > -10.5 && x < 10.5 && z > -27 && z < -14 && inCompound(x, z)) continue;
        e.set((R() - 0.5) * 0.4, R() * 6.28, (R() - 0.5) * 0.4); q.setFromEuler(e);
        const h = h0 + R() * (h1 - h0), y = terrainY(x, z) + (x >= -16 && x <= 24 && z >= -5 && z <= 17 ? 0.12 : 0.02);
        m4.compose(new THREE.Vector3(x, y, z), q, new THREE.Vector3(1 + R() * 0.8, h, 1));
        im.setMatrixAt(k, m4); col.setHSL(0.24 + R() * 0.06, 0.4 + R() * 0.2, 0.6 + R() * 0.4); im.setColorAt(k, col); k++;
      }
    }
    im.count = k; im.castShadow = false; im.receiveShadow = true; im.frustumCulled = false; ex.add(im);
  }

  // ----------------------------------------------------------- trees (instanced palms and broadleaf)
  {
    const noise = (x, y, z) => vnoise(x * 1.3 + z, y * 1.3 - x);
    // palm
    const trunk = new THREE.CylinderGeometry(0.17, 0.3, 9, 8, 14, true); trunk.translate(0, 4.5, 0);
    { const p = trunk.attributes.position, uv = trunk.attributes.uv; for (let i = 0; i < p.count; i++) { const y = p.getY(i), t = y / 9; p.setX(i, p.getX(i) + t * t * 1.6); uv.setXY(i, uv.getX(i) * 2, y * 0.35); } trunk.computeVertexNormals(); }
    const fr = []; const top = new THREE.Vector3(1.6, 9, 0);
    for (let i = 0; i < 15; i++) {
      const f = new THREE.PlaneGeometry(4.6, 1.2, 8, 1); f.translate(2.3, 0, 0); const p = f.attributes.position;
      for (let j = 0; j < p.count; j++) { const x = p.getX(j); p.setY(j, p.getY(j) - Math.pow(x / 4.6, 2) * (1.4 + (i % 3) * 0.4)); }
      f.rotateX(-Math.PI / 2 + (i % 2) * 0.3); f.rotateZ(0.5 - (i % 3) * 0.18); f.rotateY((i / 15) * Math.PI * 2 + (i % 2) * 0.4); f.translate(top.x, top.y, top.z); fr.push(f);
    }
    const fronds = mergeGeometries(fr);
    const palmFrondMat = new THREE.MeshStandardMaterial({ map: frondTex(), alphaTest: 0.5, side: THREE.DoubleSide, roughness: 0.8, color: 0xb8d8a0 });
    const palmTrunkMat = new THREE.MeshStandardMaterial({ map: mat('bark').map, bumpMap: mat('bark').map, bumpScale: 2, roughness: 1, color: 0xcfc2a8 });
    // broadleaf (mango / almond)
    const tt = new THREE.CylinderGeometry(0.28, 0.5, 4.2, 8, 3, true); tt.translate(0, 2.1, 0);
    const blobs = [];
    for (let i = 0; i < 7; i++) {
      const b = new THREE.IcosahedronGeometry(1.9 + R() * 1.1, 2), p = b.attributes.position;
      for (let j = 0; j < p.count; j++) { const k = 0.82 + noise(p.getX(j) * 1.7, p.getY(j) * 1.7, p.getZ(j) * 1.7) * 0.4; p.setXYZ(j, p.getX(j) * k, p.getY(j) * k * 0.78, p.getZ(j) * k); }
      b.translate((R() - 0.5) * 3.4, 5.0 + R() * 2.2, (R() - 0.5) * 3.4); blobs.push(b);
    }
    const crown = mergeGeometries(blobs); crown.computeVertexNormals();
    { const p = crown.attributes.position, uv = crown.attributes.uv; for (let i = 0; i < p.count; i++) uv.setXY(i, p.getX(i) * 0.4 + p.getZ(i) * 0.3, p.getY(i) * 0.4); }
    const crownMat = new THREE.MeshStandardMaterial({ map: mat('leaf').map, bumpMap: mat('leaf').map, bumpScale: 3, roughness: 0.9, color: 0xb0c890 });
    const palms = [], broads = [];
    const free = (x, z) => !inCompound(x, z, 6) && !inMansionZone(x, z, 4) && !(Math.abs(x) < 12 && z > 20) && !(Math.abs(x + 15) < 22 && z > 24 && z < 56 && false);
    let guard = 0;
    while ((palms.length < 90 || broads.length < 70) && guard++ < 6000) {
      const a = R() * 6.283, r = 18 + Math.pow(R(), 0.8) * 230, x = -15 + Math.cos(a) * r * 1.2, z = -25 + Math.sin(a) * r;
      if (!free(x, z)) continue;
      const rec = { p: new THREE.Vector3(x, terrainY(x, z) - 0.1, z), s: 0.8 + R() * 0.7, ry: R() * 6.28 };
      if (R() < 0.55 && palms.length < 90) palms.push(rec); else if (broads.length < 70) broads.push(rec);
    }
    // a few palms inside the compound corners and behind the house
    for (const [x, z] of [[-26, 24], [26, 24], [-27, 4], [27, -22], [-27, -20]]) palms.push({ p: new THREE.Vector3(x, 0, z), s: 0.85 + R() * 0.2, ry: R() * 6.28 });
    const inst = (geo, m, list, shadow) => {
      const im = new THREE.InstancedMesh(geo, m, list.length), m4 = new THREE.Matrix4(), q = new THREE.Quaternion();
      list.forEach((t, i) => { q.setFromEuler(new THREE.Euler((R() - 0.5) * 0.06, t.ry, (R() - 0.5) * 0.06)); m4.compose(t.p, q, new THREE.Vector3(t.s, t.s, t.s)); im.setMatrixAt(i, m4); });
      im.castShadow = shadow; im.receiveShadow = true; im.frustumCulled = false; ex.add(im); return im;
    };
    inst(trunk, palmTrunkMat, palms, true); const pf = inst(fronds, palmFrondMat, palms, true); W.palms = pf;
    inst(tt, palmTrunkMat, broads, true); inst(crown, crownMat, broads, true);
    for (const t of palms.slice(-5)) B.collide(t.p.x - 0.4, t.p.z - 0.4, t.p.x + 0.4, t.p.z + 0.4, 0, 9);
  }

  // ----------------------------------------------------------- distant houses on the hills
  {
    const boxG = new THREE.BoxGeometry(1, 1, 1), cols = [0xd8b48a, 0x9ec0c8, 0xd9908a, 0xe3d28a, 0xb0b0a6, 0xa8c08c, 0xcf8f58];
    const list = [];
    for (let i = 0; i < 400 && list.length < 70; i++) {
      const a = R() * 6.283, r = 190 + R() * 130, x = -15 + Math.cos(a) * r, z = -25 + Math.sin(a) * r, y = terrainY(x, z);
      if (y < 4 || Math.abs(x) < 20 && z > 0) continue; list.push([x, y, z, 4 + R() * 5, 2.8 + R() * 1.5, 4 + R() * 4, cols[(R() * cols.length) | 0]]);
    }
    const im = new THREE.InstancedMesh(boxG, new THREE.MeshStandardMaterial({ map: mat('plasterWhite').map, roughness: 0.95 }), list.length), m4 = new THREE.Matrix4(), c = new THREE.Color();
    list.forEach(([x, y, z, w, h, d, col], i) => { m4.compose(new THREE.Vector3(x, y + h / 2 - 0.4, z), new THREE.Quaternion().setFromEuler(new THREE.Euler(0, R() * 6, 0)), new THREE.Vector3(w, h, d)); im.setMatrixAt(i, m4); im.setColorAt(i, c.setHex(col)); });
    im.castShadow = true; ex.add(im);
  }

  // ----------------------------------------------------------- helicopter
  const heli = buildHeli(upd); heli.position.set(0, 0.12, 40); heli.rotation.y = 0; ex.add(heli); W.heli = heli;
  W.cockpitEye = { pos: new THREE.Vector3(0.5, 2.42, 40.15) };
  W.stats = { palms: 0 };
  return W;
}

function padTex() {
  const c = document.createElement('canvas'); c.width = c.height = 512; const g = c.getContext('2d');
  g.strokeStyle = '#e9e6dc'; g.lineWidth = 14; g.beginPath(); g.arc(256, 256, 226, 0, 7); g.stroke();
  g.fillStyle = '#e9e6dc'; g.font = 'bold 300px Arial'; g.textAlign = 'center'; g.textBaseline = 'middle'; g.fillText('H', 256, 270);
  g.strokeStyle = '#c9a23a'; g.lineWidth = 6; g.strokeRect(10, 10, 492, 492);
  const t = new THREE.CanvasTexture(c); t.colorSpace = THREE.SRGBColorSpace; t.anisotropy = 8; return t;
}
function panelTex() {
  const c = document.createElement('canvas'); c.width = 512; c.height = 256; const g = c.getContext('2d');
  g.fillStyle = '#0d1012'; g.fillRect(0, 0, 512, 256);
  const dial = (x, y, r, a) => { g.strokeStyle = '#9fd6c0'; g.lineWidth = 3; g.beginPath(); g.arc(x, y, r, 0, 7); g.stroke(); g.strokeStyle = '#ffb04a'; g.lineWidth = 3; g.beginPath(); g.moveTo(x, y); g.lineTo(x + Math.cos(a) * r * 0.8, y + Math.sin(a) * r * 0.8); g.stroke(); };
  dial(70, 70, 46, -1); dial(180, 70, 46, -2.2); dial(290, 70, 46, -0.2); dial(400, 70, 46, -2.8);
  g.fillStyle = '#12303a'; g.fillRect(24, 140, 190, 96); g.strokeStyle = '#4fd0c0'; g.strokeRect(24, 140, 190, 96);
  g.strokeStyle = '#4fd0c0'; g.beginPath(); g.moveTo(24, 188); g.lineTo(214, 188); g.moveTo(119, 140); g.lineTo(119, 236); g.stroke();
  g.fillStyle = '#ff4a3a'; g.beginPath(); g.arc(150, 170, 5, 0, 7); g.fill(); g.beginPath(); g.arc(88, 205, 5, 0, 7); g.fill();
  g.font = 'bold 22px Arial'; g.fillStyle = '#e8e8e8'; g.textAlign = 'center';
  for (let i = 0; i < 4; i++) { g.fillStyle = '#8c1a18'; g.fillRect(240 + i * 62, 150, 52, 52); g.fillStyle = '#fff'; g.fillText(String(i + 1), 266 + i * 62, 184); }
  g.fillStyle = '#e8b030'; g.fillRect(240, 212, 248, 28); g.fillStyle = '#222'; g.fillText('MASTER ARM', 364, 233);
  const t = new THREE.CanvasTexture(c); t.colorSpace = THREE.SRGBColorSpace; return t;
}

function buildHeli(upd) {
  const H = new THREE.Group(); H.name = 'helicopter';
  const body = new THREE.MeshStandardMaterial({ color: 0x5a6e52, roughness: 0.5, metalness: 0.2 });
  const belly = new THREE.MeshStandardMaterial({ color: 0x2f3638, roughness: 0.55, metalness: 0.25 });
  const add = (g, m, x, y, z, parent = H) => { const o = new THREE.Mesh(g, m); o.position.set(x, y, z); o.castShadow = o.receiveShadow = true; parent.add(o); return o; };
  // fuselage (nose toward -z)
  const fus = add(new THREE.SphereGeometry(1, 28, 20), body, 0, 1.95, 0); fus.scale.set(1.12, 1.15, 2.7);
  const under = add(new THREE.SphereGeometry(1, 20, 12, 0, 6.283, 1.5, 1.6), belly, 0, 1.95, 0); under.scale.set(1.14, 1.17, 2.72);
  const canopy = add(new THREE.SphereGeometry(1, 28, 16, 0, 6.283, 0, 1.5), EXTRA.heliGlass, 0, 2.0, -0.55); canopy.scale.set(1.0, 1.0, 1.75); canopy.renderOrder = 6;
  // tail boom, fin, stabilizer, tail rotor
  const boom = add(new THREE.CylinderGeometry(0.12, 0.42, 6.4, 12), body, 0, 2.45, 5.1); boom.rotation.x = Math.PI / 2 - 0.05;
  add(new THREE.BoxGeometry(0.08, 1.5, 1.1), body, 0, 3.05, 7.9).rotation.x = -0.25;
  add(new THREE.BoxGeometry(1.7, 0.06, 0.6), body, 0, 2.3, 7.5);
  const tail = new THREE.Group(); tail.position.set(0.14, 3.4, 8.1); H.add(tail);
  for (let i = 0; i < 4; i++) { const b = add(new THREE.BoxGeometry(0.04, 1.4, 0.14), belly, 0, 0, 0, tail); b.rotation.x = i * Math.PI / 2; b.position.y = 0; b.geometry.translate(0, 0.7, 0); }
  // skids
  for (const s of [-1, 1]) {
    const sk = add(new THREE.CylinderGeometry(0.05, 0.05, 4.4, 8), EXTRA.steel, s * 1.15, 0.22, 0.3); sk.rotation.x = Math.PI / 2;
    for (const z of [-1.0, 1.4]) { const st = add(new THREE.CylinderGeometry(0.04, 0.04, 1.2, 6), EXTRA.steel, s * 1.05, 0.8, z); st.rotation.z = s * 0.18; }
  }
  // stub wings and rocket pods (4 strike lines)
  for (const s of [-1, 1]) {
    add(new THREE.BoxGeometry(1.5, 0.08, 0.7), body, s * 1.55, 1.55, 0.2);
    const pod = add(new THREE.CylinderGeometry(0.28, 0.28, 1.9, 14), belly, s * 2.1, 1.35, -0.1); pod.rotation.x = Math.PI / 2;
    for (let i = 0; i < 7; i++) { const a = i === 0 ? 0 : (i - 1) / 6 * 6.283, r = i === 0 ? 0 : 0.17; add(new THREE.CylinderGeometry(0.045, 0.045, 0.1, 8), EXTRA.black, s * 2.1 + Math.cos(a) * r, 1.35 + Math.sin(a) * r, -1.07).rotation.x = Math.PI / 2; }
  }
  // main rotor
  add(new THREE.CylinderGeometry(0.1, 0.14, 0.7, 10), belly, 0, 3.35, 0.1);
  const rotor = new THREE.Group(); rotor.position.set(0, 3.75, 0.1); H.add(rotor);
  for (let i = 0; i < 4; i++) { const b = add(new THREE.BoxGeometry(6.3, 0.035, 0.3), belly, 0, 0, 0, rotor); b.geometry.translate(3.15, 0, 0); b.rotation.y = i * Math.PI / 2; b.rotation.z = 0.04; }
  const blur = add(new THREE.CircleGeometry(6.3, 40), new THREE.MeshBasicMaterial({ color: 0xcfd6d4, transparent: true, opacity: 0.07, side: THREE.DoubleSide, depthWrite: false }), 0, 3.78, 0.1); blur.rotation.x = -Math.PI / 2; blur.castShadow = false;
  upd.push((t) => { rotor.rotation.y = -t * 7; tail.rotation.x = t * 12; blur.material.opacity = 0.06 + 0.02 * Math.sin(t * 3); });
  // beacons
  const bc = new THREE.Mesh(new THREE.SphereGeometry(0.1, 8, 6), EXTRA.redGlow); bc.position.set(0, 3.08, 2.2); H.add(bc);
  const nav = new THREE.Mesh(new THREE.SphereGeometry(0.06, 8, 6), new THREE.MeshBasicMaterial({ color: 0x35ff6a })); nav.position.set(2.55, 1.55, 0.2); H.add(nav);
  const navR = new THREE.Mesh(new THREE.SphereGeometry(0.06, 8, 6), new THREE.MeshBasicMaterial({ color: 0xff3030 })); navR.position.set(-2.55, 1.55, 0.2); H.add(navR);
  upd.push((t) => { bc.visible = Math.sin(t * 5) > 0.2; });

  // cockpit interior (the hero sits in the right seat; the left seat is empty)
  const dash = add(new THREE.BoxGeometry(1.9, 0.5, 0.35), belly, 0, 1.78, -1.62); dash.rotation.x = -0.35;
  const pan = new THREE.Mesh(new THREE.PlaneGeometry(1.7, 0.42), new THREE.MeshBasicMaterial({ map: panelTex() })); pan.position.set(0, 1.86, -1.43); pan.rotation.x = -0.35; H.add(pan);
  const floor = add(new THREE.BoxGeometry(2.1, 0.06, 2.8), belly, 0, 1.0, -0.2);
  for (const s of [-1, 1]) {
    const seat = new THREE.Group(); seat.position.set(s * 0.5, 1.03, 0.25); H.add(seat);
    add(new THREE.BoxGeometry(0.55, 0.14, 0.55), mat('fabric'), 0, 0.28, 0, seat); const bk = add(new THREE.BoxGeometry(0.55, 0.85, 0.12), mat('fabric'), 0, 0.72, 0.32, seat); bk.rotation.x = 0.12;
    add(new THREE.BoxGeometry(0.5, 0.18, 0.1), belly, 0, 1.2, 0.36, seat);
    add(new THREE.CylinderGeometry(0.02, 0.02, 0.4, 6), EXTRA.steel, 0, 0.1, 0, seat);
  }
  // cyclic stick, collective lever, strike panel on the console
  add(new THREE.CylinderGeometry(0.018, 0.022, 0.55, 8), belly, 0.5, 1.28, -0.5);
  add(new THREE.SphereGeometry(0.03, 8, 8), EXTRA.black, 0.5, 1.56, -0.5);
  const col = add(new THREE.CylinderGeometry(0.02, 0.02, 0.6, 6), belly, 0.9, 1.25, 0.0); col.rotation.z = 0.9;
  add(new THREE.BoxGeometry(0.34, 0.07, 0.36), belly, 0.78, 1.34, -0.45).rotation.x = -0.1;
  for (let i = 0; i < 4; i++) add(new THREE.BoxGeometry(0.05, 0.03, 0.05), EXTRA.redGlow, 0.68 + i * 0.07, 1.39, -0.55);
  // the hero: seated figure in SWAT gear (right seat)
  const hero = new THREE.Group(); hero.position.set(0.5, 1.3, 0.2); H.add(hero);
  const gear = new THREE.MeshStandardMaterial({ color: 0x1b2022, roughness: 0.7 }), skin = new THREE.MeshStandardMaterial({ color: 0x5a3a26, roughness: 0.8 });
  add(new THREE.BoxGeometry(0.46, 0.55, 0.28), gear, 0, 0.55, 0.0, hero);            // torso with vest
  add(new THREE.BoxGeometry(0.4, 0.2, 0.3), gear, 0, 0.15, -0.0, hero);
  for (const s of [-1, 1]) { const th = add(new THREE.BoxGeometry(0.16, 0.16, 0.5), gear, s * 0.12, 0.2, -0.28, hero); const sh = add(new THREE.BoxGeometry(0.14, 0.14, 0.5), gear, s * 0.12, -0.1, -0.52, hero); sh.rotation.x = 0.0; }
  const hd = add(new THREE.SphereGeometry(0.15, 14, 12), skin, 0, 1.0, -0.02, hero);
  const helm = add(new THREE.SphereGeometry(0.19, 14, 10, 0, 6.283, 0, 1.7), gear, 0, 1.03, -0.02, hero);
  add(new THREE.BoxGeometry(0.3, 0.06, 0.14), EXTRA.glass, 0, 0.99, -0.16, hero);
  for (const s of [-1, 1]) { const arm = add(new THREE.BoxGeometry(0.11, 0.11, 0.55), gear, s * 0.3, 0.55, -0.28, hero); arm.rotation.x = s > 0 ? -0.5 : -0.2; arm.position.y = s > 0 ? 0.5 : 0.45; }
  // interior light
  const l = new THREE.PointLight(0x9fe8ff, 0.4, 3); l.position.set(0, 2.5, -0.9); H.add(l); H.userData.cockpitLight = l;
  return H;
}
