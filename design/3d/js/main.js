// Wout San Retou: the building and landscape in 3D. Orbit or walk through every level.
import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { GLTFExporter } from 'three/addons/exporters/GLTFExporter.js';
import { buildWorld, terrainY } from './world.js';
import { buildMansion, LV, POOL } from './mansion.js';
import { EXTRA } from './mats.js';

const canvas = document.getElementById('c');
const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, preserveDrawingBuffer: true });
renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
renderer.shadowMap.enabled = true; renderer.shadowMap.type = THREE.PCFSoftShadowMap;
renderer.toneMapping = THREE.ACESFilmicToneMapping; renderer.outputColorSpace = THREE.SRGBColorSpace;
const scene = new THREE.Scene();
const camera = new THREE.PerspectiveCamera(60, 1, 0.05, 2500);
scene.add(camera);

// ------------------------------------------------------------------ build
const status = document.getElementById('status');
const say = (t) => { if (status) status.textContent = t; };
say('Building the landscape...');
await new Promise(r => setTimeout(r, 30));
const world = buildWorld();
say('Building the mansion...');
await new Promise(r => setTimeout(r, 30));
const mans = buildMansion();

const groups = {};     // level key -> THREE.Group
const colliders = {};  // level key -> colliders
const surfaces = {};   // level key -> surfaces
const lightDefs = [];
const keys = ['SITE', 'GF-A', 'GF-B', 'F1', 'F2', 'F3', 'ROOF'];
{
  const g = world.builder.finish(); g.add(world.extras); groups.SITE = g; colliders.SITE = world.builder.colliders; surfaces.SITE = world.builder.surfaces;
  for (const l of world.builder.lights) lightDefs.push({ ...l, level: 'SITE' });
  for (const k of keys.slice(1)) {
    const B = mans.levels[k]; groups[k] = B.finish(); colliders[k] = B.colliders; surfaces[k] = B.surfaces;
    for (const l of B.lights) lightDefs.push({ ...l, level: k });
  }
  for (const k of keys) scene.add(groups[k]);
}
// secret panel markers (red diamonds, as on the drawing)
for (const [k, pts] of Object.entries(mans.meta.secrets)) {
  const im = new THREE.InstancedMesh(new THREE.OctahedronGeometry(0.11, 0), new THREE.MeshBasicMaterial({ color: 0xff2040 }), pts.length), m = new THREE.Matrix4();
  pts.forEach((p, i) => { m.compose(new THREE.Vector3(...p), new THREE.Quaternion(), new THREE.Vector3(0.8, 1.2, 0.8)); im.setMatrixAt(i, m); });
  im.userData.secret = true; groups[k].add(im);
}
for (const k of keys) groups[k].traverse(o => { if (o.material) { for (const m of [].concat(o.material)) m.clipShadows = true; } });

