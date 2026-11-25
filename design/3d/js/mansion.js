// The mansion, level by level, from the OURCAD drawing numbers (game metres). Local plan coordinates (x east,
// z south, game north = -z) are shifted into the world: the 60 x 40 m party-wing footprint is centred on
// (0, -50) behind the compound, the 40 x 40 m maze wing sits on its west side. Reconciled proposals are marked.
import * as THREE from 'three';
import { Builder } from './build.js';
import { mat, plain, EXTRA, labelTex, causticsTex, waterNormal, rng } from './mats.js';

const OZ = -50, MZX = -50.5;                                // mansion centre z, maze wing centre x
export const LV = { GF: 0, F1: 6.0, F2: 9.8, F3: 13.6, ROOF: 17.4 };
export const CEIL = { GF: 5.6, F1: 9.4, F2: 13.2, F3: 17.0 };
export const POOL = { x0: -9, z0: -12, x1: 9, z1: 12, depth: 3.0 };       // pool = glass floor outline (issue 2 settled)
const SR = { x0: 24, x1: 30, zin: -19.75, zs: -13.5, zw: -12.25 };        // the NE stair core (reconciled)
const ochre = { mat: mat('plasterOchre'), t: 0.06 };

const C = {
  sofa: plain(0x5a2f2a, { roughness: 0.9 }), leather: plain(0x2a1c16, { roughness: 0.5 }), cream: plain(0xcbbfa8, { roughness: 0.9 }),
  blue: plain(0x2d4a6a, { roughness: 0.8 }), wicker: plain(0x9b7a4a, { roughness: 0.95 }), glassB: new THREE.MeshPhysicalMaterial({ color: 0x9fd0d8, transparent: true, opacity: 0.5, roughness: 0.05 }),
  bone: plain(0xd9d2bc, { roughness: 0.9 }), stone: plain(0x77756f, { roughness: 0.95 }), black: EXTRA.black, gold: plain(0xc9a24a, { metalness: 0.8, roughness: 0.35 }),
  pink: plain(0xc96a8a, { roughness: 0.8 }), mirror: new THREE.MeshStandardMaterial({ color: 0xcfd8dc, metalness: 1, roughness: 0.05 }),
  green: plain(0x3d6a3a, { roughness: 0.9 }), bottle: (c) => new THREE.MeshPhysicalMaterial({ color: c, emissive: c, emissiveIntensity: 0.25, transparent: true, opacity: 0.85, roughness: 0.1 }),
};
C.stone.userData.uvScale = 1;

class Loc {
  constructor(B, ox, oz) { this.B = B; this.ox = ox; this.oz = oz; }
  wall(m, x1, z1, x2, z2, t, y0, y1, ops = [], skin = null) {
    const v = Math.abs(x1 - x2) < 1e-6;
    this.B.wall(m, x1 + this.ox, z1 + this.oz, x2 + this.ox, z2 + this.oz, t, y0, y1, ops.map(o => ({ ...o, c: o.c + (v ? this.oz : this.ox) })), skin);
  }
  bx(m, x0, y0, z0, x1, y1, z1) { this.B.bx(m, x0 + this.ox, y0, z0 + this.oz, x1 + this.ox, y1, z1 + this.oz); }
  solid(m, x0, y0, z0, x1, y1, z1) { this.B.solid(m, x0 + this.ox, y0, z0 + this.oz, x1 + this.ox, y1, z1 + this.oz); }
  // furniture block centred at (cx,cz) size (w x d), from y to y+h
  fur(m, cx, cz, w, d, y, h, solid = true) { (solid ? this.solid : this.bx).call(this, m, cx - w / 2, y, cz - d / 2, cx + w / 2, y + h, cz + d / 2); }
  slab(m, x0, z0, x1, z1, y0, y1, holes = [], walk = true) { this.B.slab(m, x0 + this.ox, z0 + this.oz, x1 + this.ox, z1 + this.oz, y0, y1, holes.map(h => [h[0] + this.ox, h[1] + this.oz, h[2] + this.ox, h[3] + this.oz]), walk); }
  floor(m, x0, z0, x1, z1, y, holes = []) { this.slab(m, x0, z0, x1, z1, y, y + 0.03, holes, false); }
  light(x, y, z, color, intensity, dist, tag = 'int', flick = 0) { this.B.lights.push({ x: x + this.ox, y, z: z + this.oz, color, intensity, dist, tag, flick }); }
  lamp(x, y, z, color = 0xffd9a8, intensity = 60, dist = 14) { this.bx(EXTRA.lamp, x - 0.3, y, z - 0.15, x + 0.3, y + 0.06, z + 0.15); this.light(x, y - 0.3, z, color, intensity, dist); }
  candle(x, y, z, flick = true) { this.bx(EXTRA.candle, x - 0.03, y, z - 0.03, x + 0.03, y + 0.2, z + 0.03); this.bx(EXTRA.flame, x - 0.015, y + 0.2, z - 0.015, x + 0.015, y + 0.26, z + 0.015); }
  mesh(g, m, x, y, z, cast = true) { const o = new THREE.Mesh(g, m); o.position.set(x + this.ox, y, z + this.oz); o.castShadow = cast; o.receiveShadow = true; this.B.extra.push(o); return o; }
  // steps: ascent direction along z (dir -1 = north)
  flightZ(x0, x1, zStart, dir, n, rise, tread, yBase, m = 'concrete') {
    for (let i = 1; i <= n; i++) {
      const top = yBase + i * rise, za = zStart + dir * (i - 1) * tread, zb = zStart + dir * i * tread;
      this.bx(m, x0, top - 0.28, Math.min(za, zb), x1, top, Math.max(za, zb));
    }
    this.B.surfaces.push({ kind: 'steps', ax: 'z', c0: x0 + this.ox, c1: x1 + this.ox, start: zStart + this.oz, dir, n, r: rise, tread, y0: yBase });
    this.bx(EXTRA.steel, x0, yBase + 0.9, Math.min(zStart, zStart + dir * n * tread), x0 + 0.04, yBase + 1.0, Math.max(zStart, zStart + dir * n * tread));
  }
}

function perimeterBand(L, y, h, off, x0 = -30.25, x1 = 30.25, z0 = -20.25, z1 = 20.25, m = 'concrete') {
  L.bx(m, x0 - off, y, z0 - off, x1 + off, y + h, z0); L.bx(m, x0 - off, y, z1, x1 + off, y + h, z1 + off);
  L.bx(m, x0 - off, y, z0, x0, y + h, z1); L.bx(m, x1, y, z0, x1 + off, y + h, z1);
}
function winList(a, b, step, avoid, wd) { const out = []; for (let c = a + step / 2; c < b; c += step) if (!avoid.some(v => Math.abs(v - c) < wd / 2 + 0.85)) out.push(c); return out; }
// four exterior walls of a floor. spec: {y0,y1,sill,head,wd,step,shut,hx0,hx1,hz0,hz1, S:{avoid,ops}, N, E, W}
function exterior(L, s) {
  const hx = s.hx ?? 30, hz = s.hz ?? 20, yl = s.y0, yh = s.y1;
  const mk = (side, len, spec = {}) => {
    const ops = (spec.ops || []).slice();
    for (const c of winList(-len / 2, len / 2, s.step || 6, [...(spec.avoid || []), ...ops.map(o => o.c)], s.wd)) ops.push({ c, w: s.wd, sill: s.sill, head: s.head, kind: s.noWin ? 'gap0' : 'window', shutters: s.shut ? side : 0, frame: s.frame });
    return ops.filter(o => o.kind !== 'gap0');
  };
  const sk = (side) => ({ ...ochre, side });
  L.wall('plasterWhite', -hx, -hz, hx, -hz, 0.5, yl, yh, mk(-1, hx * 2, s.N), sk(-1));
  L.wall('plasterWhite', -hx, hz, hx, hz, 0.5, yl, yh, mk(1, hx * 2, s.S), sk(1));
  L.wall('plasterWhite', -hx, -hz, -hx, hz, 0.5, yl, yh, mk(-1, hz * 2, s.W), sk(-1));
  L.wall('plasterWhite', hx, -hz, hx, hz, 0.5, yl, yh, mk(1, hz * 2, s.E), sk(1));
}
function stairRoomWalls(L, y0, y1, gate = true) {
  L.wall('plasterWhite', SR.x0, -20, SR.x0, SR.zw, 0.5, y0, y1);
  L.wall('plasterWhite', SR.x0, SR.zw, SR.x1, SR.zw, 0.5, y0, y1, [{ c: 25.55, w: 1.6, head: 2.4, kind: gate ? 'gate' : 'door' }]);
}
// U stair from yBase up by `rise` inside the core
function stairCore(L, yBase, rise, n, tread) {
  const r = rise / (2 * n), zA = SR.zs - n * tread;
  L.flightZ(24.35, 26.75, SR.zs, -1, n, r, tread, yBase);
  L.slab('concrete', 24.35, SR.zin, 29.65, zA, yBase + n * r - 0.28, yBase + n * r, [], false);
  L.B.surfaces.push({ kind: 'slab', x0: 24.35, z0: SR.zin + OZ, x1: 29.65, z1: zA + OZ, y: yBase + n * r, holes: [] });
  L.flightZ(27.35, 29.65, zA, 1, n, r, tread, yBase + n * r);
}
const holeStair = [SR.x0 + 0.25, SR.zin, SR.x1 - 0.25, SR.zs];

