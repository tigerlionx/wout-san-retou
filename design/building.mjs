// Wout San Retou: the building, drawn in OURCAD (headless).
//
// Run:   node design/building.mjs            (from game-design/wout-san-retou)
// Out:   design/wout-san-retou-building.des  (open in OURCAD: open -a OURCAD design/wout-san-retou-building.des)
//        design/wout-san-retou-building.pdf
//
// Everything is in METRES in the game; the drawing is in MILLIMETRES (1 m = 1000).
// Game (x, z) -> drawing (X, Y) = (ox + x*1000, oy - z*1000), so game NORTH (-z) is UP the sheet.
// Numbers come from src/level.rs ... src/level7.rs (walls, doorways, furniture, spawn points).
// Things that are NEW (not in the game yet) are tagged "PROPOSAL" on the drawing.
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const { Drawing } = await import(process.env.OURCAD_API);

const here = path.dirname(fileURLToPath(import.meta.url));
const M = 1000;
const d = await Drawing.create({ units: 'mm', annoScale: 100 });
let bad = 0;
const ck = (what, r) => { if (!r.ok) { bad++; console.log('FAIL', what, r.errors.join(';'), r.waitingFor || ''); } return r; };

// --------------------------------------------------------------------------- layers
const PAL = {
  MURS: '#ff6b5b', PORTES: '#ffd23f', FENETRES: '#38bdf8', PIECES: '#22d3ee', ESCALIERS: '#a3e635', COTES: '#4ade80', DALLES: '#9ca3af',
  SECRET: '#ff2d55', VERROU: '#ff2d55', MOBILIER: '#9ca3af', EAU: '#38bdf8', VITRE: '#67e8f9', PATROUILLE: '#a78bfa', TIR: '#fb923c',
  ACTEURS: '#facc15', TEXTE: '#d1d5db', AMENAGEMENT: '#4ade80', HELICO: '#fb923c', OBJECTIFS: '#facc15', AUTEL: '#c084fc', TITRE: '#ffffff',
  PROPOSITION: '#fb923c',
};
for (const [n, c] of Object.entries(PAL)) d.layer(n, { color: c, current: false });
d.layer('0', { current: true });

// --------------------------------------------------------------------------- floor frame
let fl = { ox: 0, oy: 0, pre: 'X' };
let mark0 = 0;
const X = (x) => Math.round(fl.ox + x * M);
const Y = (z) => Math.round(fl.oy - z * M);
const P = (x, z) => [X(x), Y(z)];
const S = (x, z) => `${X(x)},${Y(z)}`;
function begin(pre, ox, oy) { fl = { ox: ox * M, oy: oy * M, pre }; mark0 = d.doc.entities.length; d.doc.current = '0'; }
// Every object made since begin() goes on its own floor layers (SITE-MURS, GF1-MURS, ...): toggle them in OURCAD.
function end() {
  const made = d.doc.entities.slice(mark0);
  for (const e of made) {
    const base = e.layer || '0';
    const nm = `${fl.pre}-${base}`;
    if (!d.doc.layer(nm)) d.layer(nm, { color: d.doc.layer(base)?.color || '#d1d5db', current: false });
    e.layer = nm;
  }
  d.doc.current = '0';
  d.doc.changed();
}

// --------------------------------------------------------------------------- drawing helpers (game metres in)
const wall = async (x1, z1, x2, z2, t = 500) => ck(`wall ${x1},${z1}`, await d.run(`WALL W ${t} ${S(x1, z1)} ${S(x2, z2)} ""`));
const wallLoop = async (pts, t = 500) => ck('wallLoop', await d.run(`WALL W ${t} ${pts.map(([x, z]) => S(x, z)).join(' ')} C`));
// the game's wall_line(): a wall along z (vertical) or along x, with doorways of `w` metres at `doors`
async function wl(vertical, fixed, a, b, doors = [], w = 2.6, t = 500) {
  if (vertical) await wall(fixed, a, fixed, b, t); else await wall(a, fixed, b, fixed, t);
  for (const dd of doors) await opening(vertical ? fixed : dd, vertical ? dd : fixed, w);
}
const opening = async (x, z, w = 2.6) => ck(`opening ${x},${z}`, await d.run(`DOOR T opening W ${Math.round(w * M)} ${S(x, z)} ""`));
const R = (x0, z0, x1, z1, layer = 'MOBILIER', o = {}) => d.rect(P(x0, z0), P(x1, z1), { layer, ...o });
const RC = (cx, cz, sx, sz, layer = 'MOBILIER', o = {}) => R(cx - sx / 2, cz - sz / 2, cx + sx / 2, cz + sz / 2, layer, o);
const poly = (pts, layer, o = {}) => d.polyline(pts.map(([x, z]) => P(x, z)), { layer, ...o });
const circ = (x, z, r, layer = 'MOBILIER', o = {}) => d.circle(P(x, z), r * M, { layer, ...o });
const line = (x1, z1, x2, z2, layer, o = {}) => d.line(P(x1, z1), P(x2, z2), { layer, ...o });
const txt = (x, z, text, h = 450, layer = 'TEXTE', o = {}) => d.text(P(x, z), text, { h, align: 'center', layer, ...o });
const txtL = (x, z, text, h = 450, layer = 'TEXTE', o = {}) => d.text(P(x, z), text, { h, align: 'left', layer, ...o });
const title = (x, z, big, small) => { txt(x, z, big, 1300, 'TITRE'); if (small) txt(x, z + 1.9, small, 600, 'TEXTE'); };
const diamond = (x, z, r = 0.32, layer = 'SECRET') => poly([[x, z - r], [x + r, z], [x, z + r], [x - r, z]], layer, { closed: true });
const dot = (x, z, label, layer = 'ACTEURS', r = 0.4, dx = 0, dz = -0.9, h = 380) => {
  circ(x, z, r, layer);
  if (label) txt(x + dx, z + dz, label, h, layer);
};
const water = (x0, z0, x1, z1, layer = 'EAU', pattern = 'ANSI31', alpha = 0.35) =>
  d.hatch([P(x0, z0), P(x1, z0), P(x1, z1), P(x0, z1)], { pattern, scale: 400, alpha, layer });
const space = async (x, z, name) => ck(`space ${name}`, await d.run(`SPACE ${S(x, z)} "${name}" ""`));
const spaceAt = (pts, name) => d.add('space', { pts: pts.map(([x, z]) => P(x, z)), name }); // explicit room outline (inner wall faces)
const dimH = async (x1, x2, z, off) => ck('dimH', await d.run(`DIMLINEAR ${S(x1, z)} ${S(x2, z)} ${S((x1 + x2) / 2, z - off)}`));
const dimV = async (z1, z2, x, off) => ck('dimV', await d.run(`DIMLINEAR ${S(x, z1)} ${S(x, z2)} ${S(x - off, (z1 + z2) / 2)}`));
const north = (x, z) => { poly([[x, z + 1.8], [x + 0.7, z - 0.8], [x, z - 0.2], [x - 0.7, z - 0.8]], 'TEXTE', { closed: true }); txt(x, z - 2.6, 'N', 800, 'TEXTE'); };
// a staircase: rise in mm, width in mm, start (centre of the first tread) and a point on its direction
const stair = async (rise, width, sx, sz, ex, ez) => ck('stair', await d.run(`STAIR ${rise} ${width} ${S(sx, sz)} ${S(ex, ez)}`));
// a locked door: a red bar across the opening with its reason
function locked(x, z, w, vertical, why, tx, tz) {
  if (vertical) RC(x, z, 0.5, w, 'VERROU'); else RC(x, z, w, 0.5, 'VERROU');
  txt(tx ?? x, tz ?? z + 1.2, why, 420, 'VERROU');
}
function arrival(x, z, why) {
  poly([[x, z - 0.7], [x + 0.6, z + 0.45], [x - 0.6, z + 0.45]], 'OBJECTIFS', { closed: true });
  txt(x, z + 1.4, why, 400, 'OBJECTIFS');
}
const STAIRS_UP = []; // collected for the stack diagram