// ------------------------------------------------------------------ sky and lights
const skyMat = new THREE.ShaderMaterial({
  side: THREE.BackSide, depthWrite: false, fog: false,
  uniforms: { uTop: { value: new THREE.Color() }, uHor: { value: new THREE.Color() }, uSunDir: { value: new THREE.Vector3(0, 1, 0) }, uSunCol: { value: new THREE.Color() }, uStars: { value: 0 }, uTime: { value: 0 }, uCloud: { value: 0.6 } },
  vertexShader: 'varying vec3 vDir; void main(){ vDir = normalize(position); gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0); }',
  fragmentShader: `varying vec3 vDir; uniform vec3 uTop, uHor, uSunDir, uSunCol; uniform float uStars, uTime, uCloud;
    float hs(vec2 p){ return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453); }
    float vn(vec2 p){ vec2 i = floor(p), f = fract(p); f = f*f*(3.0-2.0*f); return mix(mix(hs(i), hs(i+vec2(1,0)), f.x), mix(hs(i+vec2(0,1)), hs(i+vec2(1,1)), f.x), f.y); }
    float fbm(vec2 p){ float a = 0.5, s = 0.0; for(int i=0;i<5;i++){ s += a*vn(p); p *= 2.03; a *= 0.5; } return s; }
    void main(){
      vec3 d = normalize(vDir); float t = clamp(d.y, 0.0, 1.0);
      vec3 col = mix(uHor, uTop, pow(t, 0.5));
      float sd = max(dot(d, uSunDir), 0.0);
      col += uSunCol * (pow(sd, 900.0) * 6.0 + pow(sd, 14.0) * 0.30 + pow(sd, 3.0) * 0.06);
      if (d.y < 0.0) col = mix(uHor, uHor * 0.35, clamp(-d.y * 5.0, 0.0, 1.0));
      if (d.y > 0.015) {
        vec2 uv = d.xz / (d.y + 0.2) * 0.55 + vec2(uTime * 0.004, 0.0);
        float n = fbm(uv * 2.6); float c = smoothstep(0.52, 0.82, n) * uCloud;
        vec3 cc = mix(uHor * 1.05 + 0.1, vec3(1.0), 0.35) * (0.35 + 0.65 * clamp(uSunDir.y * 2.0 + 0.5, 0.0, 1.0));
        col = mix(col, cc + uSunCol * 0.15 * pow(sd, 4.0), c * smoothstep(0.015, 0.22, d.y) * 0.85);
      }
      vec3 sp = floor(d * 260.0); float r = fract(sin(dot(sp, vec3(12.9898, 78.233, 37.719))) * 43758.5453);
      col += step(0.9972, r) * uStars * smoothstep(0.08, 0.4, d.y) * vec3(1.0, 0.95, 0.9) * (0.5 + r * 0.5);
      gl_FragColor = vec4(col, 1.0);
      #include <tonemapping_fragment>
      #include <colorspace_fragment>
    }`,
});
const sky = new THREE.Mesh(new THREE.SphereGeometry(1800, 32, 20), skyMat); sky.frustumCulled = false; sky.renderOrder = -10; scene.add(sky);
const sun = new THREE.DirectionalLight(0xffffff, 1);
sun.castShadow = true; sun.shadow.mapSize.set(3072, 3072);
const sc = sun.shadow.camera; sc.left = -95; sc.right = 95; sc.top = 85; sc.bottom = -85; sc.near = 10; sc.far = 520;
sun.shadow.bias = -0.0004; sun.shadow.normalBias = 0.04; scene.add(sun, sun.target);
sun.target.position.set(-15, 0, -35);
const hemi = new THREE.HemisphereLight(0xffffff, 0x444444, 1); scene.add(hemi);
const amb = new THREE.AmbientLight(0xffffff, 0.1); scene.add(amb);
const pmrem = new THREE.PMREMGenerator(renderer);
const skyProbe = new THREE.Scene(); const probeMesh = new THREE.Mesh(new THREE.SphereGeometry(10, 24, 16), skyMat); skyProbe.add(probeMesh);

// A small pool of real point lights is moved to the definitions nearest the viewer each frame
// (a WebGL context cannot afford ~90 lights at once).
const POOL_N = 12;
const pool = Array.from({ length: POOL_N }, () => { const l = new THREE.PointLight(0xffffff, 0, 10, 2); scene.add(l); return { l, cur: null, k: 0 }; });

const MOODS = {
  day: { el: 58, az: 130, sun: 0xfff0d8, si: 3.4, hemiSky: 0xbcd6ff, hemiGnd: 0x6a5a3c, hi: 0.9, amb: 0.22, top: 0x3f7cc8, hor: 0xbdd6ea, fog: 0xbcd0df, exp: 0.85, stars: 0, env: 0.7, night: 0, inner: 0.5, candle: 0.7, cloud: 0.75 },
  dusk: { el: 7, az: 250, sun: 0xff9a55, si: 2.4, hemiSky: 0xf0a47a, hemiGnd: 0x3a3040, hi: 0.5, amb: 0.1, top: 0x2d3e78, hor: 0xf2a06a, fog: 0xc88a70, exp: 0.95, stars: 0.25, env: 0.35, night: 0.6, inner: 0.85, candle: 1, cloud: 0.55 },
  night: { el: 38, az: 200, sun: 0x9db4ff, si: 1.1, hemiSky: 0x3a4c86, hemiGnd: 0x141a26, hi: 0.7, amb: 0.14, top: 0x050a18, hor: 0x1c2848, fog: 0x121a30, exp: 1.5, stars: 1, env: 0.12, night: 1, inner: 1, candle: 1, cloud: 0.25 },
};
let mood = 'dusk', cur = MOODS.dusk;
function setMood(name) {
  mood = name; const m = cur = MOODS[name];
  const az = THREE.MathUtils.degToRad(m.az), el = THREE.MathUtils.degToRad(m.el), dir = new THREE.Vector3(Math.cos(el) * Math.sin(az), Math.sin(el), Math.cos(el) * Math.cos(az));
  sun.position.copy(sun.target.position).addScaledVector(dir, 260); sun.color.setHex(m.sun); sun.intensity = m.si;
  hemi.color.setHex(m.hemiSky); hemi.groundColor.setHex(m.hemiGnd); hemi.intensity = m.hi; amb.intensity = m.amb;
  skyMat.uniforms.uTop.value.setHex(m.top); skyMat.uniforms.uHor.value.setHex(m.hor); skyMat.uniforms.uSunDir.value.copy(dir); skyMat.uniforms.uSunCol.value.setHex(m.sun); skyMat.uniforms.uStars.value = m.stars; skyMat.uniforms.uCloud.value = m.cloud;
  scene.fog = new THREE.Fog(m.fog, 130, 560); renderer.toneMappingExposure = m.exp;
  const rt = pmrem.fromScene(skyProbe, 0.02); scene.environment?.dispose?.(); scene.environment = rt.texture; scene.environmentIntensity = m.env;
  if (world.stringLights) world.stringLights.visible = true;
  if (world.neon) world.neon.visible = name !== 'day';
  document.querySelectorAll('[data-mood]').forEach(b => b.classList.toggle('on', b.dataset.mood === name));
}

