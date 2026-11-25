// Geometry builder: static boxes are baked in world space with UVs in metres, then merged per material
// so the whole building is a few dozen draw calls. Also records collision boxes and walkable surfaces.
import * as THREE from 'three';
import { mergeGeometries } from 'three/addons/utils/BufferGeometryUtils.js';
import { mat, SCALE, EXTRA } from './mats.js';

export class Builder {
  constructor(name) {
    this.group = new THREE.Group(); this.group.name = name;
    this.parts = new Map();     // material key -> geometries
    this.colliders = [];        // {x0,z0,x1,z1,yb,yt}
    this.surfaces = [];         // walkable: {x0,z0,x1,z1,y, holes:[[x0,z0,x1,z1]]} or ramps
    this.lights = [];           // {x,y,z,color,intensity,dist}
    this.extra = [];            // meshes added directly (water, glass, animated)
  }
  // material may be a key of mats.js SPEC or a THREE.Material (then UV scale = 1 m)
  box(m, cx, cy, cz, sx, sy, sz) {
    if (sx <= 1e-4 || sy <= 1e-4 || sz <= 1e-4) return;
    const g = new THREE.BoxGeometry(sx, sy, sz); g.translate(cx, cy, cz);
    const k = 1 / (typeof m === 'string' ? SCALE[m] : (m.userData.uvScale || 1)), p = g.attributes.position, n = g.attributes.normal, uv = g.attributes.uv;
    for (let i = 0; i < p.count; i++) {
      const ax = Math.abs(n.getX(i)) > 0.5 ? 0 : Math.abs(n.getY(i)) > 0.5 ? 1 : 2;
      const x = p.getX(i), y = p.getY(i), z = p.getZ(i);
      if (ax === 0) uv.setXY(i, z * k, y * k); else if (ax === 1) uv.setXY(i, x * k, z * k); else uv.setXY(i, x * k, y * k);
    }
    const isKey = typeof m === 'string', key = isKey ? m : m.uuid;
    let slot = this.parts.get(key);
    if (!slot) { slot = { mat: isKey ? mat(m) : m, geos: [] }; this.parts.set(key, slot); }
    slot.geos.push(g);
  }
  // box from corners
  bx(m, x0, y0, z0, x1, y1, z1) { this.box(m, (x0 + x1) / 2, (y0 + y1) / 2, (z0 + z1) / 2, Math.abs(x1 - x0), Math.abs(y1 - y0), Math.abs(z1 - z0)); }
  collide(x0, z0, x1, z1, yb, yt) { this.colliders.push({ x0: Math.min(x0, x1), z0: Math.min(z0, z1), x1: Math.max(x0, x1), z1: Math.max(z0, z1), yb, yt }); }
  // solid block with collision
  solid(m, x0, y0, z0, x1, y1, z1) { this.bx(m, x0, y0, z0, x1, y1, z1); this.collide(x0, z0, x1, z1, y0, y1); }