// =============================================================================================
//  0. GENERAL NOTES (left of the site plan)
// =============================================================================================
async function notes() {
  begin('NOTES', -105, 0);
  txtL(0, -34, 'WOUT SAN RETOU: THE BUILDING', 1500, 'TITRE');
  txtL(0, -31.6, 'Drawn in OURCAD from the game code (src/level.rs ... level7.rs). Fictional place, fictional characters.', 520);
  txtL(0, -30, 'Review and change anything: tell Claude, and the game will be updated to match.', 520);
  txtL(0, -27.5, 'SHEETS: 1 Site and landscape | 2 Helicopter cockpit | 3 Ground floor A (maze, altar) | 4 Ground floor B (Madanm Flanm, pool)', 450);
  txtL(0, -26.2, '5 First floor (Tet Woch, glass floor) | 6 Second floor (Zonbi Manchet, M50, reserve, altar hall) | 7 Third floor (Granmet Nwit) | 8 Roof | 9 Stair stack', 450);
  txtL(0, -24.2, 'Scale: 1 drawing unit = 1 mm = 1 mm of real size. Game NORTH is up. Each floor has its own layers (SITE-*, GF1-*, GF2-*, F1-*, F2-*, F3-*, ROOF-*).', 450);
  txtL(0, -22.9, 'Colours: red = SECRET passage panels and LOCKED stair doors | orange = PROPOSAL (new, not in the game yet) | yellow = objectives, arrivals | violet = guard patrols.', 450);
  const rows = [
    ['FLOORS AND GAME LEVELS'],
    ['FLOOR', 'GAME LEVEL', 'MAIN EVENT', 'STAIR UP (locked until)'],
    ['Outdoor', 'Level 1', 'Sabotage truck, fuel, food; sniper; reach the house', 'House door (opens after the 3 sabotages)'],
    ['Helicopter', 'NEW (before L1)', 'Hero in the cockpit launches the first strike', '-'],
    ['Ground A', 'Level 2', 'Maze, Nwit in secret panels, poison, 3-vial altar', 'Altar-room door (after the cure)'],
    ['Ground B', 'Level 3', 'Madanm Flanm, ~50 Lost, pool, ballroom', 'NE stair room (Flanm down)'],
    ['1st floor', 'Level 4', 'Tet Woch, ~120 Lost, glass floor over the pool', 'NE stair room (Tet Woch down)'],
    ['2nd floor', 'Level 5', 'M50 armory, reserve room, altar hall, Zonbi Manchet', 'NE stair room (binding broken)'],
    ['3rd floor', 'Level 6', 'Nwit quarters, bottle stash, hide and seek', 'NE stair room (Nwit flees)'],
    ['Roof', 'Level 7', 'Final duel, four endings, helicopter', 'Walk away down the stairs'],
  ];
  d.table(P(0, -20.5), rows, { h: 420, colW: [4000, 5000, 18000, 14000] });
  txtL(0, -8.8, 'POINTS THAT DISAGREE BETWEEN THE GAME AND THE STORY (decide, then I update the game):', 520, 'PROPOSITION');
  const issues = [
    '1. Outside the house is a low 18 x 7 m building, but every floor inside is 60 x 40 m (the maze wing is 40 x 40). The exterior needs to grow to match.',
    '2. SETTLED and in the game: the ground-floor pool is 18 x 24 m (x -9..9, z -12..12) right under the glass floor, in a central hall with walls at x +-12 (src/level3.rs and level4.rs updated; the game keeps a shallow wade zone, the 3D viewer shows it 3 m deep).',
    '3. SETTLED and in the game: the stairs stack in the NE core (x 24..30, door at x 25.55) on every floor. The arrival points of Levels 4-6 (SW, SE, W) are not moved yet: a short walk from the stair top.',
    '4. The 2nd floor maze is random in the game; the drawing shows one example. The Level 2 maze is fixed.',
    '5. The cockpit and the helicopter strike do not exist in the game yet (PROPOSAL).',
    '6. Your brief says two floors with Nwit: the game has one Nwit floor (3rd) plus the roof.',
  ];
  issues.forEach((t, i) => txtL(0, -(7.5 - i * 1.3), t, 440));
  end();
}

// =============================================================================================
//  1. SITE: the outdoor landscape (Level 1)  ox 0, oy 0
// =============================================================================================
async function site() {
  begin('SITE', 0, 0);
  title(0, -36, 'SHEET 1: OUTDOOR LANDSCAPE (Level 1, The Night Before)', 'Compound 60 x 60 m, perimeter wall 0.6 m x 2.6 m. Hero starts at the south edge; the house is at the north end.');
  // ground, lawn
  water(-30, -30, 30, 30, 'AMENAGEMENT', 'DOTS', 0.10);
  d.hatch([P(-16, -5), P(24, -5), P(24, 17), P(-16, 17)], { pattern: 'GRASS', scale: 30, alpha: 0.35, layer: 'AMENAGEMENT' });
  txt(4, 14.8, 'LAWN 40 x 22', 450, 'AMENAGEMENT');
  // perimeter wall
  await wallLoop([[-30.3, -30.3], [30.3, -30.3], [30.3, 30.3], [-30.3, 30.3]], 600);
  // the house (low brown house, flat roof, pergola)
  await wallLoop([[-9, -26.5], [9, -26.5], [9, -19.5], [-9, -19.5]], 400);
  ck('house door', await d.run(`DOOR T double W 1600 ${S(0, -19.5)} ""`));
  for (const x of [-6.5, 6.5]) ck('house window', await d.run(`WINDOW T fixed W 3200 S 900 ${S(x, -19.5)} ""`));
  await space(0, -23, 'THE HOUSE (low, flat roof, 3.6 m)');
  R(-10, -27.7, 10, -18.3, 'MOBILIER', { lt: 'DASHED' });
  txt(0, -28.4, 'roof slab 20 x 9.4 m, +3.6 m', 380);
  R(-9, -19.8, 9, -18.0, 'AMENAGEMENT', { lt: 'DASHED' });
  for (const x of [-8, -4, 0, 4, 8]) circ(x, -18.2, 0.15, 'AMENAGEMENT');
  txt(0, -17.2, 'PERGOLA with string lights', 380, 'AMENAGEMENT');
  txt(-6.5, -18.4, 'OPEN neon sign', 350, 'OBJECTIFS');
  locked(0, -19.5, 1.6, false, 'DOOR LOCKED until truck, fuel and food are sabotaged', 0, -16.0);
  // sniper outbuilding + nest
  await wallLoop([[-27.5, -14], [-22.5, -14], [-22.5, -10], [-27.5, -10]], 300);
  txt(-25, -12, 'OUTBUILDING', 420);
  txt(-25, -8.8, 'SNIPER NEST on the roof (+3.0 m)', 380, 'TIR');
  dot(-25, -12, 'Mackenson (sniper, fixed post)', 'TIR', 0.3, 5, 4.2, 380);
  d.polyline([P(-25, -12), P(6.7, -30.3), P(30.3, -30.3), P(30.3, 19.9)], { layer: 'TIR', closed: true, lt: 'DASHED' });
  txt(18, -22, 'sniper field of fire (60 deg, 45 m)', 380, 'TIR');
  // supply truck
  R(-17.2, -3.8, -14.8, 1.4, 'MOBILIER'); R(-17.2, -5.6, -14.8, -3.8, 'MOBILIER');
  txt(-16, -7, 'SUPPLY TRUCK', 420);
  dot(-16, 1.9, 'T1 slash the tires', 'OBJECTIFS', 0.35, 0, 1.3);
  // fuel store
  for (const [x, z] of [[18, -8], [19, -8.9], [17.8, -9.3], [19.2, -7.6]]) circ(x, z, 0.45, 'MOBILIER');
  txt(18.5, -11.2, 'FUEL STORE (drums)', 420);
  dot(17, -6.6, 'T2 open the valves', 'OBJECTIFS', 0.35, 0, 1.3);
  // food store
  await wallLoop([[11.5, 15.75], [16.5, 15.75], [16.5, 19.25], [11.5, 19.25]], 300);
  RC(11, 13.6, 1.0, 0.7); RC(12.1, 13.6, 1.0, 0.7);
  txt(14, 17.5, 'FOOD STORE', 420);
  dot(14, 14.9, 'T3 spoil the food', 'OBJECTIFS', 0.35, 0, -1.0);
  // crates and bushes
  for (const [x, z, sx, sz] of [[-4, 16, 1.6, 1.6], [-2.2, 17.2, 1.6, 1.6], [8, 6, 1.8, 1.2], [-9, 9, 1.4, 1.4], [5, -8, 1.4, 1.4], [-12, -10, 1.6, 1.2], [22, 2, 1.6, 1.6]]) RC(x, z, sx, sz);
  for (const [x, z] of [[-3, 8], [10, -1], [-8, -12], [2, -10], [22, -18], [-22, 12], [4, 20]]) circ(x, z, 1.0, 'AMENAGEMENT');
  txt(-3, 6.6, 'bush', 330, 'AMENAGEMENT');
  // guard patrols (the four yard guards)
  poly([[-8, 12], [8, 12], [8, 2], [-8, 2]], 'PATROUILLE', { closed: true, lt: 'DASHED' }); txt(0, 7, 'Dieuseul', 380, 'PATROUILLE');
  poly([[12, -12], [12, -4], [22, -4], [22, -14]], 'PATROUILLE', { closed: true, lt: 'DASHED' }); txt(17, -4.3, 'Kendy', 380, 'PATROUILLE');
  poly([[-10, -15], [10, -15]], 'PATROUILLE', { lt: 'DASHED' }); txt(0, -15.8, 'Roody', 380, 'PATROUILLE');
  poly([[-20, 10], [-20, -10], [-10, -8], [-10, 8]], 'PATROUILLE', { closed: true, lt: 'DASHED' }); txt(-20, 11, 'Frantz', 380, 'PATROUILLE');
  txt(-3, 27.5, 'The gang plague adds more patrols and arrivals (src/plague.rs).', 360, 'PATROUILLE');
  // hero start
  dot(0, 26, 'HERO START (checkpoint)', 'OBJECTIFS', 0.5, 0, 1.4);
  // helicopter and the first strike (PROPOSAL)
  const hz = 38;
  circ(0, hz, 4, 'HELICO', { lt: 'DASHED' });
  poly([[-1, hz - 1.8], [-1, hz + 1.4], [0, hz + 2.2], [1, hz + 1.4], [1, hz - 1.8], [0.2, hz - 5], [-0.2, hz - 5]], 'HELICO', { closed: true });
  txt(0, hz + 4.8, 'PROPOSAL: HELICOPTER (hero in the cockpit) launches the first strike', 480, 'PROPOSITION');
  for (const [i, [tx, tz]] of [[1, [-25, -12]], [2, [-16, -1]], [3, [18, -8.5]], [4, [14, 17.5]]].entries()) {
    line(0, hz - 1, tx, tz, 'HELICO', { lt: 'DASHED' });
    txt(tx * 0.45, (hz + tz) / 2 - 0.5 + (i % 2) * 1.2, `STRIKE ${i + 1}`, 380, 'HELICO');
  }
  // dimensions and north
  await dimH(-30.3, 30.3, 31.0, -3);
  await dimV(-30.3, 30.3, -31.0, 3);
  await dimH(-9, 9, -26.5, 3);
  north(36, -24);
  end();
}