// ------------------------------------------------------------------ visibility, section cut, explode
const vis = Object.fromEntries(keys.map(k => [k, true]));
let explode = 0, secretsOn = true;
const cutPlane = new THREE.Plane(new THREE.Vector3(0, -1, 0), 100);
function applyVis() {
  keys.forEach((k, i) => { groups[k].visible = vis[k]; groups[k].position.y = k === 'SITE' ? 0 : [0, 0, 1, 2, 3, 4][i - 1] * explode; });
  groups['GF-A'].position.y = 0;
  groups.SITE.traverse(o => { if (o.userData.secret) o.visible = secretsOn; });
  for (const k of keys) groups[k].traverse(o => { if (o.userData.secret) o.visible = secretsOn; });
  document.querySelectorAll('[data-level]').forEach(b => b.classList.toggle('on', vis[b.dataset.level]));
}
function setCut(y) { renderer.clippingPlanes = y >= 24 ? [] : [cutPlane]; cutPlane.constant = y; }

// ------------------------------------------------------------------ camera modes
const orbit = new OrbitControls(camera, canvas);
orbit.enableDamping = true; orbit.maxPolarAngle = Math.PI * 0.98; orbit.maxDistance = 900;
camera.position.set(100, 55, 110); orbit.target.set(-18, 5, -45); orbit.update();
let walking = false, frozen = false, yaw = 0, pitch = 0; const feet = new THREE.Vector3(0, 0, 26); let vy = 0;
const keysDown = new Set();
function setWalk(on) {
  walking = on; orbit.enabled = !on; document.body.classList.toggle('walk', on);
  document.querySelectorAll('[data-mode]').forEach(b => b.classList.toggle('on', (b.dataset.mode === 'walk') === on));
  if (!on && document.pointerLockElement) document.exitPointerLock();
  if (!on) { const f = new THREE.Vector3(); camera.getWorldDirection(f); orbit.target.copy(camera.position).addScaledVector(f, 12); orbit.update(); }
}
function goto(x, y, z, yw = 0, pt = 0) { frozen = false; if (camera.fov !== 60) { camera.fov = 60; camera.updateProjectionMatrix(); } feet.set(x, y, z); yaw = yw; pitch = pt; vy = 0; setWalk(true); placeCam(); }
function placeCam() { camera.position.set(feet.x, feet.y + 1.7, feet.z); camera.rotation.set(pitch, yaw, 0, 'YXZ'); }
function orbitTo(px, py, pz, tx, ty, tz) { setWalk(false); camera.position.set(px, py, pz); orbit.target.set(tx, ty, tz); orbit.update(); }