export function buildMansion() {
  const out = { levels: {}, meta: { secrets: {}, water: null, flicker: [] } };

  // =============================================================================================
  //  GF-A: the maze wing (Level 2)   40 x 40 m, 4 x 4 rooms of 10 m, walls 0.5 m
  // =============================================================================================
  {
    const B = new Builder('GF-A'), L = new Loc(B, MZX, OZ), H = 4.0;
    const OPEN2 = [[[3, 2], [3, 3]], [[3, 2], [2, 2]], [[2, 2], [2, 1]], [[2, 2], [1, 2]], [[2, 1], [2, 0]], [[2, 1], [3, 1]], [[3, 1], [3, 0]], [[1, 2], [1, 3]], [[1, 2], [0, 2]], [[1, 2], [1, 1]], [[1, 1], [0, 1]], [[0, 2], [0, 1]], [[0, 2], [0, 3]], [[1, 1], [1, 0]], [[2, 0], [1, 0]], [[1, 3], [2, 3]], [[0, 0], [1, 0]]];
    const open2 = (a, b) => OPEN2.some(([x, y]) => (x[0] === a[0] && x[1] === a[1] && y[0] === b[0] && y[1] === b[1]) || (x[0] === b[0] && x[1] === b[1] && y[0] === a[0] && y[1] === a[1]));
    const cell = (r, c) => [-15 + 10 * c, -15 + 10 * r];
    L.slab('concrete', -20.3, -20.3, 20.3, 20.3, -0.4, 0);
    L.floor('wood', -19.7, -19.7, 19.7, 19.7, 0);
    const sk = (side) => ({ mat: mat('concrete'), t: 0.08, side });
    const slit = (len) => { const o = []; for (let c = -len / 2 + 5; c < len / 2; c += 10) o.push({ c, w: 0.9, sill: 3.1, head: 3.7, kind: 'window', frame: EXTRA.frame }); return o; };
    L.wall('plasterDark', -20, -20, 20, -20, 0.5, 0, H, [{ c: -5, w: 2.0, head: 2.5, kind: 'gate' }, ...slit(40).filter(o => Math.abs(o.c + 5) > 2)], sk(-1));
    L.wall('plasterDark', -20, 20, 20, 20, 0.5, 0, H, [{ c: 5, w: 2.4, head: 2.6, kind: 'door' }, ...slit(40).filter(o => Math.abs(o.c - 5) > 2.5)], sk(1));
    L.wall('plasterDark', -20, -20, -20, 20, 0.5, 0, H, slit(40), sk(-1));
    L.wall('plasterDark', 20, -20, 20, 20, 0.5, 0, H, slit(40), sk(1));
    for (let k = 1; k <= 3; k++) {
      const v = [], h = [];
      for (let r = 0; r < 4; r++) if (open2([r, k - 1], [r, k])) v.push({ c: -15 + 10 * r, w: 2.4, head: 2.6, kind: 'door' });
      for (let c = 0; c < 4; c++) if (open2([k - 1, c], [k, c])) h.push({ c: -15 + 10 * c, w: 2.4, head: 2.6, kind: 'door' });
      L.wall('plasterDark', -20 + 10 * k, -20, -20 + 10 * k, 20, 0.5, 0, H, v);
      L.wall('plasterDark', -20, -20 + 10 * k, 20, -20 + 10 * k, 0.5, 0, H, h);
    }
    // roof of the wing, parapet, ventilation
    L.slab('plasterDark', -20.3, -20.3, 20.3, 20.3, H, H + 0.34, [], false); L.slab('roofTar', -20.3, -20.3, 20.3, 20.3, H + 0.34, H + 0.4, [], true);
    perimeterBand(L, H + 0.4, 0.8, 0, -20.3, 20.3, -20.3, 20.3, 'concrete');
    L.bx('concrete', -20.5, H + 1.2, -20.5, 20.5, H + 1.3, -20.3); L.bx('concrete', -20.5, H + 1.2, 20.3, 20.5, H + 1.3, 20.5);
    L.bx('concrete', -20.5, H + 1.2, -20.3, -20.3, H + 1.3, 20.3); L.bx('concrete', 20.3, H + 1.2, -20.3, 20.5, H + 1.3, 20.3);
    // furniture: tables, shelves, candle clusters
    const fl = [];
    for (let r = 0; r < 4; r++) for (let c = 0; c < 4; c++) {
      const [cx, cz] = cell(r, c);
      if ((r + c) % 2 === 0 && !(r === 0 && c === 1)) { L.fur('wood', cx + 1.5, cz - 1.2, 2.0, 0.9, 0, 0.78); L.bx(EXTRA.candle, cx + 1.4, 0.78, cz - 1.3, cx + 1.46, 0.98, cz - 1.24); L.bx(EXTRA.flame, cx + 1.41, 0.98, cz - 1.29, cx + 1.45, 1.04, cz - 1.25); }
      // wall candles
      for (const [dx, dz] of [[-4.6, -4.6], [4.6, 4.6]]) { L.bx(EXTRA.candle, cx + dx - 0.04, 1.4, cz + dz - 0.04, cx + dx + 0.04, 1.7, cz + dz + 0.04); L.bx(EXTRA.flame, cx + dx - 0.02, 1.7, cz + dz - 0.02, cx + dx + 0.02, 1.78, cz + dz + 0.02); }
      L.light(cx + 0.5, 1.6, cz + 0.5, 0xff9a40, 26, 11, 'candle', 1);
    }
    for (const [r, c] of [[1, 3], [1, 1]]) { const [cx, cz] = cell(r, c); L.fur('wood', cx, cz - 4.4, 7.0, 0.6, 0, 2.2); }
    L.fur('carpet', 5, 14, 5, 3, 0.0, 0.05, false);
    // skull hall (1,1) and bone room (1,3): bone piles
    for (const [r, c] of [[1, 1], [1, 3]]) { const [cx, cz] = cell(r, c); for (let i = 0; i < 9; i++) { L.bx(C.bone, cx - 3 + i * 0.7, 0, cz + 2, cx - 2.7 + i * 0.7, 0.2 + (i % 3) * 0.08, cz + 2.3); } for (let i = 0; i < 6; i++) { const s = L.mesh(new THREE.SphereGeometry(0.11, 8, 6), C.bone, cx - 1.2 + i * 0.5, 0.9, cz - 4.2); } }
    // the altar room (0,1)
    L.fur(C.stone, -5, -16.6, 5.4, 1.2, 0, 1.0); L.fur('wood', -5, -16.6, 5.6, 1.3, 1.0, 0.06, false);
    for (const x of [-8.2, -1.8]) { L.bx(EXTRA.candle, x - 0.07, 1.06, -16.7, x + 0.07, 1.6, -16.5); L.bx(EXTRA.flame, x - 0.03, 1.6, -16.63, x + 0.03, 1.72, -16.57); }
    L.light(-5, 1.9, -15.5, 0xff8a40, 40, 10, 'candle', 1);
    [[-6.4, 0x58c26a, 'cure'], [-5, 0x7a3ab0, 'poison'], [-3.6, 0xe0a040, 'sense']].forEach(([x, col]) => {
      L.mesh(new THREE.CylinderGeometry(0.07, 0.09, 0.3, 12), C.bottle(col), x, 1.21, -16.6, false); L.mesh(new THREE.CylinderGeometry(0.03, 0.03, 0.12, 8), C.bottle(col), x, 1.42, -16.6, false);
      L.light(x, 1.5, -16.4, col, 3, 1.6, 'candle', 0);
    });
    L.mesh(new THREE.SphereGeometry(0.13, 10, 8), C.bone, -7.2, 1.13, -16.6); L.mesh(new THREE.SphereGeometry(0.13, 10, 8), C.bone, -2.7, 1.13, -16.6);
    // altar exit: a corridor outside (north), runs east to the party wing locker room door (reconciled link)
    const ex = -5;                                                     // door centre (local x)
    const wx = ex + MZX;                                              // -55.5 world
    const cz0 = -70, cz1 = -73.4;
    const lc = new Loc(B, 0, 0);
    lc.slab('concrete', wx - 1.4, cz1, -26.2, cz0, -0.4, 0);          // floor
    lc.floor('tile', wx - 1.4, cz1, -26.2, cz0, 0);
    lc.slab('roofTar', wx - 1.5, cz1 - 0.2, -26.1, cz0, 3.0, 3.35, [], false);
    lc.wall('plasterDark', wx - 1.4, cz1, -26.2, cz1, 0.3, 0, 3.0, [], { mat: mat('plasterOchre'), t: 0.04, side: -1 });
    lc.wall('plasterDark', wx - 1.4, cz1, wx - 1.4, cz0, 0.3, 0, 3.0); lc.wall('plasterDark', wx + 1.4, cz1, wx + 1.4, cz0, 0.3, 0, 3.0);
    lc.wall('plasterDark', -26.2, cz1, -26.2, cz0, 0.3, 0, 3.0);
    lc.light(wx, 2.6, -71.8, 0xffd0a0, 25, 9); lc.light(-32, 2.6, -71.8, 0xffd0a0, 25, 9);
    L.B.lights.push({ x: 0, y: 0, z: 0, color: 0, intensity: 0, dist: 1, tag: 'int' }); B.lights.pop();
    // front corridor (covered walk) from the house to the maze foyer door: built in GF-B (world coordinates)
    B.group.userData.foyerDoor = new THREE.Vector3(MZX + 5, 0, OZ + 20);
    // secret panel markers (4 per room, 1.1 m inside each wall)
    const pts = [];
    for (let r = 0; r < 4; r++) for (let c = 0; c < 4; c++) { const [cx, cz] = cell(r, c); for (const [dx, dz] of [[0, -3.9], [0, 3.9], [-3.9, 0], [3.9, 0]]) pts.push([cx + dx + MZX, 1.3, cz + dz + OZ]); }
    out.meta.secrets['GF-A'] = pts;
    B.surfaces.push({ kind: 'slab', x0: -26.2, z0: cz1, x1: -26.2 + 0, z1: cz0, y: 0, holes: [] });
    out.levels['GF-A'] = B;
  }

  // =============================================================================================
  //  GF-B: party wing (Level 3)   60 x 40 m, 5.6 m clear. Pool hall in the middle (RECONCILED: x +-12).
  // =============================================================================================
  {
    const B = new Builder('GF-B'), L = new Loc(B, 0, OZ), H = CEIL.GF;
    const pool = [POOL.x0, POOL.z0, POOL.x1, POOL.z1];
    L.slab('concrete', -30.3, -20.3, 30.3, 20.3, -0.4, 0, [pool]);
    L.floor('concrete', -29.7, -19.7, -12.2, 0, 0); L.floor('wood', -29.7, 0.3, -12.2, 19.7, 0);
    L.floor('deck', -11.8, -19.7, 11.8, 19.7, 0, [pool]);
    L.floor('marble', 12.3, -12.0, 29.7, 7.7, 0); L.floor('tile', 12.3, 8.3, 29.7, 19.7, 0); L.floor('concrete', 12.3, -19.7, 23.7, -12.5, 0); L.floor('concrete', 24.3, -19.7, 29.7, -12.5, 0);
    // ---- exterior
    exterior(L, {
      y0: 0, y1: H, sill: 0.9, head: 4.4, wd: 1.8, step: 6, shut: true,
      S: { ops: [{ c: 0, w: 2.8, head: 3.2, kind: 'door' }], avoid: [-12, 12, 0] },
      N: { ops: [{ c: -27, w: 1.4, head: 2.5, kind: 'door' }], avoid: [-12, 12, 24, -27] },
      W: { avoid: [0, -14, 10] }, E: { avoid: [-14, -2, 14, 8, -12.25] },
    });
    perimeterBand(L, 0, 0.5, 0.12); perimeterBand(L, 5.35, 0.25, 0.14);
    // ---- interior
    L.wall('plasterWhite', -12, -20, -12, 20, 0.5, 0, H, [{ c: -14, w: 2.6, kind: 'door' }, { c: 10, w: 2.6, kind: 'door' }]);
    L.wall('plasterWhite', 12, -20, 12, 20, 0.5, 0, H, [{ c: -14, w: 2.6, kind: 'door' }, { c: -2, w: 2.6, kind: 'door' }, { c: 14, w: 2.6, kind: 'door' }]);
    L.wall('plasterWhite', -30, 0, -12, 0, 0.5, 0, H, [{ c: -24, w: 2.6, kind: 'door' }]);
    L.wall('plasterWhite', 12, 8, 30, 8, 0.5, 0, H, [{ c: 15, w: 2.6, kind: 'door' }, { c: 24, w: 2.6, kind: 'door' }]);
    stairRoomWalls(L, 0, H, true);
    // ---- pool: basin, water, caustics, steps in the NW corner
    const px0 = POOL.x0, pz0 = POOL.z0, px1 = POOL.x1, pz1 = POOL.z1, d = POOL.depth;
    L.bx('concrete', px0 - 0.4, -d - 0.5, pz0 - 0.4, px1 + 0.4, -d, pz1 + 0.4);
    L.bx('poolTile', px0, -d, pz0, px1, -d + 0.02, pz1);
    for (const [a, b, c, e] of [[px0 - 0.4, pz0 - 0.4, px0, pz1 + 0.4], [px1, pz0 - 0.4, px1 + 0.4, pz1 + 0.4], [px0, pz0 - 0.4, px1, pz0], [px0, pz1, px1, pz1 + 0.4]]) L.bx('concrete', a, -d, b, c, 0, e);
    L.bx('poolTile', px0, -d, pz0, px0 + 0.03, -0.35, pz1); L.bx('poolTile', px1 - 0.03, -d, pz0, px1, -0.35, pz1);
    L.bx('poolTile', px0, -d, pz0, px1, -0.35, pz0 + 0.03); L.bx('poolTile', px0, -d, pz1 - 0.03, px1, -0.35, pz1);
    L.bx('marble', px0 - 0.4, -0.04, pz0 - 0.4, px0, 0.06, pz1 + 0.4); L.bx('marble', px1, -0.04, pz0 - 0.4, px1 + 0.4, 0.06, pz1 + 0.4);
    L.bx('marble', px0, -0.04, pz0 - 0.4, px1, 0.06, pz0); L.bx('marble', px0, -0.04, pz1, px1, 0.06, pz1 + 0.4);
    // lane lines on the bottom
    for (let i = -6; i <= 6; i += 3) L.bx(EXTRA.black, i - 0.08, -d + 0.02, pz0 + 0.5, i + 0.08, -d + 0.03, pz1 - 0.5);
    // pit stairs (out of the pool) along the west wall at the north end
    { const n = 12, r = d / n; for (let i = 1; i <= n; i++) { L.bx('poolTile', px0, -d + (i - 1) * r - 0.01, pz0, px0 + (n - i + 1) * 0.28, -d + i * r, pz0 + 2.4); }
      B.surfaces.push({ kind: 'steps', ax: 'x', c0: pz0 + OZ, c1: pz0 + 2.4 + OZ, start: px0 + 3.36, dir: -1, n, r, tread: 0.28, y0: -d }); }
    B.surfaces.push({ kind: 'slab', x0: px0, z0: pz0 + OZ, x1: px1, z1: pz1 + OZ, y: -d + 0.02, holes: [] });
    const water = new THREE.Mesh(new THREE.PlaneGeometry(px1 - px0, pz1 - pz0), new THREE.MeshPhysicalMaterial({
      color: 0x1f8fa6, roughness: 0.04, metalness: 0, transparent: true, opacity: 0.74, normalMap: waterNormal(), normalScale: new THREE.Vector2(0.5, 0.5), envMapIntensity: 1.8, clearcoat: 0.4 }));
    water.rotation.x = -Math.PI / 2; water.position.set(0, -0.28, OZ); water.material.normalMap.repeat.set(5, 6); water.renderOrder = 4; water.receiveShadow = true; B.extra.push(water);
    const caus = new THREE.Mesh(new THREE.PlaneGeometry(px1 - px0, pz1 - pz0), new THREE.MeshBasicMaterial({ map: causticsTex(), blending: THREE.AdditiveBlending, transparent: true, opacity: 0.35, depthWrite: false }));
    caus.material.map.repeat.set(5, 6); caus.rotation.x = -Math.PI / 2; caus.position.set(0, -d + 0.05, OZ); B.extra.push(caus);
    out.meta.water = { water, caus };
    L.light(0, -1.5, 0, 0x40d0e0, 40, 16, 'int');
    // ---- atrium: bar, loungers, planters
    L.fur(C.leather, 0, -18, 14, 0.9, 0, 1.1); L.fur('marble', 0, -18, 14.3, 1.1, 1.1, 0.06, false);
    for (let i = -6; i <= 6; i += 1.5) { L.fur('wood', i, -16.9, 0.4, 0.4, 0, 0.75); L.fur(C.leather, i, -16.9, 0.45, 0.45, 0.75, 0.08, false); }
    L.fur('wood', 0, -19.55, 14, 0.4, 0.9, 0.06, false); L.fur('wood', 0, -19.55, 14, 0.4, 1.7, 0.06, false);
    const bc = [0x2f7a3a, 0xa0601c, 0x7a1c2c, 0xd0b060, 0x2a4a8a];
    for (let i = -6.5; i <= 6.5; i += 0.4) { const col = bc[((i * 5) | 0) & 3]; L.mesh(new THREE.CylinderGeometry(0.05, 0.06, 0.3, 8), C.bottle(col), i, 1.05 + 0.0, -19.55, false).position.y = 1.07 + ((i * 3) | 0) % 2 * 0.8; }
    for (const x of [-9.5, -6, -2, 2, 6, 9.5]) { L.fur(C.wicker, x, 17.2, 0.8, 2.0, 0.0, 0.35); L.fur(C.cream, x, 17.0, 0.7, 1.5, 0.35, 0.1, false); }
    for (const [x, z] of [[-10.8, -18.6], [10.8, -18.6], [-10.8, 18.6], [10.8, 18.6]]) {
      L.fur(C.stone, x, z, 1.0, 1.0, 0, 0.7); L.mesh(new THREE.SphereGeometry(0.9, 12, 10), new THREE.MeshStandardMaterial({ map: mat('leaf').map, color: 0x9ab87a, roughness: 0.9 }), x, 1.7, z);
    }
    // chandeliers / ceiling lamps
    for (const [x, z] of [[-6, -10], [6, -10], [-6, 10], [6, 10], [0, 0]]) { L.bx(EXTRA.lamp, x - 0.6, 5.0, z - 0.6, x + 0.6, 5.08, z + 0.6); L.bx(EXTRA.steel, x - 0.02, 5.08, z - 0.02, x + 0.02, 5.6, z + 0.02); }
    L.light(-6, 4.7, -6, 0xffe2b8, 120, 22); L.light(6, 4.7, 6, 0xffe2b8, 120, 22);
    // ---- locker room and gym
    for (let i = 0; i < 6; i++) L.fur(C.blue, -28 + i * 1.7, -19.35, 1.4, 0.7, 0, 2.0);
    L.fur('wood', -24, -10, 4, 0.8, 0, 0.5); L.fur('wood', -24, -6, 4, 0.8, 0, 0.5);
    for (const z of [4, 8, 12]) { L.fur(C.black, -27.5, z, 1.0, 2.0, 0, 0.45); L.fur(EXTRA.steel, -27.5, z, 1.2, 0.1, 0.45, 1.2); }
    L.fur(C.leather, -22, 6, 2.2, 0.9, 0, 0.5); L.bx(C.mirror, -29.7, 0.8, 2, -29.68, 3.4, 18);
    for (const x of [-26, -20]) L.mesh(new THREE.CylinderGeometry(0.18, 0.18, 1.1, 12), C.leather, x, 3.4, 15, true);
    L.light(-21, 4.4, -10, 0xdde8ff, 70, 16); L.light(-21, 4.4, 10, 0xdde8ff, 70, 16);
    // ---- ballroom: dance floor, DJ booth, sofas, mirrorball, coloured lights
    L.fur(C.black, 17, -19, 6, 1.6, 0, 1.0); L.fur(C.black, 11.5 + 1, -18.8, 1.2, 1.2, 0, 1.8); L.fur(C.black, 22.5, -18.8, 1.2, 1.2, 0, 1.8);
    L.bx(EXTRA.redGlow, 14.5, 1.0, -19.5, 19.5, 1.04, -18.5);
    for (const z of [-6, -1]) L.fur(C.sofa, 28.4, z, 1.2, 3.0, 0, 0.6); L.fur(C.sofa, 11, 2, 3, 1.2, 0, 0.6);
    const ball = new THREE.Mesh(new THREE.SphereGeometry(0.45, 16, 12), C.mirror); ball.position.set(21, 4.7, -3 + OZ); B.extra.push(ball); { const wire = new THREE.Mesh(new THREE.CylinderGeometry(0.01, 0.01, 0.9), EXTRA.steel); wire.position.set(21, 5.3, -3 + OZ); B.extra.push(wire); }
    out.meta.ball = ball;
    L.light(17, 4.6, -4, 0xff2a90, 60, 16, 'int', 2); L.light(24, 4.6, 2, 0x2ab0ff, 60, 16, 'int', 2);
    // ---- veranda (SE)
    for (const x of [14, 19, 26]) { L.fur(C.wicker, x, 18.5, 2, 0.8, 0, 0.5); }
    for (const x of [16, 22]) { L.fur('wood', x, 14, 1.2, 1.2, 0, 0.75); L.fur(C.wicker, x - 1, 14, 0.6, 0.6, 0, 0.45); L.fur(C.wicker, x + 1, 14, 0.6, 0.6, 0, 0.45); }
    L.light(21, 4.7, 14, 0xffd9a0, 70, 14);
    // ---- the NE stair (reconciled core) up to F1
    stairCore(L, 0, 6.0, 15, 0.22);
    L.light(27, 5.0, -16.5, 0xffc890, 40, 8);
    // ---- covered walk: house lobby -> mansion door (x=0) and west to the maze foyer door (x=-45.5)
    const cl = new Loc(B, 0, 0);
    cl.slab('concrete', -47, -29.75, 3.6, -26.6, -0.4, 0); cl.floor('tile', -47, -29.75, 3.6, -26.6, 0);
    cl.slab('roofTar', -47.2, -30.0, 3.8, -26.4, 3.1, 3.4, [], false);
    cl.wall('plasterWhite', -46.9, -26.75, -9, -26.75, 0.3, 0, 3.1, [{ c: -20, w: 0.01, kind: 'gap' }].slice(0, 0), { mat: mat('plasterOchre'), t: 0.04, side: 1 });
    cl.wall('plasterWhite', 3.45, -29.75, 3.45, -26.5, 0.3, 0, 3.1);
    cl.wall('plasterWhite', -46.9, -29.75, -46.9, -26.6, 0.3, 0, 3.1);
    for (const x of [-40, -30, -20, -12]) { cl.bx(EXTRA.lamp, x - 0.4, 2.95, -28.4, x + 0.4, 3.0, -28.1); B.lights.push({ x, y: 2.6, z: -28.2, color: 0xffd0a0, intensity: 22, dist: 9, tag: 'int' }); }
    for (const x of [-38, -26, -16]) cl.bx(EXTRA.shutter, x - 0.6, 0.0, -26.8, x + 0.6, 1.0, -26.5);    // planters under the walk
    out.levels['GF-B'] = B;
  }

  // =============================================================================================
  //  F1: first floor (Level 4): penthouses and the glass floor over the pool
  // =============================================================================================
  {
    const B = new Builder('F1'), L = new Loc(B, 0, OZ), Y0 = LV.F1, Y1 = CEIL.F1;
    const gl = [POOL.x0, POOL.z0, POOL.x1, POOL.z1];
    L.slab('plasterWhite', -30.3, -20.3, 30.3, 20.3, Y0 - 0.4, Y0, [gl, holeStair]);
    L.floor('wood', -29.7, -19.7, -12.2, 19.7, Y0, []); L.floor('marble', -11.8, -19.7, 11.8, 19.7, Y0, [gl]); L.floor('wood', 12.3, -19.7, 29.7, 19.7, Y0, [holeStair]);
    // glass floor: steel grid below, a laminated pane on top
    for (let i = 0; i <= 6; i++) { const x = POOL.x0 + i * 3; L.bx(EXTRA.steel, x - 0.1, Y0 - 0.45, POOL.z0, x + 0.1, Y0 - 0.08, POOL.z1); }
    for (let i = 0; i <= 6; i++) { const z = POOL.z0 + i * 4; L.bx(EXTRA.steel, POOL.x0, Y0 - 0.38, z - 0.08, POOL.x1, Y0 - 0.08, z + 0.08); }
    const pane = new THREE.Mesh(new THREE.BoxGeometry(18, 0.1, 24), EXTRA.glassFloor); pane.position.set(0, Y0 - 0.05, OZ); pane.renderOrder = 6; B.extra.push(pane);
    for (let i = 0; i <= 6; i++) { L.bx(EXTRA.frame, POOL.x0 + i * 3 - 0.03, Y0 - 0.02, POOL.z0, POOL.x0 + i * 3 + 0.03, Y0 + 0.005, POOL.z1); }
    for (let i = 0; i <= 6; i++) { L.bx(EXTRA.frame, POOL.x0, Y0 - 0.02, POOL.z0 + i * 4 - 0.03, POOL.x1, Y0 + 0.005, POOL.z0 + i * 4 + 0.03); }
    B.surfaces.push({ kind: 'slab', x0: POOL.x0, z0: POOL.z0 + OZ, x1: POOL.x1, z1: POOL.z1 + OZ, y: Y0, holes: [] });
    out.meta.glass = pane;
    exterior(L, { y0: Y0, y1: Y1, sill: 0.9, head: 2.7, wd: 1.6, step: 5, shut: true, S: { avoid: [-12, 12, 0] }, N: { avoid: [-12, 12] }, W: { avoid: [0] }, E: { avoid: [0, -12.25, 24] } });
    perimeterBand(L, Y0 - 0.45, 0.45, 0.14); perimeterBand(L, Y1 - 0.2, 0.2, 0.16);
    L.wall('plasterWhite', -12, -14, -12, 14, 0.5, Y0, Y1, [-8, 0, 8].map(c => ({ c, w: 2.6, kind: 'door' })));
    L.wall('plasterWhite', 12, -14, 12, 14, 0.5, Y0, Y1, [-8, 0, 8].map(c => ({ c, w: 2.6, kind: 'door' })));
    L.wall('plasterWhite', -30, 0, -12, 0, 0.5, Y0, Y1, [{ c: -21, w: 2.6, kind: 'door' }]);
    L.wall('plasterWhite', 12, 0, 30, 0, 0.5, Y0, Y1, [{ c: 21, w: 2.6, kind: 'door' }]);
    stairRoomWalls(L, Y0, Y1, true);
    // west penthouse: lounge + bar + bedroom
    L.fur(C.sofa, -24, -10, 3.2, 1.2, Y0, 0.6); L.fur(C.sofa, -17, -6, 1.2, 3.2, Y0, 0.6); L.fur('wood', -21, -7, 1.4, 1.4, Y0, 0.45);
    L.fur(C.leather, -27, -17.5, 6, 1.0, Y0, 1.1); L.fur('marble', -27, -17.5, 6.2, 1.2, Y0 + 1.1, 0.06, false);
    L.fur(C.cream, -26, 6, 2.4, 3.4, Y0, 0.55); L.fur('fabric', -26, 6.2, 2.5, 2.8, Y0 + 0.55, 0.15, false); L.fur('wood', -18, 12, 2, 2, Y0, 0.9);
    L.fur(C.leather, -27, 13, 2.6, 0.9, Y0, 2.2);
    // east penthouse: Tet Woch gym + bedroom
    L.fur('darkMetal', 14, -17, 1, 2, Y0, 0.5); L.fur('darkMetal', 14, -13, 1, 2, Y0, 0.5);
    for (const z of [-17.5, -13.5]) L.mesh(new THREE.CylinderGeometry(0.2, 0.2, 1.2, 12), C.leather, 18 + (z < -15 ? 0 : 2), Y0 + 2.6, z);
    L.fur('darkMetal', 20, -8, 1.6, 0.6, Y0, 0.35); for (let i = 0; i < 4; i++) L.mesh(new THREE.CylinderGeometry(0.22 - i * 0.02, 0.22 - i * 0.02, 0.05, 14), EXTRA.steel, 20.7, Y0 + 0.4 + i * 0.06, -8, false);
    L.fur(C.cream, 27, 8, 3, 2.6, Y0, 0.5); L.fur('fabric', 27, 8, 3.1, 2.7, Y0 + 0.5, 0.18, false); L.fur('wood', 20, 10, 4, 1, Y0, 0.85); L.fur(C.sofa, 20, -6, 3, 1.2, Y0, 0.6); L.fur('wood', 17, 4, 3, 1.2, Y0, 0.5);
    // breaker panel on the north wall
    L.fur(EXTRA.steel, -14, -19.55, 0.6, 0.15, Y0 + 1.2, 0.9); L.bx(EXTRA.redGlow, -14.1, Y0 + 1.9, -19.45, -13.9, Y0 + 2.0, -19.4);
    // lights
    L.lamp(-21, Y1 - 0.2, -8, 0xffe0b0, 90, 15); L.lamp(-21, Y1 - 0.2, 10, 0xffe0b0, 80, 14); L.lamp(21, Y1 - 0.2, -8, 0xffe0b0, 90, 15); L.lamp(21, Y1 - 0.2, 10, 0xffe0b0, 80, 14);
    L.lamp(0, Y1 - 0.2, -17, 0xffffff, 60, 16); L.lamp(0, Y1 - 0.2, 17, 0xffffff, 60, 16);
    stairCore(L, Y0, 3.8, 10, 0.26);
    out.levels['F1'] = B;
  }

  // =============================================================================================
  //  F2: second floor (Level 5): maze of 3 x 6 rooms (example), spikes, armory, altar hall, reserve room
  // =============================================================================================
  {
    const B = new Builder('F2'), L = new Loc(B, 0, OZ), Y0 = LV.F2, Y1 = CEIL.F2;
    const OPEN5 = [];
    for (let c = 0; c < 5; c++) OPEN5.push([[2, c], [2, c + 1]]);
    for (let c = 0; c < 5; c++) if (c !== 2) OPEN5.push([[1, c], [1, c + 1]]);
    for (let c = 0; c < 5; c++) if (c !== 1) OPEN5.push([[0, c], [0, c + 1]]);
    OPEN5.push([[2, 0], [1, 0]], [[2, 3], [1, 3]], [[1, 2], [0, 2]], [[1, 1], [0, 1]], [[1, 5], [0, 5]]);
    const open5 = (a, b) => OPEN5.some(([x, y]) => (x[0] === a[0] && x[1] === a[1] && y[0] === b[0] && y[1] === b[1]) || (x[0] === b[0] && x[1] === b[1] && y[0] === a[0] && y[1] === a[1]));
    L.slab('plasterWhite', -30.3, -20.3, 30.3, 20.3, Y0 - 0.4, Y0, [holeStair]);
    L.floor('wood', -29.7, -4, 29.7, 19.7, Y0); L.floor('tile', -29.7, -19.7, 23.7, -4.3, Y0); L.floor('tile', 24.3, -12.0, 29.7, -4.3, Y0);
    exterior(L, { y0: Y0, y1: Y1, sill: 0.9, head: 2.7, wd: 1.6, step: 5, shut: true, S: { avoid: [-20, -10, 0, 10, 20] }, N: { avoid: [-22, 24] }, W: { avoid: [-4, 4, 12, -12] }, E: { avoid: [-4, 4, 12, -12.25] } });
    perimeterBand(L, Y0 - 0.45, 0.45, 0.14); perimeterBand(L, Y1 - 0.2, 0.2, 0.16);
    L.wall('plasterWhite', -30, -4, 30, -4, 0.5, Y0, Y1, [{ c: -15, w: 2.6, kind: 'door' }, { c: 15, w: 2.6, kind: 'door' }]);
    for (let k = 1; k <= 5; k++) { const v = []; for (let r = 0; r < 3; r++) if (open5([r, k - 1], [r, k])) v.push({ c: 8 * r, w: 2.6, kind: 'door' }); L.wall('plasterDark', -30 + 10 * k, -4, -30 + 10 * k, 20, 0.5, Y0, Y1, v); }
    for (let k = 1; k <= 2; k++) { const h = []; for (let c = 0; c < 6; c++) if (open5([k - 1, c], [k, c])) h.push({ c: -25 + 10 * c, w: 2.6, kind: 'door' }); L.wall('plasterDark', -30, -4 + 8 * k, 30, -4 + 8 * k, 0.5, Y0, Y1, h); }
    L.wall('plasterWhite', -22, -20, -22, -12, 0.5, Y0, Y1, [{ c: -16, w: 2.6, kind: 'door' }]); L.wall('plasterWhite', -30, -12, -22, -12, 0.5, Y0, Y1);
    L.fur('wood', -22, -16, 0.3, 2.6, Y0, 2.4);                              // the hidden door: a bookcase until the lever is pulled
    for (let i = 0; i < 5; i++) L.bx(C.bone, -22.15, Y0 + 0.3 + i * 0.45, -17.1, -21.85, Y0 + 0.65 + i * 0.45, -14.9);
    stairRoomWalls(L, Y0, Y1, true);
    // altar hall: altar, pillars, drums, ledger, binding
    L.fur(C.stone, 0, -12, 8, 3, Y0, 0.9); L.fur(C.stone, 0, -14.5, 12, 1, Y0, 1.6); L.fur('wood', 0, -12, 8.4, 3.4, Y0 + 0.9, 0.08, false);
    for (const x of [-12, -6, 6, 12]) for (const z of [-9, -18]) { L.fur('concrete', x, z, 0.8, 0.8, Y0, Y1 - Y0 - 0.2); L.bx('concrete', x - 0.6, Y0, z - 0.6, x + 0.6, Y0 + 0.25, z + 0.6); }
    for (let i = 0; i < 6; i++) { const col = [0x58c26a, 0x7a3ab0, 0xe0a040, 0xc02a2a, 0x2a8ac0, 0xe0e060][i]; L.mesh(new THREE.CylinderGeometry(0.07, 0.09, 0.3, 12), C.bottle(col), -2.5 + i, Y0 + 1.1, -11, false); }
    for (let i = 0; i < 16; i++) { const a = i * 0.4; L.candle(-5 + i * 0.66, Y0 + 0.98, -13.5 + (i % 2) * 0.4); }
    for (const [x, z] of [[-27, -8], [27, -8], [27, -17]]) L.mesh(new THREE.CylinderGeometry(0.45, 0.45, 0.9, 14), mat('rust'), x, Y0 + 0.45, z);
    L.fur(C.gold, 3.8, -12, 0.3, 0.2, Y0 + 0.98, 0.1, false); L.fur(EXTRA.redGlow, 0, -8.2, 0.5, 0.5, Y0, 0.3, false);
    L.fur('wood', -20, -6.5, 0.2, 0.2, Y0, 1.0); L.bx(EXTRA.redGlow, -20.12, Y0 + 1.0, -6.62, -19.88, Y0 + 1.1, -6.38);
    L.light(0, Y0 + 3.0, -12, 0xff7a30, 130, 22, 'candle', 1); L.light(-14, Y0 + 3, -14, 0xff7a30, 60, 14, 'candle', 1); L.light(14, Y0 + 3, -14, 0xff7a30, 60, 14, 'candle', 1);
    // reserve room (Nwit's health bottles): shelves of glowing vials
    for (const z of [-18.8, -13.2]) for (let i = 0; i < 12; i++) { const col = 0x58c26a; L.mesh(new THREE.CylinderGeometry(0.06, 0.08, 0.28, 10), C.bottle(col), -29.2 + (i % 6) * 1.1, Y0 + 0.9 + Math.floor(i / 6) * 0.9, z, false); }
    L.fur('wood', -26, -19.3, 6.0, 0.5, Y0, 2.0); L.fur('wood', -26, -12.7, 6.0, 0.4, Y0, 2.0);
    L.light(-26, Y0 + 2.8, -16, 0x58ffa0, 40, 10, 'int');
    // armory (room r=2,c=0): M50 crate + ammunition boxes
    L.fur('crate', -25, 15.5, 1.4, 0.8, Y0, 0.8); L.fur(C.green, -22.5, 17.5, 1.0, 0.6, Y0, 0.4); L.fur(C.green, -27, 18, 1.0, 0.6, Y0, 0.4); L.fur(C.green, -27, 18, 0.8, 0.5, Y0 + 0.4, 0.35); L.bx(EXTRA.redGlow, -25.7, Y0 + 0.82, 15.1, -24.3, Y0 + 0.85, 15.9);
    L.light(-25, Y0 + 2.6, 16, 0xffb060, 50, 9, 'int');
    // maze rooms: tables, bones; spike traps
    const cell = (r, c) => [-25 + 10 * c, 8 * r];
    for (let r = 0; r < 3; r++) for (let c = 0; c < 6; c++) { const [cx, cz] = cell(r, c); if ((r + c) % 2 === 1) L.fur('wood', cx + 1, cz - 1, 2, 0.9, Y0, 0.75); L.lamp(cx, Y1 - 0.2, cz, 0xffc880, 38, 9); }
    const spk = new THREE.ConeGeometry(0.07, 0.55, 5);
    for (const [r, c] of [[1, 4], [1, 3], [1, 2], [0, 5], [2, 3], [1, 0]]) {
      const [cx, cz] = cell(r, c); L.bx(EXTRA.black, cx - 2, Y0, cz - 1.5, cx + 2, Y0 + 0.03, cz + 1.5);
      const im = new THREE.InstancedMesh(spk, EXTRA.steel, 7 * 5), m4 = new THREE.Matrix4(); let k = 0;
      for (let i = 0; i < 7; i++) for (let j = 0; j < 5; j++) { m4.setPosition(cx - 1.8 + i * 0.6 + MZX * 0 + 0, Y0 + 0.28, cz - 1.2 + j * 0.6 + OZ); im.setMatrixAt(k++, m4); }
      im.castShadow = true; B.extra.push(im);
    }
    stairCore(L, Y0, 3.8, 10, 0.26);
    out.levels['F2'] = B;
  }

  // =============================================================================================
  //  F3: Granmet Nwit's quarters (Level 6): 3 x 5 rooms of 12 x 13 m
  // =============================================================================================
  {
    const B = new Builder('F3'), L = new Loc(B, 0, OZ), Y0 = LV.F3, Y1 = CEIL.F3, HZ = 19.75;
    const OPEN6 = [[[2, 0], [2, 1]], [[2, 1], [2, 2]], [[2, 2], [2, 3]], [[2, 3], [2, 4]], [[1, 0], [1, 1]], [[1, 1], [1, 2]], [[1, 2], [1, 3]], [[1, 3], [1, 4]], [[0, 0], [0, 1]], [[0, 2], [0, 3]], [[0, 3], [0, 4]], [[2, 0], [1, 0]], [[2, 2], [1, 2]], [[2, 4], [1, 4]], [[1, 1], [0, 1]], [[1, 3], [0, 3]], [[1, 4], [0, 4]], [[1, 0], [0, 0]], [[0, 1], [0, 2]]];
    const open6 = (a, b) => OPEN6.some(([x, y]) => (x[0] === a[0] && x[1] === a[1] && y[0] === b[0] && y[1] === b[1]) || (x[0] === b[0] && x[1] === b[1] && y[0] === a[0] && y[1] === a[1]));
    const cell = (r, c) => [-24 + 12 * c, -13 + 13 * r];
    L.slab('plasterWhite', -30.3, -20.3, 30.3, 20.3, Y0 - 0.4, Y0, [holeStair]);
    L.floor('marble', -29.5, -19.5, 29.5, 19.5, Y0, [holeStair]);
    exterior(L, { y0: Y0, y1: Y1, hz: HZ, sill: 0.9, head: 2.7, wd: 1.6, step: 6, shut: true, S: { avoid: [-18, -6, 6, 18] }, N: { avoid: [-18, -6, 6, 18, 24] }, W: { avoid: [-6.5, 6.5] }, E: { avoid: [-6.5, 6.5, -12.25] } });
    perimeterBand(L, Y0 - 0.45, 0.45, 0.14, -30.25, 30.25, -HZ - 0.25, HZ + 0.25); perimeterBand(L, Y1 - 0.2, 0.2, 0.16, -30.25, 30.25, -HZ - 0.25, HZ + 0.25);
    for (let k = 1; k <= 4; k++) { const v = []; for (let r = 0; r < 3; r++) if (open6([r, k - 1], [r, k])) v.push({ c: -13 + 13 * r, w: 2.6, kind: 'door' }); L.wall('plasterWhite', -30 + 12 * k, -HZ, -30 + 12 * k, HZ, 0.5, Y0, Y1, v); }
    for (let k = 1; k <= 2; k++) { const h = []; for (let c = 0; c < 5; c++) if (open6([k - 1, c], [k, c])) h.push({ c: -24 + 12 * c, w: 2.6, kind: 'door' }); L.wall('plasterWhite', -30, -HZ + 13 * k + 0.25, 30, -HZ + 13 * k + 0.25, 0.5, Y0, Y1, h); }
    stairRoomWalls(L, Y0, Y1, true);
    let [cx, cz] = cell(2, 1); L.fur(C.sofa, cx - 3, cz + 3.5, 3.6, 1.2, Y0, 0.6); L.fur(C.sofa, cx + 4, cz, 1.2, 3.4, Y0, 0.6); L.fur('wood', cx, cz, 2, 1, Y0, 0.45);
    [cx, cz] = cell(2, 2); for (let i = 0; i < 5; i++) L.fur('wood', cx - 4.4 + i * 2.2, cz + 5.9, 1.8, 0.7, Y0, 2.2);
    [cx, cz] = cell(2, 3); L.fur('wood', cx, cz - 5.8, 9, 0.6, Y0, 2.8); L.fur('wood', cx + 2, cz + 1, 3, 1.4, Y0, 0.78);
    for (let i = 0; i < 9; i++) L.bx(C.bone, cx - 4.3 + i, Y0 + 0.5 + (i % 3) * 0.7, cz - 5.55, cx - 3.7 + i, Y0 + 1.1 + (i % 3) * 0.7, cz - 5.5);
    [cx, cz] = cell(2, 4); L.fur('wood', cx + 2, cz, 3, 1.4, Y0, 0.78);
    [cx, cz] = cell(1, 0); L.fur(C.sofa, cx - 1, cz - 2, 4.4, 3.4, Y0, 0.55); L.fur('fabric', cx - 1, cz - 2, 4.5, 3.4, Y0 + 0.55, 0.2, false); L.fur('wood', cx + 3, cz - 4, 0.9, 0.9, Y0, 0.55); L.fur('wood', cx - 5, cz - 4, 0.9, 0.9, Y0, 0.55);
    [cx, cz] = cell(1, 1); for (let i = 0; i < 8; i++) { L.fur('wood', cx - 5 + i * 1.4, cz - 6.1, 0.7, 0.06, Y0 + 1.4, 0.9, false); L.fur(C.cream, cx - 5 + i * 1.4, cz - 6.12, 0.55, 0.02, Y0 + 1.5, 0.7, false); } L.fur(C.leather, cx, cz + 1, 3, 1, Y0, 0.5);
    [cx, cz] = cell(1, 2); L.fur('concrete', cx, cz, 3.2, 1.6, Y0, 0.9); L.fur(C.black, cx, cz, 3.3, 1.7, Y0 + 0.9, 0.05, false);
    for (let i = 0; i < 12; i++) { const a = i / 12 * Math.PI * 2; L.candle(cx + Math.cos(a) * 4, Y0, cz + Math.sin(a) * 3.6); }
    L.light(cx, Y0 + 1.8, cz, 0xff5a30, 90, 14, 'candle', 1);
    [cx, cz] = cell(1, 3);                                            // the vast bathroom: Jacuzzi
    L.mesh(new THREE.CylinderGeometry(3.6, 3.6, 0.9, 36), C.cream, cx, Y0 + 0.45, cz); L.mesh(new THREE.CylinderGeometry(3.2, 3.2, 0.2, 36), new THREE.MeshPhysicalMaterial({ color: 0x2aa0b8, roughness: 0.05, transparent: true, opacity: 0.8 }), cx, Y0 + 0.86, cz, false);
    for (const [dx, dz] of [[-5, -5.5], [5, -5.5], [-5, 5.5], [5, 5.5]]) L.mesh(new THREE.CylinderGeometry(0.4, 0.4, 2.4, 12), C.cream, cx + dx, Y0 + 1.2, cz + dz);
    L.light(cx, Y0 + 2.5, cz, 0xa0ffff, 50, 10);
    [cx, cz] = cell(1, 4); L.fur(C.cream, cx + 3, cz - 2, 2.6, 4, Y0, 0.6);
    for (let c = 0; c < 4; c++) { [cx, cz] = cell(0, c); L.fur('wood', cx, cz - 5.8, 8, 0.6, Y0, 1.0); L.fur(C.stone, cx - 3.5, cz + 2, 0.5, 0.5, Y0, 1.0); L.fur(C.stone, cx + 3.5, cz + 2, 0.5, 0.5, Y0, 1.0); L.candle(cx - 3.5, Y0 + 1.0, cz + 2); L.candle(cx + 3.5, Y0 + 1.0, cz + 2); }
    // the stash: 8 health bottles on pedestals
    [[0, 0, 3, -3], [0, 2, -3, 3], [0, 4, 3, 3], [1, 1, -3, -3], [1, 3, 3, 2], [2, 1, 2.5, -3], [2, 3, -3, 3], [2, 4, 3, -3]].forEach(([r, c, dx, dz]) => {
      const [bx, bz] = cell(r, c); L.fur(C.stone, bx + dx, bz + dz, 0.5, 0.5, Y0, 0.9); L.mesh(new THREE.CylinderGeometry(0.07, 0.09, 0.3, 12), C.bottle(0x58c26a), bx + dx, Y0 + 1.05, bz + dz, false);
    });
    for (const [r, c] of [[2, 0], [1, 1], [0, 1], [2, 3], [1, 3], [0, 3], [1, 0], [2, 4]]) { const [x, z] = cell(r, c); L.lamp(x, Y1 - 0.2, z + 0.5, 0xffb870, 55, 13); }
    L.light(-24, Y0 + 2.6, 13, 0xffd9a0, 40, 9);
    // secret panels
    const pts = [];
    for (let r = 0; r < 3; r++) for (let c = 0; c < 5; c++) { const [px, pz] = cell(r, c); for (const [dx, dz] of [[0, -5.2], [0, 5.2], [-4.8, 0], [4.8, 0]]) pts.push([px + dx, Y0 + 1.3, pz + dz + OZ]); }
    out.meta.secrets['F3'] = pts;
    stairCore(L, Y0, 3.8, 10, 0.26);
    out.levels['F3'] = B;
  }

  // =============================================================================================
  //  ROOF (Level 7): parapet, tank, units, mast, candle ring, hoist zone, stair head house
  // =============================================================================================
  {
    const B = new Builder('ROOF'), L = new Loc(B, 0, OZ), Y0 = LV.ROOF;
    L.slab('plasterWhite', -30.3, -20.3, 30.3, 20.3, Y0 - 0.4, Y0 - 0.06, [holeStair], false); L.slab('roofTar', -30.3, -20.3, 30.3, 20.3, Y0 - 0.06, Y0, [holeStair]);
    // parapet 1.2 m with coping
    const pz = (z0, z1) => { L.solid('plasterWhite', -30.3, Y0, z0, 30.3, Y0 + 1.2, z1); };
    L.solid('plasterWhite', -30.3, Y0, -20.3, 30.3, Y0 + 1.2, -19.95); L.solid('plasterWhite', -30.3, Y0, 19.95, 30.3, Y0 + 1.2, 20.3);
    L.solid('plasterWhite', -30.3, Y0, -19.95, -29.95, Y0 + 1.2, 19.95); L.solid('plasterWhite', 29.95, Y0, -19.95, 30.3, Y0 + 1.2, 19.95);
    L.bx('concrete', -30.45, Y0 + 1.2, -20.45, 30.45, Y0 + 1.3, -19.85); L.bx('concrete', -30.45, Y0 + 1.2, 19.85, 30.45, Y0 + 1.3, 20.45);
    L.bx('concrete', -30.45, Y0 + 1.2, -19.85, -29.85, Y0 + 1.3, 19.85); L.bx('concrete', 29.85, Y0 + 1.2, -19.85, 30.45, Y0 + 1.3, 19.85);
    // stair head house at the NE (the reconciled stair core): door onto the deck
    const hh = Y0 + 2.8;
    L.wall('plasterWhite', SR.x0, -20, SR.x0, SR.zw, 0.5, Y0, hh); L.wall('plasterWhite', SR.x1, -20, SR.x1, SR.zw, 0.5, Y0, hh, [], { mat: mat('plasterOchre'), t: 0.05, side: 1 });
    L.wall('plasterWhite', SR.x0, SR.zw, SR.x1, SR.zw, 0.5, Y0, hh, [{ c: 25.55, w: 1.6, head: 2.4, kind: 'door' }]);
    L.wall('plasterWhite', SR.x0, -20, SR.x1, -20, 0.5, Y0, hh);
    L.slab('concrete', SR.x0 - 0.5, -20.5, SR.x1 + 0.5, SR.zw + 0.5, hh, hh + 0.3, [], true);
    L.light(27, Y0 + 2.3, -16.5, 0xffc890, 30, 7);
    // water tank on a steel stand
    for (const [dx, dz] of [[-1.6, -1.6], [1.6, -1.6], [-1.6, 1.6], [1.6, 1.6]]) L.solid(EXTRA.steel, -12 + dx - 0.08, Y0, -10 + dz - 0.08, -12 + dx + 0.08, Y0 + 2.4, -10 + dz + 0.08);
    L.bx(EXTRA.steel, -14, Y0 + 2.4, -12, -10, Y0 + 2.5, -8); L.mesh(new THREE.CylinderGeometry(2.2, 2.2, 2.8, 32), mat('metal'), -12, Y0 + 3.9, -10); L.mesh(new THREE.CylinderGeometry(2.2, 0.1, 0.5, 32), mat('metal'), -12, Y0 + 5.55, -10);
    // AC units with fans, vents, antenna mast with a red beacon
    for (const [x, z, w, d] of [[12, -10, 3, 2], [14, 10, 3, 2], [4, 9, 1.6, 1.6], [6, -8, 1.6, 1.6], [-16, 8, 2, 1.2], [-4, -12, 1.4, 1.4]]) {
      L.fur(EXTRA.white, x, z, w, d, Y0, 1.3); L.mesh(new THREE.CylinderGeometry(0.4, 0.4, 0.05, 20), EXTRA.black, x, Y0 + 1.33, z, false);
    }
    L.mesh(new THREE.CylinderGeometry(0.1, 0.16, 9, 8), EXTRA.steel, -20, Y0 + 4.5, 0);
    for (let y = 1.5; y < 8; y += 1.5) L.mesh(new THREE.BoxGeometry(1.2, 0.04, 0.04), EXTRA.steel, -20, Y0 + y, 0, false);
    const beacon = L.mesh(new THREE.SphereGeometry(0.14, 10, 8), EXTRA.redGlow, -20, Y0 + 9.1, 0, false); out.meta.beacon = beacon; L.light(-20, Y0 + 9, 0, 0xff2020, 12, 14, 'night');
    // the candle ring (Nwit starts here) and the helicopter hoist zone painted on the deck
    for (let i = 0; i < 12; i++) { const a = i / 12 * Math.PI * 2; L.candle(-6 + Math.cos(a) * 2.6, Y0, Math.sin(a) * 2.6); }
    L.light(-6, Y0 + 0.9, 0, 0xff7a30, 40, 12, 'candle', 1);
    { const ring = new THREE.Mesh(new THREE.RingGeometry(3.8, 4.0, 48), new THREE.MeshBasicMaterial({ color: 0xe0b020, side: THREE.DoubleSide })); ring.rotation.x = -Math.PI / 2; ring.position.set(-6, Y0 + 0.02, OZ); B.extra.push(ring);
      const stain = new THREE.Mesh(new THREE.CircleGeometry(0.7, 20), new THREE.MeshBasicMaterial({ color: 0x3a0c0c, transparent: true, opacity: 0.8 })); stain.rotation.x = -Math.PI / 2; stain.position.set(-7, Y0 + 0.02, 0.6 + OZ); B.extra.push(stain); }
    L.lamp(0, Y0 + 3.5, 0, 0xffffff, 0, 1);                       // (no ceiling): a night light on the mast instead
    B.lights.pop(); B.lights.push({ x: 0, y: Y0 + 4, z: OZ, color: 0xb8c8ff, intensity: 60, dist: 30, tag: 'night' });
    out.levels['ROOF'] = B;
  }
  return out;
}