// =============================================================================================
//  2. THE HELICOPTER COCKPIT (PROPOSAL)   true size 1:1
// =============================================================================================
async function cockpit() {
  begin('HELI', 85, 0);
  // This sheet is drawn at 1:20, so its text is smaller than on the other sheets.
  txt(0, -4.3, 'SHEET 2: HELICOPTER COCKPIT (PROPOSAL, true size, 1:20)', 330, 'TITRE');
  txt(0, -3.7, 'The hero sits in the left seat and launches the first strike on the four targets of Level 1. Nose points up the sheet.', 150, 'TEXTE');
  // plan: nose up (-z is north/up)
  poly([[-0.95, 2.6], [-0.95, -0.4], [-0.6, -1.5], [0, -1.9], [0.6, -1.5], [0.95, -0.4], [0.95, 2.6]], 'HELICO', { closed: true });
  poly([[-0.8, -0.2], [-0.5, -1.25], [0, -1.55], [0.5, -1.25], [0.8, -0.2]], 'VITRE', { lt: 'DASHED' });
  const tag = (n, x, z) => { circ(x, z, 0.1, 'OBJECTIFS'); txt(x, z - 0.04, String(n), 110, 'OBJECTIFS'); };
  // instrument panel and its three screens
  R(-0.8, -0.35, 0.8, -0.15, 'MOBILIER');
  R(-0.65, -0.33, -0.25, -0.17, 'OBJECTIFS'); R(-0.18, -0.33, 0.18, -0.17, 'OBJECTIFS'); R(0.25, -0.33, 0.65, -0.17, 'OBJECTIFS');
  tag(3, 0, -0.7);
  // seats, controls
  RC(-0.45, 0.65, 0.55, 0.55, 'ACTEURS'); RC(0.45, 0.65, 0.55, 0.55, 'MOBILIER');
  tag(1, -0.45, 0.65); tag(2, 0.45, 0.65);
  circ(-0.45, 0.2, 0.06, 'MOBILIER'); tag(5, -0.45, 0.02);
  RC(-0.65, -0.05, 0.1, 0.12, 'MOBILIER'); RC(-0.25, -0.05, 0.1, 0.12, 'MOBILIER');
  RC(-0.82, 0.6, 0.06, 0.35, 'MOBILIER'); tag(6, -1.15, 0.6);
  // the strike panel on the centre console
  RC(0, 0.55, 0.22, 0.9, 'OBJECTIFS');
  for (let i = 0; i < 4; i++) circ(0, 0.25 + i * 0.16, 0.04, 'OBJECTIFS');
  tag(4, 0.28, 0.45);
  // rear cabin, doors, rocket pods, tail
  RC(0, 1.9, 1.5, 0.5, 'MOBILIER'); tag(9, 0, 1.9);
  line(-0.95, 1.3, -0.95, 2.4, 'PORTES'); line(0.95, 1.3, 0.95, 2.4, 'PORTES'); tag(10, 1.2, 1.9);
  RC(-1.25, 0.9, 0.3, 1.2, 'HELICO'); RC(1.25, 0.9, 0.3, 1.2, 'HELICO'); tag(8, -1.25, 1.7); tag(8, 1.25, 1.7);
  line(0, 2.6, 0, 4.2, 'HELICO'); RC(0, 4.2, 0.1, 0.5, 'HELICO');
  txt(0.55, 3.3, 'tail boom', 130, 'TEXTE', { align: 'left' });
  await dimH(-0.95, 0.95, 2.6, -0.5);
  await dimV(-1.9, 2.6, -1.4, 0.6);
  // legend
  const leg = ['1  HERO seat (left): fires the strike', '2  PILOT seat (right)', '3  Instrument panel with 3 screens', '4  STRIKE PANEL: T1 sniper nest, T2 truck, T3 fuel, T4 food + MASTER ARM', '5  Cyclic stick', '6  Collective lever', '7  Pedals (under the panel)', '8  Rocket pods, left and right', '9  Rear bench (3 seats)', '10  Sliding doors, left and right'];
  txtL(2.6, -1.7, 'LEGEND', 220, 'TITRE');
  leg.forEach((t, i) => txtL(2.6, -1.2 + i * 0.42, t, 150, 'TEXTE'));
  txtL(2.6, 3.3, 'Main rotor disc is 10 m across, mast over the cabin (not drawn).', 140, 'HELICO');
  txtL(2.6, 3.7, 'Everything here is a PROPOSAL: the game has no cockpit yet.', 140, 'PROPOSITION');
  // side view, nose to the right (points are [x, height above the ground line])
  const e0 = 9;
  const H = (pts) => pts.map(([x, h]) => [x, e0 - h]);
  txt(0, e0 + 0.5, 'SIDE VIEW (nose right)', 200, 'TEXTE');
  line(-3.5, e0, 3.5, e0, 'DALLES');
  poly(H([[-3.4, 1.3], [-1.4, 1.15], [-1.4, 1.9], [0.6, 2.0], [1.1, 2.0], [2.0, 1.35], [2.7, 0.9], [2.7, 0.6], [-1.4, 0.6]]), 'HELICO', { closed: true });
  poly(H([[0.7, 0.6], [0.7, 1.95], [1.1, 2.0], [2.0, 1.35], [2.7, 0.9]]), 'VITRE');
  R(-0.6, e0 - 1.7, 0.4, e0 - 0.8, 'PORTES', { lt: 'DASHED' }); txt(-0.1, e0 - 1.2, 'sliding door', 110, 'PORTES');
  line(-0.3, e0 - 2.0, -0.3, e0 - 2.6, 'HELICO'); line(-3, e0 - 2.6, 3, e0 - 2.6, 'HELICO');
  line(-1.2, e0 - 0.1, 2.2, e0 - 0.1, 'MOBILIER'); line(-0.8, e0 - 0.1, -0.7, e0 - 0.6, 'MOBILIER'); line(1.6, e0 - 0.1, 1.7, e0 - 0.6, 'MOBILIER');
  RC(1.5, e0 - 0.9, 0.45, 0.5, 'ACTEURS'); txt(1.5, e0 - 0.95, 'HERO', 110, 'ACTEURS');
  RC(0.6, e0 - 0.35, 1.0, 0.2, 'HELICO'); txt(0.6, e0 - 0.1 + 0.25, 'rocket pod', 100, 'HELICO');
  txt(0, e0 + 1.1, 'Seat height 0.45 m, canopy top 1.4 m, mast top 2.6 m, door 0.9 x 1.2 m (PROPOSAL values).', 130, 'PROPOSITION');
  end();
}