const R = Math.PI;
export const VIEWS = [
  ['Overview', () => orbitTo(105, 62, 118, -18, 6, -42)],
  ['Compound from the gate', () => goto(0, 0, 33, 0, 0.02)],
  ['Compound aerial', () => orbitTo(-62, 46, 70, 0, 2, -10)],
  ['Helicopter cockpit (hero)', () => { goto(0.5, 0.74, 40.2, 0, -0.22); frozen = true; camera.fov = 78; camera.updateProjectionMatrix(); }],
  ['Helicopter outside', () => orbitTo(14, 5, 55, 0, 2.2, 40)],
  ['Hero start (Level 1)', () => goto(0, 0, 26, 0, 0)],
  ['House front and pergola', () => goto(0, 0, -8, 0, 0.05)],
  ['Sniper outbuilding', () => goto(-18, 0, -8, R * 1.55, 0.2)],
  ['Truck, fuel, food store', () => orbitTo(-2, 14, 24, 2, 0.5, 0)],
  ['Maze foyer (Level 2)', () => goto(-45.5, 0, -37, 0, 0)],
  ['Altar room, 3 vials', () => goto(-55.5, 0, -60, 0, -0.1)],
  ['Pool hall (Level 3)', () => goto(0, 0, -33, 0, -0.1)],
  ['Underwater: glass ceiling', () => goto(0, -2.8, -50, 0, 1.15)],
  ['Ballroom and DJ booth', () => goto(21, 0, -45, 0.25, 0)],
  ['Gym', () => goto(-22, 0, -36, 0, 0)],
  ['Glass floor (Level 4)', () => goto(0, LV.F1, -38, 0, -0.55)],
  ['Tet Woch penthouse', () => goto(15, LV.F1, -35, -0.5, -0.05)],
  ['Stair core (NE) from the ballroom', () => goto(24, 0, -48, -0.2, 0.05)],
  ['Inside the stair core', () => goto(25.55, 0, -63.3, 0, 0.35)],
  ['Second floor altar hall', () => goto(14, LV.F2, -57.5, 1.26, -0.05)],
  ['Armory (M50)', () => goto(-21.5, LV.F2, -34.5, 1.4, -0.1)],
  ['Nwit reserve room', () => goto(-26, LV.F2, -64.5, 0, 0)],
  ['Third floor antechamber', () => goto(-24, LV.F3, -31, 0, 0)],
  ['Nwit bedroom', () => goto(-24, LV.F3, -44.5, 0, -0.05)],
  ['Jacuzzi', () => goto(4, LV.F3, -50, -1.57, -0.12)],
  ['Roof candle ring', () => goto(6, LV.ROOF, -50, 1.57, 0)],
  ['Roof overview', () => orbitTo(48, 38, -20, -6, 17, -50)],
  ['Cutaway (section)', () => { setCut(12); orbitTo(80, 42, 20, 0, 6, -50); document.getElementById('cut').value = 12; }],
];