  // Axis-aligned wall. (x1,z1)-(x2,z2) centre line, thickness t, heights y0..y1.
  // ops: [{c, w, sill=0, head=2.5, kind:'door'|'window'|'arch'|'gap'}] c = coordinate along the wall.
  // skin: {mat, t, side} an outer render coat on one side (+1 = +z/+x side, -1 = other side).
  wall(m, x1, z1, x2, z2, t, y0, y1, ops = [], skin = null) {
    const vertical = Math.abs(x1 - x2) < 1e-6;        // runs along z
    const a = vertical ? Math.min(z1, z2) : Math.min(x1, x2), b = vertical ? Math.max(z1, z2) : Math.max(x1, x2);
    const fixed = vertical ? x1 : z1;
    const piece = (c0, c1, ya, yb, collide) => {
      if (c1 - c0 < 1e-4 || yb - ya < 1e-4) return;
      const core = skin ? t - skin.t : t, off = skin ? (skin.t / 2) * -skin.side : 0;
      const mk = (mm, f0, f1) => vertical ? this.bx(mm, f0, ya, c0, f1, yb, c1) : this.bx(mm, c0, ya, f0, c1, yb, f1);
      mk(m, fixed + off - core / 2, fixed + off + core / 2);
      if (skin) { const e = fixed + skin.side * (t / 2 - skin.t / 2); mk(skin.mat, e - skin.t / 2, e + skin.t / 2); }
      if (collide) vertical ? this.collide(fixed - t / 2, c0, fixed + t / 2, c1, ya, yb) : this.collide(c0, fixed - t / 2, c1, fixed + t / 2, ya, yb);
    };
    const list = ops.map(o => ({ ...o, sill: o.sill ?? 0, head: o.head ?? 2.5, lo: o.c - o.w / 2, hi: o.c + o.w / 2 })).sort((p, q) => p.lo - q.lo);
    let cur = a;
    for (const o of list) {
      piece(cur, o.lo, y0, y1, true);
      if (o.kind === 'gap') { cur = o.hi; continue; }
      piece(o.lo, o.hi, y1 > y0 + o.head ? y0 + o.head : y1, y1, false);               // lintel
      if (o.sill > 0) piece(o.lo, o.hi, y0, y0 + o.sill, true);                            // sill wall
      if (o.kind === 'window') {
        const yb = y0 + o.sill, yt = y0 + o.head;
        vertical ? this.collide(fixed - t / 2, o.lo, fixed + t / 2, o.hi, yb, yt) : this.collide(o.lo, fixed - t / 2, o.hi, fixed + t / 2, yb, yt);
        this.windowDeco(vertical, fixed, t, o, yb, yt, skin);
      } else if (o.kind === 'gate') this.gate(vertical, fixed, o, y0, y0 + o.head);
      cur = o.hi;
    }
    piece(cur, b, y0, y1, true);
  }
  windowDeco(vertical, fixed, t, o, yb, yt, skin) {
    const f = 0.07, w = o.hi - o.lo, h = yt - yb, mid = (o.lo + o.hi) / 2, ym = (yb + yt) / 2;
    const at = (c, y, sc, sy, sf) => vertical ? [fixed, y, c, sf, sy, sc] : [c, y, fixed, sc, sy, sf];
    const FR = o.frame || EXTRA.frame;
    this.box(FR, ...at(o.lo + f / 2, ym, f, h, t + 0.1)); this.box(FR, ...at(o.hi - f / 2, ym, f, h, t + 0.1));
    this.box(FR, ...at(mid, yb + f / 2, w, f, t + 0.14)); this.box(FR, ...at(mid, yt - f / 2, w, f, t + 0.1));
    this.box(FR, ...at(mid, ym, f * 0.6, h, 0.05));
    if (h > 1.4) this.box(FR, ...at(mid, ym + h * 0.18, w, f * 0.6, 0.05));
    const g = new THREE.BoxGeometry(...(vertical ? [0.02, h, w] : [w, h, 0.02])); g.translate(...(vertical ? [fixed, ym, mid] : [mid, ym, fixed]));
    this.glassGeos = this.glassGeos || []; this.glassGeos.push(g);
    if (o.shutters) {
      const sw = w / 2 - 0.02, sgn = o.shutters;
      for (const s of [-1, 1]) this.box(EXTRA.shutter, ...at(mid + s * (w / 2 + sw / 2 + 0.03), ym, sw, h, 0.06).map((v, i) => (i === (vertical ? 0 : 2) ? v + sgn * (t / 2 + 0.1) : v)));
    }
  }
  gate(vertical, fixed, o, y0, y1) {          // a locked steel gate (red bars) in a doorway
    const w = o.hi - o.lo, at = (c, y, sc, sy, sf) => vertical ? [fixed, y, c, sf, sy, sc] : [c, y, fixed, sc, sy, sf];
    const n = Math.round(w / 0.16);
    for (let i = 0; i <= n; i++) this.box(EXTRA.red, ...at(o.lo + (w * i) / n, (y0 + y1) / 2, 0.035, y1 - y0, 0.035));
    for (const y of [y0 + 0.1, (y0 + y1) / 2, y1 - 0.1]) this.box(EXTRA.red, ...at((o.lo + o.hi) / 2, y, w, 0.05, 0.05));
    vertical ? this.collide(fixed - 0.1, o.lo, fixed + 0.1, o.hi, y0, y1) : this.collide(o.lo, fixed - 0.1, o.hi, fixed + 0.1, y0, y1);
  }
  // rectangular slab with rectangular holes
  slab(m, x0, z0, x1, z1, y0, y1, holes = [], walk = true) {
    let rects = [[x0, z0, x1, z1]];
    for (const [hx0, hz0, hx1, hz1] of holes) {
      const out = [];
      for (const [a0, b0, a1, b1] of rects) {
        if (hx1 <= a0 || hx0 >= a1 || hz1 <= b0 || hz0 >= b1) { out.push([a0, b0, a1, b1]); continue; }
        if (hx0 > a0) out.push([a0, b0, hx0, b1]);
        if (hx1 < a1) out.push([hx1, b0, a1, b1]);
        const ax0 = Math.max(a0, hx0), ax1 = Math.min(a1, hx1);
        if (hz0 > b0) out.push([ax0, b0, ax1, hz0]);
        if (hz1 < b1) out.push([ax0, hz1, ax1, b1]);
      }
      rects = out;
    }
    for (const [a0, b0, a1, b1] of rects) this.bx(m, a0, y0, b0, a1, y1, b1);
    if (walk) this.surfaces.push({ kind: 'slab', x0, z0, x1, z1, y: y1, holes });
  }
  // finished floor skin over a room (thin, walkable already counted by the slab)
  floor(m, x0, z0, x1, z1, y, holes = []) { this.slab(m, x0, z0, x1, z1, y, y + 0.03, holes, false); }
  light(x, y, z, color = 0xffd9a8, intensity = 14, dist = 14, tag = 'int') { this.lights.push({ x, y, z, color, intensity, dist, tag }); }
  lamp(x, y, z, color, intensity, dist) { this.box(EXTRA.lamp, x, y, z, 0.5, 0.06, 0.25); this.light(x, y - 0.3, z, color, intensity, dist); }

  finish() {
    for (const { mat: mm, geos } of this.parts.values()) {
      if (!geos.length) continue;
      const g = mergeGeometries(geos, false); geos.forEach(x => x.dispose());
      const mesh = new THREE.Mesh(g, mm); mesh.castShadow = true; mesh.receiveShadow = true; this.group.add(mesh);
    }
    if (this.glassGeos?.length) {
      const g = mergeGeometries(this.glassGeos.map(x => { x.deleteAttribute('uv'); return x; }), false);
      const mesh = new THREE.Mesh(g, EXTRA.glass); mesh.renderOrder = 5; this.group.add(mesh);
    }
    for (const e of this.extra) this.group.add(e);
    return this.group;
  }
}