// =============================================================================================
//  3. GROUND FLOOR A: the maze wing and the altar room (Level 2)   ox 0, oy -80
// =============================================================================================
const OPEN2 = [[[3, 2], [3, 3]], [[3, 2], [2, 2]], [[2, 2], [2, 1]], [[2, 2], [1, 2]], [[2, 1], [2, 0]], [[2, 1], [3, 1]], [[3, 1], [3, 0]], [[1, 2], [1, 3]], [[1, 2], [0, 2]], [[1, 2], [1, 1]], [[1, 1], [0, 1]], [[0, 2], [0, 1]], [[0, 2], [0, 3]], [[1, 1], [1, 0]], [[2, 0], [1, 0]], [[1, 3], [2, 3]], [[0, 0], [1, 0]]];
const open2 = (a, b) => OPEN2.some(([x, y]) => (x[0] === a[0] && x[1] === a[1] && y[0] === b[0] && y[1] === b[1]) || (x[0] === b[0] && x[1] === b[1] && y[0] === a[0] && y[1] === a[1]));
const cell2 = (r, c) => [-15 + 10 * c, -15 + 10 * r];
async function groundA() {
  begin('GF1', 0, -80);
  title(0, -26.5, 'SHEET 3: GROUND FLOOR A, THE MAZE WING AND THE ALTAR ROOM (Level 2)', '40 x 40 m, 4 x 4 rooms of 10 m, walls 0.5 m. Nwit steps out of the SECRET PANELS (red diamonds). Stair: none; the way out is the altar-room door.');
  await wallLoop([[-20, -20], [20, -20], [20, 20], [-20, 20]]);
  for (let k = 1; k <= 3; k++) {
    const line0 = -20 + 10 * k;
    await wall(line0, -20, line0, 20); // vertical line
    await wall(-20, line0, 20, line0); // horizontal line
  }
  for (let k = 1; k <= 3; k++) {
    for (let r = 0; r < 4; r++) if (open2([r, k - 1], [r, k])) await opening(-20 + 10 * k, -15 + 10 * r, 2.4);
    for (let c = 0; c < 4; c++) if (open2([k - 1, c], [k, c])) await opening(-15 + 10 * c, -20 + 10 * k, 2.4);
  }
  await opening(-5, -20, 2.0); // the way out of the altar room
  // rooms
  const names = { '3,2': 'FOYER (start)', '0,1': 'ALTAR ROOM', '1,3': 'BONE ROOM', '1,1': 'SKULL HALL' };
  for (let r = 0; r < 4; r++) for (let c = 0; c < 4; c++) {
    const [cx, cz] = cell2(r, c);
    await space(cx, cz + 3.2, names[`${r},${c}`] || `ROOM ${'ABCD'[c]}${r + 1}`);
  }
  // furniture: tables, shelves, rug
  for (let r = 0; r < 4; r++) for (let c = 0; c < 4; c++) {
    const [cx, cz] = cell2(r, c);
    if ((r + c) % 2 === 0 && !(r === 0 && c === 1)) RC(cx + 1.5, cz - 1.2, 2.0, 0.9);
  }
  for (const [r, c] of [[1, 3], [1, 1]]) { const [cx, cz] = cell2(r, c); RC(cx, cz - 4.4, 7.0, 0.6); }
  RC(5, 14, 5, 3, 'MOBILIER', { lt: 'DASHED' }); txt(5, 12.2, 'rug', 300);
  // altar room
  RC(-5, -16.6, 5.4, 1.2, 'AUTEL');
  for (const x of [-8.2, -1.8]) circ(x, -16.6, 0.15, 'AUTEL');
  [[-6.4, 'V1'], [-5, 'V2'], [-3.6, 'V3']].forEach(([x, n]) => { circ(x, -16.6, 0.12, 'AUTEL'); txt(x, -15.6, n, 300, 'AUTEL'); });
  ['ALTAR: 3 VIALS', 'one cures, one kills,', 'one drugs (shuffled)', 'poison timer: 2 min'].forEach((t, i) => txt(-5, -13.6 + i * 0.8, t, 300, 'AUTEL'));
  locked(-5, -20, 2.0, false, 'EXIT DOOR sealed until the cure is drunk', -5, -21.6);
  // notes (clues), start
  dot(...cell2(3, 3).map((v, i) => v + (i === 0 ? -2 : 0)), 'NOTE (cure clue)', 'OBJECTIFS', 0.3, 0, 1.3, 300);
  dot(cell2(2, 0)[0] + 2, cell2(2, 0)[1] + 2, 'NOTE (Kreyol clue)', 'OBJECTIFS', 0.3, 0, 1.3, 300);
  arrival(5, 17.2, 'HERO START');
  // secret panels: four per room, 1.1 m inside each wall
  for (let r = 0; r < 4; r++) for (let c = 0; c < 4; c++) {
    const [cx, cz] = cell2(r, c);
    for (const [dx, dz] of [[0, -3.9], [0, 3.9], [-3.9, 0], [3.9, 0]]) diamond(cx + dx, cz + dz);
  }
  txt(0, 22.2, 'RED DIAMONDS = SECRET PANELS (64). Nwit hides in the wall cavities and steps out of any panel near Ro, then vanishes after each hit ("Woy!").', 420, 'SECRET');
  txt(0, 23.2, 'He slashes twice, then stabs with the poisoned blade: 2 minutes to reach the altar.', 420, 'SECRET');
  // link to wing B
  txt(0, 25.0, 'From the altar-room door the way leads to the party wing (Sheet 4), stair foot in the gym, south-west corner.', 400, 'OBJECTIFS');
  await dimH(-20, 20, 20, -2);
  await dimV(-20, 20, -21, 3);
  north(26, -14);
  end();
}