// ------------------------------------------------------------------ walking physics
const noTerrain = [[-30.4, -70.4, 30.4, -29.6], [-70.9, -70.4, -30.1, -29.6], [-9.3, -27, 9.3, -19.4]];
function groundAt(x, z, y) {
  let best = -1e9;
  const inside = noTerrain.some(r => x > r[0] && x < r[2] && z > r[1] && z < r[3]);
  if (!inside) best = terrainY(x, z);
  for (const k of keys) {
    if (!vis[k]) continue;
    for (const s of surfaces[k]) {
      let h = null;
      if (s.kind === 'slab') {
        if (x < s.x0 || x > s.x1 || z < s.z0 || z > s.z1) continue;
        if (s.holes.some(hh => x > hh[0] && x < hh[2] && z > hh[1] && z < hh[3])) continue; h = s.y;
      } else if (s.kind === 'steps') {
        const a = s.ax === 'z' ? z : x, c = s.ax === 'z' ? x : z;
        if (c < s.c0 || c > s.c1) continue; const t = (a - s.start) * s.dir; if (t < 0 || t > s.n * s.tread) continue;
        h = s.y0 + (Math.floor(t / s.tread) + 1) * s.r;
      }
      if (h != null && h <= y + 0.55 && h > best) best = h;
    }
  }
  return best;
}
function resolve(p) {
  const rad = 0.34;
  for (let it = 0; it < 3; it++) for (const k of keys) {
    if (!vis[k]) continue;
    for (const c of colliders[k]) {
      if (p.y + 1.6 < c.yb || p.y + 0.4 > c.yt) continue;
      const nx = Math.max(c.x0, Math.min(p.x, c.x1)), nz = Math.max(c.z0, Math.min(p.z, c.z1)), dx = p.x - nx, dz = p.z - nz, d2 = dx * dx + dz * dz;
      if (d2 >= rad * rad) continue;
      if (d2 > 1e-8) { const d = Math.sqrt(d2), k2 = (rad - d) / d; p.x += dx * k2; p.z += dz * k2; }
      else { const l = p.x - c.x0, r = c.x1 - p.x, t = p.z - c.z0, b = c.z1 - p.z, m = Math.min(l, r, t, b); if (m === l) p.x = c.x0 - rad; else if (m === r) p.x = c.x1 + rad; else if (m === t) p.z = c.z0 - rad; else p.z = c.z1 + rad; }
    }
  }
}
function stepWalk(dt) {
  if (frozen) { if (keysDown.has('arrowleft')) yaw += 1.6 * dt; if (keysDown.has('arrowright')) yaw -= 1.6 * dt; placeCam(); return; }
  const run = keysDown.has('shift') ? 2.2 : 1, sp = 3.2 * run;
  const fx = -Math.sin(yaw), fz = -Math.cos(yaw), rx = Math.cos(yaw), rz = -Math.sin(yaw);
  let mx = 0, mz = 0;
  if (keysDown.has('w') || keysDown.has('arrowup')) { mx += fx; mz += fz; } if (keysDown.has('s') || keysDown.has('arrowdown')) { mx -= fx; mz -= fz; }
  if (keysDown.has('d')) { mx += rx; mz += rz; } if (keysDown.has('a')) { mx -= rx; mz -= rz; }
  if (keysDown.has('arrowleft')) yaw += 1.6 * dt; if (keysDown.has('arrowright')) yaw -= 1.6 * dt;
  const l = Math.hypot(mx, mz) || 1; feet.x += mx / l * sp * dt * (mx || mz ? 1 : 0); feet.z += mz / l * sp * dt * (mx || mz ? 1 : 0);
  resolve(feet);
  const g = groundAt(feet.x, feet.z, feet.y);
  if (feet.y > g + 0.02) { vy -= 20 * dt; feet.y += vy * dt; if (feet.y < g) { feet.y = g; vy = 0; } } else { feet.y = g; vy = 0; }
  placeCam();
}
addEventListener('keydown', e => {
  const k = e.key.toLowerCase(); keysDown.add(k === ' ' ? 'space' : k);
  if (k === 'f' && !e.repeat) setWalk(!walking);
  if (walking && k === ' ' && vy === 0) vy = 6.2;
  if (['arrowup', 'arrowdown', 'arrowleft', 'arrowright', ' '].includes(k)) e.preventDefault();
});
addEventListener('keyup', e => keysDown.delete(e.key.toLowerCase() === ' ' ? 'space' : e.key.toLowerCase()));
canvas.addEventListener('mousedown', () => { if (walking && canvas.requestPointerLock) canvas.requestPointerLock?.(); });
addEventListener('mousemove', e => {
  if (!walking) return;
  if (document.pointerLockElement === canvas || (e.buttons & 1)) { yaw -= e.movementX * 0.0024; pitch = Math.max(-1.45, Math.min(1.45, pitch - e.movementY * 0.0024)); }
});