// =============================================================================================
//  4. GROUND FLOOR B: the party wing (Level 3)   ox 85, oy -80
// =============================================================================================
async function groundB() {
  begin('GF2', 85, -80);
  title(0, -26.5, 'SHEET 4: GROUND FLOOR B, MADANM FLANM AND THE PARTY WING (Level 3)', '60 x 40 m. ~50 Lost. Madanm Flanm guards the NE stair room (red) at the DJ booth. Pool wadeable (slow, noisy).');
  await wallLoop([[-30, -20], [30, -20], [30, 20], [-30, 20]]);
  // RECONCILED (issue 2 settled): the pool hall is the full-depth central hall x -12..12, the same walls
  // that carry the first-floor gallery above, and the pool is exactly the glass-floor outline (18 x 24 m).
  await wl(true, -12, -20, 20, [10, -14]);   // west rooms | pool hall
  await wl(false, 0, -30, -12, [-24]);       // locker | gym
  await wl(true, 12, -20, 20, [-14, -2, 14]); // pool hall | ballroom, veranda
  await wl(false, 8, 12, 30, [15, 24]);      // ballroom | veranda
  await wl(true, 24, -20, -12.25, []);       // stair core (NE, stacks on every floor)
  await wl(false, -12.25, 24, 30, [25.55]);  // stair core door (locked)
  await space(-21, -10, 'LOCKER ROOM (6 Lost)');
  await space(-21, 10, 'GYM (8 Lost)');
  await space(-3, 16.5, 'POOL HALL + BAR (20 Lost)');
  await space(21, 1, 'BALLROOM + DJ (12 Lost)');
  await space(21, 14, 'VERANDA (4 Lost)');
  await space(27, -16, 'STAIR CORE');
  // pool
  R(-9, -12, 9, 12, 'EAU'); water(-9, -12, 9, 12);
  txt(0, 0, 'SWIMMING POOL 18 x 24 m', 520, 'EAU');
  txt(0, -1.2, '3 m deep, wade or swim: slow and noisy. Pit stairs NW.', 360, 'EAU');
  for (const x of [-9.5, -6, -2, 2, 6, 9.5]) RC(x, 17.2, 0.8, 2.0);
  txt(0, 1.4, 'the 1st-floor glass floor (Sheet 5) is exactly above: same 18 x 24 m', 330, 'VITRE');
  // locker room, gym
  for (let i = 0; i < 6; i++) RC(-28 + i * 1.7, -19.4, 1.4, 0.7);
  RC(-24, -10, 4, 0.8); RC(-24, -6, 4, 0.8);
  for (const z of [4, 8, 12]) RC(-27.5, z, 1, 2);
  RC(-22, 6, 2.2, 0.9);
  // bar
  RC(0, -18, 14, 0.9); RC(0, -19.55, 14, 0.4);
  for (let i = 0; i < 9; i++) circ(-6 + i * 1.5, -16.9, 0.22);
  for (const x of [-3, 3]) dot(x, -18, null, 'OBJECTIFS', 0.2);
  txt(0, -14.6, 'BAR (north end of the pool hall): free gifts (nothing is free)', 330, 'OBJECTIFS');
  // ballroom
  water(13, -12, 29, 7.7, 'MOBILIER', 'SOLID', 0.15);
  txt(21, -4, 'DANCE FLOOR', 450);
  RC(17, -19, 6, 1.6); RC(11.5, -18.8, 1.2, 1.2); RC(22.5, -18.8, 1.2, 1.2);
  RC(28, -6, 1.2, 3); RC(28, -1, 1.2, 3); RC(11, 2, 3, 1.2);
  dot(17, -16.6, 'DJ BOOTH: hold E to cut the music', 'OBJECTIFS', 0.35, 0, 1.2, 360);
  dot(19, -14.5, 'MADANM FLANM', 'ACTEURS', 0.5, 0, 1.2, 420);
  for (const x of [12, 18, 26]) RC(x, 18.5, 2, 0.8);
  txt(18, 11, 'veranda: planters, string lights', 330, 'AMENAGEMENT');
  // stairs: arrival stair foot (SW) and the locked stair up (NE)
  RC(-28, 19, 3, 2, 'ESCALIERS');
  await stair(3000, 2000, -27, 17, -27, 12);
  txt(-26, 14.2, 'STAIR FOOT (arrival)', 330, 'ESCALIERS');
  arrival(-27, 16, 'ARRIVAL');
  await stair(3000, 2000, 27, -14.8, 27, -19.4);
  locked(27, -14, 2.6, false, 'LOCKED until Madanm Flanm is down', 27, -12.6);
  txt(27, -20.9, 'STAIR UP to the 1st floor', 380, 'ESCALIERS');
  STAIRS_UP.push(['GF', 27, -17]);
  txt(-5, 21.8, 'Lost: 50 in total, named, 13 to 20 years old: woken by name, never shot.', 380, 'OBJECTIFS');
  await dimH(-30, 30, 20, -2);
  await dimV(-20, 20, -31, 3);
  north(36, -14);
  end();
}

// =============================================================================================
//  5. FIRST FLOOR (Level 4)   ox 0, oy -160
// =============================================================================================
async function floor1() {
  begin('F1', 0, -160);
  title(0, -33, 'SHEET 5: FIRST FLOOR, TET WOCH AND THE GLASS FLOOR (Level 4)', '60 x 40 m. Two penthouses face each other across a gallery whose floor is bulletproof glass over the pool, 6 m below. ~120 Lost.');
  await wallLoop([[-30, -20], [30, -20], [30, 20], [-30, 20]]);
  await wl(true, -12, -14, 14, [-8, 0, 8]);
  await wl(true, 12, -14, 14, [-8, 0, 8]);
  await wl(false, 0, -30, -12, [-21]);
  await wl(false, 0, 12, 30, [21]);
  await wl(true, 24, -20, -12.25, []);
  await wl(false, -12.25, 24, 30, [25.55]);
  const a = 29.75, b = 12.25, c = 19.75; // inner faces of the 0.5 m walls
  spaceAt([[-a, -c], [-b, -c], [-b, -0.25], [-a, -0.25]], 'WEST PENTHOUSE: LOUNGE + BAR');
  spaceAt([[-a, 0.25], [-b, 0.25], [-b, c], [-a, c]], 'WEST PENTHOUSE: BEDROOM');
  spaceAt([[b, -c], [23.75, -c], [23.75, -12.5], [a, -12.5], [a, -0.25], [b, -0.25]], 'EAST PENTHOUSE: TET WOCH GYM');
  spaceAt([[b, 0.25], [a, 0.25], [a, c], [b, c]], 'EAST PENTHOUSE: BEDROOM');
  spaceAt([[-11.75, -c], [11.75, -c], [11.75, c], [-11.75, c]], 'GALLERY + GLASS FLOOR');
  spaceAt([[24.25, -c], [a, -c], [a, -12.5], [24.25, -12.5]], 'STAIR ROOM');
  // the glass floor over the pool
  R(-9, -12, 9, 12, 'VITRE');
  d.hatch([P(-9, -12), P(9, -12), P(9, 12), P(-9, 12)], { pattern: 'ANSI31', scale: 500, alpha: 0.30, layer: 'VITRE' });
  for (let i = 0; i <= 6; i++) { line(-9 + i * 3, -12, -9 + i * 3, 12, 'VITRE'); line(-9, -12 + i * 4, 9, -12 + i * 4, 'VITRE'); }
  txt(0, 0.5, 'BULLETPROOF GLASS FLOOR 18 x 24 m', 520, 'VITRE');
  txt(0, -0.8, 'over the POOL 6 m below. Bullets cannot break it.', 380, 'VITRE');
  txt(0, -2.0, '1st grenade cracks it, the 2nd collapses it (single-use trap)', 380, 'VITRE');
  // pit stairs (rise to the west edge along the north wall)
  R(-9, -11.6, 2.4, -10.0, 'ESCALIERS');
  txt(-3, -9.3, 'PIT STAIRS (out of the pool)', 330, 'ESCALIERS');
  // furniture
  RC(-24, -10, 3.2, 1.2); RC(-17, -6, 1.2, 3.2); RC(-27, -17.5, 6, 1.0); RC(-26, 6, 2.4, 3.4); RC(-18, 12, 2, 2);
  RC(14, -17, 1, 2); RC(14, -13, 1, 2); RC(27, 8, 3, 2.6); RC(20, 10, 4, 1); RC(20, -6, 3, 1.2); RC(17, 4, 3, 1.2);
  dot(-14, -19.2, 'BREAKER PANEL: hold E (stops the trance)', 'OBJECTIFS', 0.35, 0, 1.3, 330);
  dot(19, -8, 'TET WOCH (helmet, vest)', 'ACTEURS', 0.5, 0, 1.3, 420);
  poly([[-8, -17], [8, -17]], 'PATROUILLE', { lt: 'DASHED' }); txt(0, -17.9, 'Gaspard', 330, 'PATROUILLE');
  poly([[-8, 17], [8, 17]], 'PATROUILLE', { lt: 'DASHED' }); txt(0, 17.9, 'Lucien', 330, 'PATROUILLE');
  // stairs
  arrival(25.55, -10.0, 'ARRIVAL: top of the stairs (NE core door), SAVE POINT');
  await stair(3000, 2000, 27, -14.8, 27, -19.4);
  locked(25.55, -12.25, 2.6, false, 'LOCKED until Tet Woch is down', 26, -10.5);
  txt(27, -20.9, 'STAIR UP to the 2nd floor', 380, 'ESCALIERS');
  STAIRS_UP.push(['1F', 27, -17]);
  await dimH(-30, 30, 20, -2);
  await dimV(-20, 20, -31, 3);
  north(36, -14);
  end();
}

// =============================================================================================
//  6. SECOND FLOOR (Level 5)   ox 85, oy -160
// =============================================================================================
// One example maze (the game builds a different random one each run): row, col pairs that are open.
const OPEN5 = [];
for (let c = 0; c < 5; c++) OPEN5.push([[2, c], [2, c + 1]]);
for (let c = 0; c < 5; c++) if (c !== 2) OPEN5.push([[1, c], [1, c + 1]]);
for (let c = 0; c < 5; c++) if (c !== 1) OPEN5.push([[0, c], [0, c + 1]]);
OPEN5.push([[2, 0], [1, 0]], [[2, 3], [1, 3]], [[1, 2], [0, 2]], [[1, 1], [0, 1]], [[1, 5], [0, 5]]);
const open5 = (a, b) => OPEN5.some(([x, y]) => (x[0] === a[0] && x[1] === a[1] && y[0] === b[0] && y[1] === b[1]) || (x[0] === b[0] && x[1] === b[1] && y[0] === a[0] && y[1] === a[1]));
const cell5 = (r, c) => [-25 + 10 * c, 8 * r];
async function floor2() {
  begin('F2', 85, -160);
  title(0, -34, 'SHEET 6: SECOND FLOOR, ZONBI MANCHET, THE M50 AND THE ALTAR HALL (Level 5)', '60 x 40 m. South: a maze of 3 x 6 rooms with spike traps (EXAMPLE: the game generates a new maze every run). North: the vast altar hall.');
  await wallLoop([[-30, -20], [30, -20], [30, 20], [-30, 20]]);
  await wl(true, 24, -20, -12.25, []); // NE stair core, stacked on every floor
  await wl(false, -12.25, 24, 30, [25.55]);
  await wl(false, -4, -30, 30, [-15, 15]); // maze | hall
  for (let k = 1; k <= 5; k++) await wall(-30 + 10 * k, -4, -30 + 10 * k, 20);
  for (let k = 1; k <= 2; k++) await wall(-30, -4 + 8 * k, 30, -4 + 8 * k);
  for (let k = 1; k <= 5; k++) for (let r = 0; r < 3; r++) if (open5([r, k - 1], [r, k])) await opening(-30 + 10 * k, 8 * r, 2.6);
  for (let k = 1; k <= 2; k++) for (let c = 0; c < 6; c++) if (open5([k - 1, c], [k, c])) await opening(-25 + 10 * c, -4 + 8 * k, 2.6);
  // reserve room behind the hall's west end
  await wl(true, -22, -20, -12, [-16]);
  await wall(-30, -12, -22, -12);
  // rooms
  const nm = { '2,5': 'ARRIVAL (from the 1st floor)', '2,0': 'ARMORY: M50 + AMMUNITION' };
  for (let r = 0; r < 3; r++) for (let c = 0; c < 6; c++) { const [cx, cz] = cell5(r, c); await space(cx, cz + 1.8, nm[`${r},${c}`] || `ROOM ${'ABCDEF'[c]}${r + 1}`); }
  await space(8, -14, 'ALTAR HALL (Zonbi Manchet)');
  await space(-26, -17, 'NWIT RESERVE ROOM (bottle storage)');
  // example room furniture and bones
  for (let r = 0; r < 3; r++) for (let c = 0; c < 6; c++) { const [cx, cz] = cell5(r, c); if ((r + c) % 2 === 1) RC(cx + 1, cz - 1, 2, 0.9); }
  // spike traps
  for (const [r, c] of [[1, 4], [1, 3], [1, 2], [0, 5], [2, 3], [1, 0]]) {
    const [cx, cz] = cell5(r, c);
    R(cx - 2, cz - 1.5, cx + 2, cz + 1.5, 'VERROU', { lt: 'DASHED' });
    txt(cx, cz + 0.2, 'SPIKE TRAP', 330, 'VERROU');
  }
  // armory
  RC(-25, 15.5, 1.4, 0.8, 'OBJECTIFS');
  dot(-25, 16.5, 'M50 CRATE', 'OBJECTIFS', 0.35, 0, 2.5, 380);
  // altar hall
  RC(0, -12, 8, 3, 'AUTEL'); RC(0, -14.5, 12, 1, 'AUTEL');
  for (const x of [-12, -6, 6, 12]) { RC(x, -9, 0.8, 0.8); RC(x, -18, 0.8, 0.8); }
  for (let i = 0; i < 6; i++) circ(-2.5 + i, -11, 0.1, 'AUTEL');
  txt(0, -10.2, 'ALTAR: 6 bottles', 380, 'AUTEL');
  dot(0, -8.2, 'BINDING: hold E to break it (opens the stairs)', 'OBJECTIFS', 0.35, 0, 1.2, 360);
  dot(0, -15.5, 'ZONBI MANCHET rises here', 'ACTEURS', 0.5, 0, -1.0, 420);
  dot(3.8, -12, 'LEDGER (his real name)', 'OBJECTIFS', 0.25, 4, 0.2, 320);
  for (const [x, z] of [[-27, -8], [27, -8], [27, -17]]) { circ(x, z, 0.7, 'MOBILIER'); txt(x, z + 1.2, 'drum', 300); }
  dot(-20, -6.5, 'LEVER (opens the reserve door)', 'OBJECTIFS', 0.3, 2, 1.0, 320);
  // reserve room
  dot(-27, -17.5, 'SOURCE: drink', 'OBJECTIFS', 0.3, 0, -1.0, 300);
  dot(-27, -14, 'SOURCE: sever', 'OBJECTIFS', 0.3, 0, 1.0, 300);
  txt(-26, -19.0, 'HIDDEN ROOM: unlimited health, high Weight cost', 300, 'SECRET');
  locked(-22, -16, 2.6, true, 'RESERVE DOOR hidden until the lever', -16.5, -12.9);
  // stairs
  arrival(25.55, -10.0, 'ARRIVAL: stair down to the 1st floor (NE core door)');
  await stair(3000, 2000, 27, -14.8, 27, -19.4);
  locked(25.55, -12.25, 2.6, false, 'STAIR DOOR sealed until the binding is broken', 26, -10.5);
  txt(27, -20.9, 'STAIR UP to the 3rd floor (Nwit)', 380, 'ESCALIERS');
  STAIRS_UP.push(['2F', 27, -17]);
  txt(0, 21.8, 'Hall: ~150 Lost close in when Ro enters. Zonbi Manchet: 1000 bullets, grenades do nothing, faster with every wound.', 380, 'ACTEURS');
  await dimH(-30, 30, 20, -2.5);
  await dimV(-20, 20, -31, 3);
  north(36, -14);
  end();
}