// ------------------------------------------------------------------ resize and loop
function resize() {
  const w = canvas.clientWidth || innerWidth, h = canvas.clientHeight || innerHeight;
  renderer.setSize(w, h, false); camera.aspect = w / h; camera.updateProjectionMatrix();
}
addEventListener('resize', resize); resize();
const clock = new THREE.Clock(); let frames = 0, fpsT = 0, T = 0;
function loop() {
  const dt = Math.min(clock.getDelta(), 0.1); T += dt;
  world.update(T); skyMat.uniforms.uTime.value = T;
  sky.position.copy(camera.position);
  if (mans.meta.water) { const m = mans.meta.water; m.water.material.normalMap.offset.set(T * 0.02, T * 0.013); m.caus.material.map.offset.set(T * 0.03, -T * 0.02); }
  if (mans.meta.ball) mans.meta.ball.rotation.y = T * 0.6;
  if (mans.meta.beacon) mans.meta.beacon.visible = Math.sin(T * 3) > 0;
  const mm = cur, ref = walking ? camera.position : orbit.target;
  const scored = [];
  for (const d of lightDefs) {
    if (!vis[d.level]) continue;
    const f0 = d.tag === 'night' ? mm.night : d.tag === 'candle' ? mm.candle : mm.inner; if (f0 <= 0) continue;
    const dx = d.x - ref.x, dy = d.y - ref.y, dz = d.z - ref.z; scored.push([dx * dx + dy * dy * 4 + dz * dz, d]);
  }
  scored.sort((p, q) => p[0] - q[0]);
  const want = scored.slice(0, POOL_N).map(x => x[1]);
  const free = pool.filter(p => !want.includes(p.cur));
  for (const d of want) if (!pool.some(p => p.cur === d)) { const p = free.pop(); if (p) { p.cur = d; p.k = 0; p.l.color.setHex(d.color); p.l.distance = d.dist * 1.6; p.l.position.set(d.x, d.y, d.z); } }
  for (const p of pool) {
    const d = p.cur, on = d && want.includes(d);
    p.k += ((on ? 1 : 0) - p.k) * Math.min(1, dt * 6);
    if (!on && p.k < 0.01) { p.cur = null; p.l.intensity = 0; continue; }
    let f = d.tag === 'night' ? mm.night : d.tag === 'candle' ? mm.candle : mm.inner;
    if (d.flick === 1) f *= 0.85 + 0.15 * Math.sin(T * 9 + d.x) * Math.sin(T * 5.3 + d.z);
    if (d.flick === 2) f *= 0.6 + 0.4 * Math.sin(T * 1.3 + d.x);
    p.l.intensity = d.intensity * f * p.k * 1.35;
  }
  if (walking) stepWalk(dt); else orbit.update();
  renderer.render(scene, camera);
  frames++; fpsT += dt; if (fpsT > 1) { const e = document.getElementById('fps'); if (e) e.textContent = Math.round(frames / fpsT) + ' fps'; frames = 0; fpsT = 0; }
  requestAnimationFrame(loop);
}

// ------------------------------------------------------------------ UI
function buildUI() {
  const v = document.getElementById('views');
  VIEWS.forEach(([n, f]) => { const b = document.createElement('button'); b.textContent = n; b.onclick = () => { f(); }; v.appendChild(b); });
  document.querySelectorAll('[data-mood]').forEach(b => b.onclick = () => setMood(b.dataset.mood));
  document.querySelectorAll('[data-level]').forEach(b => b.onclick = () => { vis[b.dataset.level] = !vis[b.dataset.level]; applyVis(); });
  document.querySelectorAll('[data-mode]').forEach(b => b.onclick = () => setWalk(b.dataset.mode === 'walk'));
  document.getElementById('cut').oninput = (e) => setCut(+e.target.value);
  document.getElementById('explode').oninput = (e) => { explode = +e.target.value; applyVis(); };
  document.getElementById('secrets').onchange = (e) => { secretsOn = e.target.checked; applyVis(); };
  document.getElementById('onlyup').onclick = () => { for (const k of keys) vis[k] = true; applyVis(); setCut(100); document.getElementById('cut').value = 24; document.getElementById('explode').value = 0; explode = 0; applyVis(); };
  document.getElementById('export').onclick = async () => {
    say('Exporting GLB...');
    const ex = new GLTFExporter(); const src = new THREE.Group(); for (const k of keys) if (vis[k]) src.add(groups[k]);
    ex.parse(src, (buf) => { for (const k of keys) scene.add(groups[k]); const a = document.createElement('a'); a.href = URL.createObjectURL(new Blob([buf], { type: 'model/gltf-binary' })); a.download = 'wout-san-retou-building.glb'; a.click(); say('GLB saved.'); },
      (e) => { for (const k of keys) scene.add(groups[k]); say('Export failed: ' + e.message); }, { binary: true, onlyVisible: true, maxTextureSize: 1024 });
  };
}
buildUI(); setMood('dusk'); applyVis(); setCut(100);
say(''); document.getElementById('loading').style.display = 'none';
window.wsr = { THREE, scene, camera, renderer, setMood, goto, orbitTo, VIEWS, vis, applyVis, setCut, groups, setWalk, feet, get yaw() { return yaw; } };
const q = new URLSearchParams(location.hash.slice(1));
if (q.get('mood')) setMood(q.get('mood'));
if (q.get('view')) { const v = VIEWS.find(x => x[0].toLowerCase().includes(q.get('view').toLowerCase())); v && v[1](); }
loop();