// =============================================================================================
//  7. THIRD FLOOR: Granmet Nwit's quarters (Level 6)   ox 0, oy -240
// =============================================================================================
const OPEN6 = [[[2, 0], [2, 1]], [[2, 1], [2, 2]], [[2, 2], [2, 3]], [[2, 3], [2, 4]], [[1, 0], [1, 1]], [[1, 1], [1, 2]], [[1, 2], [1, 3]], [[1, 3], [1, 4]], [[0, 0], [0, 1]], [[0, 2], [0, 3]], [[0, 3], [0, 4]], [[2, 0], [1, 0]], [[2, 2], [1, 2]], [[2, 4], [1, 4]], [[1, 1], [0, 1]], [[1, 3], [0, 3]], [[1, 4], [0, 4]], [[1, 0], [0, 0]], [[0, 1], [0, 2]]];
const open6 = (a, b) => OPEN6.some(([x, y]) => (x[0] === a[0] && x[1] === a[1] && y[0] === b[0] && y[1] === b[1]) || (x[0] === b[0] && x[1] === b[1] && y[0] === a[0] && y[1] === a[1]));
const cell6 = (r, c) => [-24 + 12 * c, -13 + 13 * r];
async function floor3() {
  begin('F3', 0, -240);
  title(0, -34, "SHEET 7: THIRD FLOOR, GRANMET NWIT'S QUARTERS (Level 6)", '60 x 39 m, 3 x 5 rooms of 12 x 13 m. Secret panels (red), the bottle stash, the gallery of the taken, the vast bathroom. Nwit flees to the roof at half health.');
  await wallLoop([[-30, -19.5], [30, -19.5], [30, 19.5], [-30, 19.5]]);
  for (let k = 1; k <= 4; k++) await wall(-30 + 12 * k, -19.5, -30 + 12 * k, 19.5);
  for (let k = 1; k <= 2; k++) await wall(-30, -19.5 + 13 * k, 30, -19.5 + 13 * k);
  for (let k = 1; k <= 4; k++) for (let r = 0; r < 3; r++) if (open6([r, k - 1], [r, k])) await opening(-30 + 12 * k, -13 + 13 * r, 2.6);
  for (let k = 1; k <= 2; k++) for (let c = 0; c < 5; c++) if (open6([k - 1, c], [k, c])) await opening(-24 + 12 * c, -19.5 + 13 * k, 2.6);
  await wl(true, 24, -19.5, -12.25, []); // NE stair core, stacked on every floor
  await wl(false, -12.25, 24, 30, [25.55]);
  const nm = { '2,0': 'ANTECHAMBER', '2,1': 'SITTING ROOM', '2,2': 'WARDROBE', '2,3': 'LIBRARY', '2,4': 'STUDY', '1,0': "NWIT'S BEDROOM", '1,1': 'GALLERY OF THE TAKEN', '1,2': 'RITUAL CHAMBER', '1,3': 'VAST BATHROOM: JACUZZI', '1,4': 'BATH ANNEX', '0,0': 'SHRINE ROOM 1', '0,1': 'SHRINE ROOM 2', '0,2': 'SHRINE ROOM 3', '0,3': 'SHRINE ROOM 4', '0,4': 'ROOF STAIR ROOM (arrival)' };
  for (let r = 0; r < 3; r++) for (let c = 0; c < 5; c++) { const [cx, cz] = cell6(r, c); await space(cx, cz + 2.5, nm[`${r},${c}`]); }
  // furniture (from level6.rs)
  let [cx, cz] = cell6(2, 1); RC(cx - 3, cz + 3.5, 3.6, 1.2); RC(cx + 4, cz, 1.2, 3.4); RC(cx, cz, 2, 1);
  [cx, cz] = cell6(2, 2); for (let i = 0; i < 5; i++) RC(cx - 4.4 + i * 2.2, cz + 5.9, 1.8, 0.7);
  [cx, cz] = cell6(2, 3); RC(cx, cz - 5.8, 9, 0.6); RC(cx + 2, cz + 1, 3, 1.4);
  [cx, cz] = cell6(2, 4); RC(cx + 2, cz, 3, 1.4);
  [cx, cz] = cell6(1, 0); RC(cx - 1, cz - 2, 4.4, 3.4); RC(cx + 3, cz - 4, 0.9, 0.9); RC(cx - 5, cz - 4, 0.9, 0.9);
  [cx, cz] = cell6(1, 1); for (let i = 0; i < 8; i++) RC(cx - 5 + i * 1.4, cz - 6.1, 0.7, 0.1, 'OBJECTIFS'); RC(cx, cz + 1, 3, 1);
  txt(cx, cz - 4.8, 'framed photos of the 51 taken kids', 330, 'OBJECTIFS');
  [cx, cz] = cell6(1, 2); RC(cx, cz, 3.2, 1.6, 'AUTEL');
  for (let i = 0; i < 12; i++) { const a = i / 12 * Math.PI * 2; circ(cx + Math.cos(a) * 4, cz + Math.sin(a) * 3.6, 0.14, 'AUTEL'); }
  txt(cx, cz + 0.2, 'black slab', 300, 'AUTEL');
  [cx, cz] = cell6(1, 3); circ(cx, cz, 3.6, 'EAU'); circ(cx, cz, 3.2, 'EAU');
  for (const [dx, dz] of [[-5, -5.5], [5, -5.5], [-5, 5.5], [5, 5.5]]) RC(cx + dx, cz + dz, 0.9, 0.9);
  txt(cx, cz, 'JACUZZI', 500, 'EAU');
  [cx, cz] = cell6(1, 4); RC(cx + 3, cz - 2, 2.6, 4);
  for (let c = 0; c < 4; c++) { [cx, cz] = cell6(0, c); RC(cx, cz - 5.8, 8, 0.6); RC(cx - 3.5, cz + 2, 0.5, 0.5); RC(cx + 3.5, cz + 2, 0.5, 0.5); }
  // voice horns, notes
  for (const [x, z] of [[-18, -6.4], [6, -6.4], [18, 6.4], [-6, 6.4]]) { dot(x, z, null, 'SECRET', 0.2); }
  txt(0, -5.4, 'voice horns in the walls (Nwit is a man and a tube, not a spirit)', 300, 'SECRET');
  // bottle stash
  const STASH = [[0, 0, 3, -3], [0, 2, -3, 3], [0, 4, 3, 3], [1, 1, -3, -3], [1, 3, 3, 2], [2, 1, 2.5, -3], [2, 3, -3, 3], [2, 4, 3, -3]];
  STASH.forEach(([r, c, dx, dz], i) => { const [bx, bz] = cell6(r, c); circ(bx + dx, bz + dz, 0.25, 'OBJECTIFS'); txt(bx + dx, bz + dz + 0.8, `BOTTLE ${i + 1}`, 280, 'OBJECTIFS'); });
  txt(0, 24.2, "NWIT'S STASH: 8 health bottles. While any remain he drinks and comes back. Take them all and he is mortal.", 400, 'OBJECTIFS');
  // secret panels: four per room
  for (let r = 0; r < 3; r++) for (let c = 0; c < 5; c++) { const [px, pz] = cell6(r, c); for (const [dx, dz] of [[0, -5.2], [0, 5.2], [-4.8, 0], [4.8, 0]]) diamond(px + dx, pz + dz); }
  txt(0, 25.4, 'RED DIAMONDS = SECRET PANELS (60). Hunt him by sound and the "Tip, tip!" cue; the glow in the wall shows where he steps out.', 400, 'SECRET');
  // arrival and the roof stair
  arrival(25.55, -10.0, 'ARRIVAL: stair down to the 2nd floor (NE core door)');
  await stair(3000, 2000, 27, -14.8, 27, -19.2);
  locked(25.55, -12.25, 2.6, false, 'ROOF STAIR DOOR sealed until Nwit flees', 25, -10.6);
  txt(27, -20.6, 'STAIR UP to the ROOF', 380, 'ESCALIERS');
  STAIRS_UP.push(['3F', 27, -16]);
  await dimH(-30, 30, 19.5, -2.5);
  await dimV(-19.5, 19.5, -31, 3);
  north(36, -14);
  end();
}

// =============================================================================================
//  8. ROOF (Level 7)   ox 85, oy -240
// =============================================================================================
async function roof() {
  begin('ROOF', 85, -240);
  title(0, -22, 'SHEET 8: THE ROOF, FINAL SCENES WITH GRANMET NWIT (Level 7)', '49 x 33 m, parapet 1.2 m. Four endings: hand him over (helicopter), take his place, kill him, or walk away.');
  await wallLoop([[-24.4, -16.4], [24.4, -16.4], [24.4, 16.4], [-24.4, 16.4]], 800);
  await opening(24.4, 0, 2.6);
  await space(0, 6, 'ROOFTOP: final duel');
  circ(-12, -10, 2.2, 'MOBILIER'); txt(-12, -10, 'WATER TANK', 330);
  for (const [x, z, sx, sz] of [[12, -10, 3, 2], [14, 10, 3, 2], [4, 9, 1.6, 1.6], [6, -8, 1.6, 1.6], [-16, 8, 2, 1.2], [-4, -12, 1.4, 1.4]]) RC(x, z, sx, sz);
  txt(12, -10, 'AC unit', 300); txt(14, 10, 'AC unit', 300);
  circ(-20, 0, 0.3, 'MOBILIER'); txt(-20, 1.2, 'antenna mast 9 m (red light)', 300);
  circ(-6, 0, 2.6, 'AUTEL', { lt: 'DASHED' });
  dot(-6, 0, "NWIT STARTS HERE: on all fours, candle ring", 'ACTEURS', 0.5, 0, -3.8, 380);
  RC(-7, 0.6, 1.0, 0.7, 'MOBILIER'); txt(-7, 1.5, 'stain', 260);
  arrival(20, 0, 'ARRIVAL (from the stair)');
  // the stair head house on top of the NE core (24..30 x -20..-12.25): same stack as every floor
  R(24, -20, 30, -12.25, 'MURS');
  txt(27, -16, 'STAIR HEAD HOUSE (NE core)', 320, 'ESCALIERS');
  STAIRS_UP.push(['ROOF', 27, -16]);
  txt(22, -10, 'The game still starts the duel at (20, 0): about 12 m from the head-house door.', 300, 'PROPOSITION');
  // the helicopter
  circ(-6, 0, 4, 'HELICO', { lt: 'DASHED' });
  txt(-6, 5.6, 'HELICOPTER HOIST ZONE (hovers 8 m over wherever Nwit ends)', 330, 'HELICO');
  line(-45, 0, -8, 0, 'HELICO', { lt: 'DASHED' }); txt(-34, 1, 'approach from the far west', 300, 'HELICO');
  line(-6, 0, 28, -20, 'HELICO', { lt: 'DASHED' }); txt(18, -11, 'leaves north-east', 300, 'HELICO');
  txt(0, 19.8, 'First move is a trap: hand him a bottle (fake death + lunge), shoot (he disarms Ro), or rush to cuff (counter-stab).', 360, 'ACTEURS');
  txt(0, 21.0, 'At a fifth of his health he kneels and offers money. Money position is chosen in the code (MoneyAt).', 360, 'ACTEURS');
  await dimH(-24.4, 24.4, 16.4, -2.2);
  await dimV(-16.4, 16.4, -25.5, 3);
  north(34, -15);
  end();
}

// =============================================================================================
//  9. STAIR STACK: a side section of the whole building
// =============================================================================================
async function stack() {
  begin('STACK', 0, -330);
  const H = 3.0; // floor to floor
  title(0, -27, 'SHEET 9: STAIR STACK, A SECTION THROUGH THE BUILDING (schematic)', 'Floor to floor 3.0 m. Each stair is LOCKED (red) until the boss of its floor is beaten. The four stairs now STACK in the NE core (x 24..30), as in the game.');
  const floors = [['GROUND', 0], ['1ST FLOOR', H], ['2ND FLOOR', 2 * H], ['3RD FLOOR (NWIT)', 3 * H], ['ROOF', 4 * H]];
  const zs = (e) => -e * 1.0; // drawing z for an elevation (so up is up)
  for (const [n, e] of floors) {
    line(-30, zs(e), 30, zs(e), 'MURS'); line(-30, zs(e) - 0.2, 30, zs(e) - 0.2, 'MURS');
    txt(-33, zs(e) - 0.4, n, 420, 'TEXTE', { align: 'right' });
    txt(33.5, zs(e) - 0.4, `+${e.toFixed(2)}`, 360, 'COTES', { align: 'left' });
  }
  line(-30, zs(0), -30, zs(4 * H), 'MURS'); line(30, zs(0), 30, zs(4 * H), 'MURS');
  // the pool below the glass floor
  R(-9, zs(H) + 0, 9, zs(H) + 6, 'EAU'); // pit 6 m below the 1st floor level (drawn up the sheet = down in the building)
  txt(0, zs(H) + 3, 'POOL 6 m deep under the glass floor', 360, 'EAU');
  line(-9, zs(H) + 0.05, 9, zs(H) + 0.05, 'VITRE'); txt(0, zs(H) - 0.9, 'glass floor', 330, 'VITRE');
  // ground floor basement label
  txt(0, zs(0) + 6.5, 'sunken pool bottom at -3.0 m (PROPOSAL, to fix issue 2)', 300, 'PROPOSITION');
  // stairs between floors (zigzag lines) with locks
  const links = [
    [0, H, 27, 'GF to 1F: NE stair room, LOCKED until Madanm Flanm is down'],
    [H, 2 * H, 27, '1F to 2F: NE stair room, LOCKED until Tet Woch is down'],
    [2 * H, 3 * H, 27, '2F to 3F: NE stair room, LOCKED until the binding is broken'],
    [3 * H, 4 * H, 27, '3F to ROOF: NE stair room, LOCKED until Nwit flees'],
  ];
  links.forEach(([e0, e1, x, label], i) => {
    const n = 10, w = 4.6;
    const pts = [];
    for (let k = 0; k <= n; k++) { pts.push([x - w / 2 + (w * k) / n, zs(e0) - 0.2 - ((e1 - e0 - 0.2) * k) / n]); if (k < n) pts.push([x - w / 2 + (w * (k + 1)) / n, zs(e0) - 0.2 - ((e1 - e0 - 0.2) * k) / n]); }
    poly(pts, 'ESCALIERS');
    RC(x, zs(e1) + 0.2, 3.0, 0.5, 'VERROU');
    txt(x, zs(e0) - 1.7, label, 330, 'VERROU', x > 15 ? { align: 'center' } : {});
    // label at the left for readability
  });
  txt(-14, zs(0) - 0.9, 'Ground floor has TWO wings (A: maze, B: party) side by side, see sheets 3 and 4', 330);
  txt(-14, zs(4 * H) - 2.5, 'Roof door on the east parapet (24.4, 0)', 330);
  // the helicopter above
  poly([[-2, zs(4 * H) - 8], [2, zs(4 * H) - 8], [2, zs(4 * H) - 7], [-2, zs(4 * H) - 7]], 'HELICO', { closed: true });
  txt(0, zs(4 * H) - 9.5, 'helicopter (opening strike from the air, ending hoist from the roof)', 320, 'HELICO');
  end();
}

// --------------------------------------------------------------------------- print sheets (A0), one per drawing
// [layout name, title block text, model centre of the viewport (mm), scale 1:N]
const SHEETS = [
  ['S0-NOTES', '0 Notes and schedule', [-72000, 10000], 100, 'NOTES'],
  ['S1-SITE', '1 Outdoor landscape (Level 1)', [0, -4000], 150, 'SITE'],
  ['S2-COCKPIT', '2 Helicopter cockpit (proposal)', [85000, -2750], 20, 'HELI'],
  ['S3-GROUND-A', '3 Ground floor A: maze and altar (Level 2)', [0, -80000], 100, 'GF1'],
  ['S4-GROUND-B', '4 Ground floor B: party wing and pool (Level 3)', [85000, -80000], 100, 'GF2'],
  ['S5-FIRST', '5 First floor: Tet Woch, glass floor (Level 4)', [0, -160000], 100, 'F1'],
  ['S6-SECOND', '6 Second floor: Zonbi Manchet, M50 (Level 5)', [85000, -160000], 100, 'F2'],
  ['S7-THIRD', '7 Third floor: Granmet Nwit quarters (Level 6)', [0, -240000], 100, 'F3'],
  ['S8-ROOF', '8 Roof: final scenes (Level 7)', [85000, -240000], 100, 'ROOF'],
  ['S9-STACK', '9 Stair stack: section of the building', [0, -319500], 100, 'STACK'],
];
async function sheets() {
  for (const [name] of SHEETS) {
    ck(`layout ${name}`, await d.run(`LAYOUT N ${name}`));
    const L = d.doc.layouts.find((l) => l.name === name);
    L.paper = 'A0';
  }
  for (const [name, , vc, scale, pre] of SHEETS) {
    const L = d.doc.layouts.find((l) => l.name === name);
    const vp = L.entities.find((e) => e.type === 'viewport');
    // only this drawing is visible in its sheet: freeze the layers of every other floor
    const frozen = d.doc.layers.map((l) => l.name).filter((n) => /^[A-Z0-9]+-/.test(n) && !n.startsWith(pre + '-'));
    Object.assign(vp, { p: [594.5, 446], w: 1140, h: 760, vc, vs: scale, frozen });
  }
  d.doc.layouts = d.doc.layouts.filter((l) => l.name !== 'Layout1');
  for (const [name, t] of SHEETS) {
    d.doc.setSpace(name);
    ck(`titleblock ${name}`, await d.run(`TITLEBLOCK "WOUT SAN RETOU: THE BUILDING" "${t}"`));
  }
  d.doc.setSpace('model');
}

// --------------------------------------------------------------------------- run
const t0 = Date.now();
await notes();
await site();
await cockpit();
await groundA();
await groundB();
await floor1();
await floor2();
await floor3();
await roof();
await stack();
await sheets();
console.log('built in', ((Date.now() - t0) / 1000).toFixed(1), 's; failures:', bad);
const s = d.summary({ limit: 0 });
console.log(JSON.stringify({ count: s.count, extents: s.extents, byType: s.byType }));
const out = path.join(here, 'wout-san-retou-building');
console.log('saved', await d.save(out + '.des'));
console.log('saved', await d.save(out + '.pdf', { layouts: true }));
process.exit(0);
