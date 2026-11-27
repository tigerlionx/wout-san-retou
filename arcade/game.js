/* Wout San Retou: Arcade. A first-person shoot-'em-up in seven levels, from the yard to the rooftop.
   Original code, no libraries. Music is plain MIDI played by a tiny Web Audio synth; gunshots are synthesised.
   All characters, places and events are fictional. */
'use strict';
(() => {
const W = 480, H = 300;
const canvas = document.getElementById('game');
canvas.width = W; canvas.height = H;
const ctx = canvas.getContext('2d', { alpha: false });
const img = ctx.createImageData(W, H);
const buf = new Uint32Array(img.data.buffer);
const TAU = Math.PI * 2;
const clamp = (v, a, b) => Math.max(a, Math.min(b, v));
const rand = (a, b) => a + Math.random() * (b - a);
const rint = (a, b) => Math.floor(rand(a, b + 1));
const pick = (a) => a[Math.floor(Math.random() * a.length)];
const mk = (w, h) => { const c = document.createElement('canvas'); c.width = w; c.height = h; return c; };

/* ---------- progress, chances and the OK Music LionCoin wallet ----------
   Guests keep their level and daily chances in this browser and earn nothing.
   Signed-in players (the same OK Music login) earn and spend REAL LionCoin in wallets/{uid}, the same
   document the rest of the site uses. Amounts below are in tenths of a LionCoin. */
const DAILY = 20;           // chances per day
const CONTINUE_COST = 20;   // 2.0 LionCoin
const KILL_T = 1;           // 0.1 LionCoin per kill
const BULLET_HIT = 100 / 6; // health one enemy bullet takes: the same as in the 3D game
const HEAD_HIT = 50;        // a bullet to the head, as in the 3D game
const ROCKET_HIT = 50;      // a bazooka rocket takes half of the full 100%
const INV_EVERY = 20, INV_TIME = 10; // every 20 kills: 10 seconds of invincibility
const CROUCH_CAN_SHOOT = false;      // crouched behind cover you hide; stand up to shoot
const COVER = 7, COVER_H = 0.34;     // low obstacles (sandbags, crates): you shoot over them standing, hide behind them crouched
const WP = {
  knife:   { name: 'KNIFE', cd: 0.42, melee: true },
  glock:   { name: 'GLOCK', cd: 0.19, mag: 15, res: 90, pk: 15, rl: 1.0, dmg: 1, snd: 'glock', kick: 0.8 },
  magnum:  { name: 'MAGNUM 747', cd: 0.7, mag: 6, res: 24, pk: 6, rl: 1.5, dmg: 3, snd: 'magnum', kick: 2.2 },
  shotgun: { name: 'SHOTGUN', cd: 0.95, mag: 6, res: 24, pk: 6, rl: 1.7, dmg: 1, pellets: 8, spread: 34, snd: 'shotgun', kick: 2.6 },
  ak:      { name: 'AK-47', cd: 0.095, cdSingle: 0.16, mag: 30, res: 90, pk: 30, rl: 1.1, dmg: 1, snd: 'ak', kick: 1 },
  grenade: { name: 'GRENADE', cd: 0.9, grenade: true },
};
const WORDER = ['knife', 'glock', 'magnum', 'shotgun', 'ak', 'grenade'];
function today() { return new Date().toISOString().slice(0, 10); } // UTC day, same as the site's daily-login reward
const save = { chances: DAILY, day: today(), level: 1, best: 0 };
function loadSave() {
  try { const s = JSON.parse(localStorage.getItem('wsr_arcade_v1') || 'null'); if (s) { save.level = s.level || 1; save.best = s.best || 0; if (s.day === today()) { save.day = s.day; save.chances = s.chances; } } } catch (e) {}
}
function rollDay() { if (save.day !== today()) { save.day = today(); save.chances = DAILY; persist(); } }
function persist() { try { localStorage.setItem('wsr_arcade_v1', JSON.stringify(save)); } catch (e) {} }

const Acct = {
  uid: null, db: null, auth: null, bal: 0, chances: DAILY, earnedT: 0, pendingT: 0, day: today(), err: '', unsub: null, ready: false,
  init() {
    try {
      if (typeof fbAuth === 'undefined' || typeof fbDB === 'undefined') return;
      this.auth = fbAuth; this.db = fbDB; this.hook();
    } catch (e) {}
  },
  hook() {
    this.auth.onAuthStateChanged((user) => {
      if (this.unsub) { this.unsub(); this.unsub = null; }
      this.uid = user ? user.uid : null; this.ready = false; this.pendingT = 0;
      if (!user) { this.bal = 0; return; }
      this.unsub = this.db.collection('wallets').doc(this.uid).onSnapshot((s) => {
        const d = s.exists ? s.data() : {}; const same = d.arcadeDay === today();
        this.bal = parseFloat(d.balance) || 0; this.chances = same ? (d.arcadeChances == null ? DAILY : d.arcadeChances) : DAILY; this.earnedT = same ? (d.arcadeEarnedT || 0) : 0; this.day = today(); this.ready = true;
      }, (e) => { this.err = String(e && e.message || e); });
    });
  },
  get signedIn() { return !!this.uid; },
  refreshDay() { if (this.day !== today()) { this.day = today(); this.chances = DAILY; this.earnedT = 0; } rollDay(); },
  coinsT() { return this.uid ? Math.round(this.bal * 10) + this.pendingT : 0; },
  chancesLeft() { this.refreshDay(); return this.uid ? this.chances : save.chances; },
  earn(t) { // a kill, a Lost one woken
    if (!this.uid || t <= 0) return;
    this.pendingT += t; if (this.pendingT >= 10) this.flush();
  },
  penalize(t) { if (this.uid) this.pendingT = Math.max(0, this.pendingT - t); },
  async tx(fn) { // one atomic read-modify-write of the player's own wallet document
    const ref = this.db.collection('wallets').doc(this.uid), F = firebase.firestore.FieldValue; let out = null;
    await this.db.runTransaction(async (t) => {
      const s = await t.get(ref), d = s.exists ? s.data() : {}, same = d.arcadeDay === today();
      const st = { bal: parseFloat(d.balance) || 0, chances: same ? (d.arcadeChances == null ? DAILY : d.arcadeChances) : DAILY, earnedT: same ? (d.arcadeEarnedT || 0) : 0 };
      const r = fn(st); const upd = { arcadeDay: today(), arcadeChances: r.chances, arcadeEarnedT: r.earnedT };
      if (r.dBal > 0) { upd.balance = F.increment(r.dBal); upd.totalEarned = F.increment(r.dBal); } else if (r.dBal < 0) { upd.balance = F.increment(r.dBal); upd.totalSpent = F.increment(-r.dBal); }
      if (s.exists) t.update(ref, upd); else t.set(ref, Object.assign({}, upd, { balance: Math.max(0, r.dBal), totalEarned: Math.max(0, r.dBal), totalSpent: 0, isPublic: false, streak: 0, lastLoginDate: '', lastMilestone: 0, createdAt: Date.now() }));
      out = r;
    });
    return out;
  },
  note(type, amount, description) { try { this.db.collection('wallets').doc(this.uid).collection('transactions').add({ type, amount, description, ref: '', createdAt: Date.now() }).catch(() => {}); } catch (e) {} },
  async flush() { // one wallet write at a time; kills that arrive meanwhile wait for the next one
    if (!this.uid || this.pendingT <= 0) return;
    if (this.flushing) { this.again = true; return; }
    this.flushing = true; const take = this.pendingT; this.pendingT = 0; let paid = 0;
    try {
      const r = await this.tx((st) => { paid = take; return { dBal: Math.round(paid) / 10, chances: st.chances, earnedT: st.earnedT + paid }; });
      this.earnedT = r.earnedT; if (paid > 0) this.note('arcade_kills', paid / 10, 'Wout San Retou arcade');
    } catch (e) { this.pendingT += take; this.err = String(e && e.message || e); }
    this.flushing = false;
    if (this.again || this.pendingT >= 10) { this.again = false; if (this.pendingT > 0 && !this.err) this.flush(); }
  },
  async useChance() { // a loss
    if (!this.uid) { save.chances = Math.max(0, save.chances - 1); persist(); return; }
    this.chances = Math.max(0, this.chances - 1);
    try { const r = await this.tx((st) => ({ dBal: 0, chances: Math.max(0, st.chances - 1), earnedT: st.earnedT })); this.chances = r.chances; } catch (e) { this.err = String(e && e.message || e); }
  },
  async payChances() { // 2 LionCoin for 20 more chances; true on success
    if (!this.uid) return false;
    await this.flush();
    try {
      const r = await this.tx((st) => { if (st.bal < CONTINUE_COST / 10 - 1e-9) throw new Error('Insufficient balance'); return { dBal: -CONTINUE_COST / 10, chances: st.chances + DAILY, earnedT: st.earnedT }; });
      this.chances = r.chances; this.note('arcade_chances', -CONTINUE_COST / 10, 'Wout San Retou arcade: 20 more chances'); return true;
    } catch (e) { this.err = String(e && e.message || e); return false; }
  },
};
setInterval(() => Acct.flush(), 30000);
document.addEventListener('visibilitychange', () => { if (document.hidden) Acct.flush(); });
window.addEventListener('pagehide', () => Acct.flush());

/* ---------- audio: synthesised sfx + MIDI player ---------- */
const A = {
  ctx: null, master: null, musicG: null, sfxG: null, noise: null, muted: false, ambient: null,
  init() {
    if (this.ctx) return;
    const AC = window.AudioContext || window.webkitAudioContext; if (!AC) return;
    this.ctx = new AC();
    const c = this.ctx;
    const comp = c.createDynamicsCompressor(); comp.connect(c.destination);
    this.master = c.createGain(); this.master.gain.value = this.muted ? 0 : 0.8; this.master.connect(comp);
    this.musicG = c.createGain(); this.musicG.gain.value = 0.32; this.musicG.connect(this.master);
    this.sfxG = c.createGain(); this.sfxG.gain.value = 0.9; this.sfxG.connect(this.master);
    const n = c.createBuffer(1, c.sampleRate, c.sampleRate), d = n.getChannelData(0);
    for (let i = 0; i < d.length; i++) d[i] = Math.random() * 2 - 1;
    this.noise = n;
  },
  resume() { if (this.ctx && this.ctx.state === 'suspended') this.ctx.resume(); },
  setMuted(m) { this.muted = m; if (this.master) this.master.gain.value = m ? 0 : 0.8; },
  out(pan) {
    const c = this.ctx; if (!c.createStereoPanner || !pan) return this.sfxG;
    const p = c.createStereoPanner(); p.pan.value = clamp(pan, -1, 1); p.connect(this.sfxG); return p;
  },
  burst(t, dur, type, freq, q, gain, dest) {
    const c = this.ctx, s = c.createBufferSource(); s.buffer = this.noise; s.loop = true;
    const f = c.createBiquadFilter(); f.type = type; f.frequency.value = freq; f.Q.value = q;
    const g = c.createGain(); g.gain.setValueAtTime(gain, t); g.gain.exponentialRampToValueAtTime(0.001, t + dur);
    s.connect(f); f.connect(g); g.connect(dest); s.start(t, Math.random()); s.stop(t + dur + 0.02);
  },
  tone(t, dur, type, f0, f1, gain, dest) {
    const c = this.ctx, o = c.createOscillator(); o.type = type; o.frequency.setValueAtTime(f0, t);
    if (f1 !== f0) o.frequency.exponentialRampToValueAtTime(Math.max(1, f1), t + dur);
    const g = c.createGain(); g.gain.setValueAtTime(gain, t); g.gain.exponentialRampToValueAtTime(0.001, t + dur);
    o.connect(g); g.connect(dest); o.start(t); o.stop(t + dur + 0.02);
  },
  sweep(t, dur, f0, f1, q, gain, dest) {
    const c = this.ctx, s = c.createBufferSource(); s.buffer = this.noise; s.loop = true;
    const f = c.createBiquadFilter(); f.type = 'bandpass'; f.Q.value = q; f.frequency.setValueAtTime(f0, t); f.frequency.exponentialRampToValueAtTime(f1, t + dur);
    const g = c.createGain(); g.gain.setValueAtTime(0.0001, t); g.gain.linearRampToValueAtTime(gain, t + dur * 0.3); g.gain.exponentialRampToValueAtTime(0.001, t + dur);
    s.connect(f); f.connect(g); g.connect(dest); s.start(t, Math.random()); s.stop(t + dur + 0.02);
  },
  sfx(name, vol = 1, pan = 0) {
    if (!this.ctx || this.muted) return;
    const c = this.ctx, t = c.currentTime, d = this.out(pan);
    const v = (x) => x * vol;
    switch (name) {
      case 'pistol': this.burst(t, 0.1, 'bandpass', 1800, 0.8, v(0.9), d); this.tone(t, 0.13, 'sine', 170, 45, v(0.9), d); break;
      case 'ak': this.burst(t, 0.07, 'bandpass', 3000, 0.6, v(0.95), d); this.tone(t, 0.1, 'sawtooth', 160, 50, v(0.45), d); this.tone(t, 0.03, 'square', 1200, 300, v(0.14), d); break;
      case 'glock': this.burst(t, 0.06, 'bandpass', 2400, 0.9, v(0.85), d); this.tone(t, 0.08, 'square', 420, 110, v(0.2), d); this.tone(t, 0.07, 'sine', 190, 70, v(0.5), d); break;
      case 'magnum': this.burst(t, 0.16, 'bandpass', 1100, 0.7, v(1), d); this.burst(t, 0.45, 'lowpass', 500, 0.6, v(0.9), d); this.tone(t, 0.4, 'sine', 95, 28, v(1), d); this.burst(t + 0.11, 0.35, 'lowpass', 380, 0.5, v(0.35), d); break;
      case 'shotgun': this.burst(t, 0.3, 'lowpass', 1500, 0.5, v(1), d); this.burst(t, 0.12, 'highpass', 1200, 0.6, v(0.6), d); this.tone(t, 0.3, 'sine', 80, 26, v(1), d); this.burst(t + 0.5, 0.04, 'highpass', 2500, 0.8, v(0.5), d); this.burst(t + 0.64, 0.05, 'highpass', 1800, 0.8, v(0.6), d); break;
      case 'knife': this.sweep(t, 0.16, 900, 3500, 1.2, v(0.45), d); break;
      case 'stab': this.burst(t, 0.09, 'lowpass', 700, 0.8, v(0.8), d); this.tone(t, 0.1, 'triangle', 150, 60, v(0.6), d); break;
      case 'throw': this.sweep(t, 0.22, 400, 1300, 1, v(0.35), d); break;
      case 'clink': this.tone(t, 0.06, 'triangle', 1800, 900, v(0.2), d); break;
      case 'rocket': this.sweep(t, 0.9, 300, 1900, 0.8, v(0.6), d); this.tone(t, 0.8, 'sawtooth', 90, 190, v(0.25), d); break;
      case 'shield': this.tone(t, 0.12, 'sine', 1400, 1900, v(0.22), d); this.tone(t + 0.05, 0.12, 'sine', 2100, 2100, v(0.15), d); break;
      case 'ricochet': this.tone(t, 0.12, 'sawtooth', 1600, 700, v(0.22), d); this.burst(t, 0.05, 'highpass', 4000, 0.7, v(0.4), d); break;
      case 'power': [523, 659, 784, 1047, 1319].forEach((f, i) => this.tone(t + i * 0.07, 0.14, 'square', f, f, v(0.16), d)); break;
      case 'powerdown': [1047, 784, 659, 523].forEach((f, i) => this.tone(t + i * 0.08, 0.14, 'square', f, f, v(0.14), d)); break;
      case 'oh': { // the player's cry when he is hit: a short "Oh!"
        const f0 = 150, o = c.createOscillator(); o.type = 'sawtooth'; o.frequency.setValueAtTime(f0 * 1.25, t); o.frequency.exponentialRampToValueAtTime(f0 * 0.85, t + 0.4);
        const g = c.createGain(); g.gain.setValueAtTime(0.0001, t); g.gain.linearRampToValueAtTime(v(0.6), t + 0.03); g.gain.exponentialRampToValueAtTime(0.001, t + 0.42);
        [[520, 7, 1], [900, 9, 0.6], [2400, 10, 0.15]].forEach(([fr, q, gn]) => { const b = c.createBiquadFilter(); b.type = 'bandpass'; b.frequency.setValueAtTime(fr * 1.15, t); b.frequency.linearRampToValueAtTime(fr * 0.85, t + 0.4); b.Q.value = q; const gg = c.createGain(); gg.gain.value = gn * 2.5; o.connect(b); b.connect(gg); gg.connect(g); });
        g.connect(d); o.start(t); o.stop(t + 0.45); this.burst(t, 0.06, 'bandpass', 1500, 0.8, v(0.15), d); break;
      }
      case 'enemy': this.burst(t, 0.14, 'lowpass', 1400, 0.7, v(0.7), d); this.tone(t, 0.12, 'sine', 140, 50, v(0.6), d); break;
      case 'boom': this.burst(t, 0.5, 'lowpass', 600, 0.7, v(1), d); this.tone(t, 0.5, 'sine', 90, 30, v(1), d); break;
      case 'hit': this.burst(t, 0.05, 'highpass', 1500, 0.7, v(0.5), d); this.tone(t, 0.08, 'triangle', 320, 120, v(0.4), d); break;
      case 'head': this.burst(t, 0.07, 'highpass', 2500, 0.7, v(0.7), d); this.tone(t, 0.15, 'square', 900, 300, v(0.3), d); break;
      case 'die': this.tone(t, 0.4, 'sawtooth', 220, 45, v(0.35), d); this.burst(t, 0.2, 'lowpass', 800, 0.7, v(0.4), d); break;
      case 'groan': {
        const f = rand(55, 95), o = c.createOscillator(); o.type = 'sawtooth'; o.frequency.setValueAtTime(f, t); o.frequency.linearRampToValueAtTime(f * rand(0.6, 0.85), t + 0.7);
        const lp = c.createBiquadFilter(); lp.type = 'lowpass'; lp.frequency.value = 380; const g = c.createGain();
        g.gain.setValueAtTime(0.0001, t); g.gain.linearRampToValueAtTime(v(0.35), t + 0.15); g.gain.exponentialRampToValueAtTime(0.001, t + 0.75);
        o.connect(lp); lp.connect(g); g.connect(d); o.start(t); o.stop(t + 0.8); break;
      }
      case 'roar': this.tone(t, 1.1, 'sawtooth', 70, 38, v(0.6), d); this.tone(t, 1.1, 'sawtooth', 73, 40, v(0.5), d); this.burst(t, 0.9, 'lowpass', 500, 0.7, v(0.5), d); break;
      case 'woy': this.tone(t, 0.35, 'sine', 180, 1400, v(0.4), d); this.tone(t + 0.02, 0.3, 'triangle', 90, 700, v(0.3), d); break;
      case 'pickup': [660, 880, 1100].forEach((f, i) => this.tone(t + i * 0.06, 0.12, 'square', f, f, v(0.18), d)); break;
      case 'coin': this.tone(t, 0.08, 'square', 988, 988, v(0.12), d); this.tone(t + 0.06, 0.18, 'square', 1319, 1319, v(0.12), d); break;
      case 'reload': this.burst(t, 0.04, 'highpass', 3000, 0.7, v(0.5), d); this.burst(t + 0.5, 0.05, 'highpass', 2000, 0.7, v(0.6), d); break;
      case 'click': this.burst(t, 0.03, 'highpass', 3500, 0.7, v(0.4), d); break;
      case 'hurt': this.tone(t, 0.18, 'sine', 120, 50, v(0.7), d); this.burst(t, 0.12, 'lowpass', 700, 0.7, v(0.5), d); break;
      case 'warn': this.tone(t, 0.5, 'square', 300, 900, v(0.1), d); break;
      case 'drink': this.burst(t, 0.3, 'bandpass', 700, 1.2, v(0.3), d); break;
      case 'name': this.tone(t, 0.25, 'triangle', 440, 660, v(0.3), d); this.tone(t + 0.2, 0.3, 'triangle', 660, 880, v(0.3), d); break;
    }
  },
  ambience(kind) {
    if (!this.ctx) return;
    if (this.ambient) { try { this.ambient.stop(); } catch (e) {} this.ambient = null; }
    const c = this.ctx, s = c.createBufferSource(); s.buffer = this.noise; s.loop = true;
    const f = c.createBiquadFilter(); f.type = kind === 'wind' ? 'bandpass' : 'lowpass'; f.frequency.value = kind === 'wind' ? 420 : 160; f.Q.value = 0.6;
    const g = c.createGain(); g.gain.value = kind === 'wind' ? 0.07 : 0.05;
    const lfo = c.createOscillator(); lfo.frequency.value = 0.13; const lg = c.createGain(); lg.gain.value = kind === 'wind' ? 220 : 50;
    lfo.connect(lg); lg.connect(f.frequency); lfo.start();
    s.connect(f); f.connect(g); g.connect(this.musicG); s.start(); this.ambient = s;
  },
};

/* MIDI player */
const M = {
  cache: {}, cur: null, timer: null, name: '', rate: 1,
  parse(ab) {
    const d = new DataView(ab); let p = 0;
    const u32 = () => { const v = d.getUint32(p); p += 4; return v; };
    const u16 = () => { const v = d.getUint16(p); p += 2; return v; };
    p += 4; u32(); u16(); const nt = u16(), div = u16();
    const notes = []; let usq = 500000;
    for (let t = 0; t < nt; t++) {
      p += 4; const len = u32(), end = p + len; let tick = 0, run = 0; const on = {}; const prog = {};
      while (p < end) {
        let dt = 0, b; do { b = d.getUint8(p++); dt = (dt << 7) | (b & 0x7f); } while (b & 0x80);
        tick += dt; let st = d.getUint8(p);
        if (st < 0x80) st = run; else p++;
        if (st === 0xff) { const ty = d.getUint8(p++); let l = 0; do { b = d.getUint8(p++); l = (l << 7) | (b & 0x7f); } while (b & 0x80); if (ty === 0x51) usq = (d.getUint8(p) << 16) | (d.getUint8(p + 1) << 8) | d.getUint8(p + 2); p += l; }
        else if (st === 0xf0 || st === 0xf7) { let l = 0; do { b = d.getUint8(p++); l = (l << 7) | (b & 0x7f); } while (b & 0x80); p += l; }
        else {
          run = st; const ty = st & 0xf0, ch = st & 0x0f;
          if (ty === 0xc0) { prog[ch] = d.getUint8(p++); }
          else if (ty === 0xd0) p++;
          else {
            const a = d.getUint8(p++), v = d.getUint8(p++);
            if (ty === 0x90 && v > 0) on[ch + ':' + a] = { tick, v };
            else if (ty === 0x80 || (ty === 0x90 && v === 0)) { const k = ch + ':' + a, o = on[k]; if (o) { notes.push({ tk: o.tick, dk: tick - o.tick, ch, n: a, v: o.v, p: prog[ch] || 0 }); delete on[k]; } }
          }
        }
      }
      p = end;
    }
    const sec = usq / 1e6 / div;
    notes.forEach((n) => { n.t = n.tk * sec; n.d = Math.max(0.05, n.dk * sec); });
    notes.sort((a, b) => a.t - b.t);
    const last = notes.reduce((m, n) => Math.max(m, n.t + n.d), 0), bar = 4 * div * sec;
    return { notes, len: Math.ceil(last / bar) * bar };
  },
  async load(name) {
    if (this.cache[name]) return this.cache[name];
    const r = await fetch('music/' + name + '.mid'); const ab = await r.arrayBuffer();
    return (this.cache[name] = this.parse(ab));
  },
  async play(name, loop = true) {
    this.stop(); this.name = name;
    if (!A.ctx) return;
    let tune; try { tune = await this.load(name); } catch (e) { return; }
    if (this.name !== name) return;
    const st = { tune, idx: 0, off: 0, sp: -0.1, last: A.ctx.currentTime, loop };
    this.cur = st;
    this.timer = setInterval(() => this.tick(st), 60);
  },
  stop() { this.name = ''; if (this.timer) clearInterval(this.timer); this.timer = null; this.cur = null; },
  tick(st) { // song position advances at M.rate, so the tempo can change while it plays
    if (this.cur !== st || !A.ctx) return;
    const now = A.ctx.currentTime, r = this.rate; st.sp += Math.min(0.5, now - st.last) * r; st.last = now;
    const ahead = st.sp + 0.3 * r, ns = st.tune.notes;
    while (true) {
      if (st.idx >= ns.length) { if (!st.loop) { this.stop(); return; } st.off += st.tune.len; st.idx = 0; }
      const n = ns[st.idx], T = st.off + n.t; if (T > ahead) break;
      const at = now + (T - st.sp) / r; if (at >= now - 0.05) this.voice(n, Math.max(at, now), r);
      st.idx++;
    }
  },
  voice(n, t, r = 1) {
    const c = A.ctx, out = A.musicG, vel = n.v / 127, f = 440 * Math.pow(2, (n.n - 69) / 12), dd = n.d / r;
    if (n.ch === 9) {
      if (n.n === 36) { A.tone(t, 0.13, 'sine', 150, 42, 0.9 * vel, out); }
      else if (n.n === 38) { A.burst(t, 0.12, 'bandpass', 1900, 0.8, 0.55 * vel, out); A.tone(t, 0.08, 'triangle', 210, 140, 0.3 * vel, out); }
      else A.burst(t, n.n === 46 ? 0.16 : 0.04, 'highpass', 7500, 0.7, 0.22 * vel, out);
      return;
    }
    const o = c.createOscillator(), g = c.createGain(), lp = c.createBiquadFilter();
    let type = 'square', gain = 0.07, cut = 3500, att = 0.004;
    if (n.p === 38) { type = 'square'; gain = 0.16; cut = 700; }
    else if (n.p === 81) { type = 'sawtooth'; gain = 0.075; cut = 2800; }
    else if (n.p === 89) { type = 'triangle'; gain = 0.06; cut = 1800; att = 0.08; }
    else { type = 'square'; gain = 0.05; cut = 3200; }
    o.type = type; o.frequency.value = f; lp.type = 'lowpass'; lp.frequency.value = cut;
    g.gain.setValueAtTime(0.0001, t); g.gain.linearRampToValueAtTime(gain * vel * 1.6, t + att);
    g.gain.setValueAtTime(gain * vel * 1.6, t + Math.max(att, dd - 0.04)); g.gain.linearRampToValueAtTime(0.0001, t + dd + 0.03);
    o.connect(lp); lp.connect(g); g.connect(out); o.start(t); o.stop(t + dd + 0.05);
  },
};

/* ---------- textures (procedural, 64x64) ---------- */
const TS = 64;
function hash(x, y, s) { let h = (x * 374761393 + y * 668265263 + s * 2147483647) | 0; h = (h ^ (h >> 13)) * 1274126177; return ((h ^ (h >> 16)) >>> 0) / 4294967295; }
function makeTex(fn) {
  const t = new Uint8Array(TS * TS * 3);
  for (let y = 0; y < TS; y++) for (let x = 0; x < TS; x++) { const c = fn(x, y); const i = (y * TS + x) * 3; t[i] = clamp(c[0], 0, 255); t[i + 1] = clamp(c[1], 0, 255); t[i + 2] = clamp(c[2], 0, 255); }
  return t;
}
const TEX = [null,
  makeTex((x, y) => { const row = (y / 16) | 0, bx = (x + (row % 2) * 16) % 32; const m = y % 16 < 2 || bx < 2; const n = hash(x >> 1, y >> 1, 1) * 30; return m ? [95 + n * 0.3, 90 + n * 0.3, 84 + n * 0.3] : [150 + n, 64 + n * 0.6, 48 + n * 0.5]; }),
  makeTex((x, y) => { const n = hash(x, y, 2) * 22, s = hash(x >> 4, y >> 3, 9) * 30; const e = (x < 2 || x > 61) ? -25 : 0; return [205 + n - s + e, 188 + n - s + e, 150 + n - s * 1.2 + e]; }),
  makeTex((x, y) => { const plank = (y / 16) | 0, g = y % 16 < 1 || x % 32 < 1; const n = hash(x >> 2, y, 3) * 30; const br = Math.abs(x - y) < 3 || Math.abs(x + y - 63) < 3; if (g) return [60, 38, 20]; if (br) return [120, 80, 40]; return [158 + n - plank * 6, 108 + n * 0.7, 58 + n * 0.4]; }),
  makeTex((x, y) => { const rib = x % 8 < 2, rv = (x % 16 === 5 && (y % 16 === 3 || y % 16 === 12)); const n = hash(x, y >> 2, 4) * 14; if (rv) return [190, 190, 195]; return rib ? [62 + n, 76 + n, 86 + n] : [96 + n, 112 + n, 124 + n]; }),
  makeTex((x, y) => { const bx = (x + (((y / 16) | 0) % 2) * 16) % 32; const g = y % 16 < 1 || bx < 1; const n = hash(x >> 2, y >> 2, 5) * 24; return g ? [60, 58, 66] : [122 + n, 116 + n, 130 + n]; }),
  makeTex((x, y) => { const fx = x % 32, fy = y % 32, frame = fx < 3 || fx > 28 || fy < 3 || fy > 28; const n = hash(x >> 1, y, 6) * 18; return frame ? [62 + n, 38 + n * 0.6, 24] : [104 + n, 66 + n * 0.7, 44 + n * 0.4]; }),
  makeTex((x, y) => { const row = (y / 10) | 0, bx = (x + (row % 2) * 8) % 16, seam = y % 10 === 0 || bx === 0, n = hash(x >> 1, y >> 1, 7) * 28; return seam ? [70, 60, 40] : [150 + n, 128 + n * 0.8, 82 + n * 0.5]; }), // sandbags
];

/* ---------- sprites: ink-outlined pixel figures ---------- */
function makeFigure(o) {
  const w = 32, h = 52, dy = 4;
  function frame(kind) {
    const c = mk(w, h), g = c.getContext('2d'), parts = [];
    const P = (x, y, pw, ph, col) => parts.push([x, y + dy, pw, ph, col]);
    const step = kind === 'b' ? 3 : 0;
    // legs
    if (o.dress) { P(9, 29, 15, 18, o.dress); P(11, 45, 4, 3, o.skin); P(18, 45, 4, 3, o.skin); }
    else { P(11, 30, 5, 17 - step, o.pants); P(17, 30 + step, 5, 17 - step, o.pants); P(10, 46 - step, 7, 2, '#111'); P(17, 46, 7, 2, '#111'); }
    // torso
    P(10, 13, 13, 18, o.shirt);
    if (o.vest) P(11, 14, 11, 14, o.vest);
    if (o.trim) { P(10, 13, 13, 2, o.trim); P(15, 13, 3, 18, o.trim); }
    // arms and weapon
    if (o.zombie) { P(4, 15, 5, 11, o.skin); P(24, 15, 5, 11, o.skin); P(4, 12, 5, 4, o.skin); P(24, 12, 5, 4, o.skin); if (kind === 'aim') { P(4, 9, 5, 4, o.skin); P(24, 9, 5, 4, o.skin); } }
    else if (o.weapon === 'rifle' || o.weapon === 'long') {
      P(5, 16, 5, 9, o.shirt); P(23, 16, 5, 9, o.shirt);
      if (kind === 'aim') { P(14, 8, 4, 13, '#1b1b1f'); P(13, 20, 6, 4, o.skin); P(15, 3, 2, 5, '#ffd34a'); P(13, 5, 6, 2, '#fff2a0'); }
      else { P(8, 20, o.weapon === 'long' ? 18 : 15, 3, '#1b1b1f'); P(22, 19, 5, 5, o.skin); P(8, 19, 4, 5, o.skin); }
    } else if (o.weapon === 'bazooka') {
      P(5, 16, 5, 9, o.shirt); P(23, 16, 5, 9, o.shirt);
      if (kind === 'aim') { P(10, 6, 12, 14, '#4a5a3a'); P(12, 8, 8, 10, '#0b0b0d'); P(9, 5, 14, 2, '#2a2f22'); P(14, 0, 4, 5, '#ff9a2a'); P(13, -1, 6, 2, '#fff2a0'); P(5, 18, 5, 5, o.skin); P(23, 18, 5, 5, o.skin); }
      else { P(3, 8, 26, 5, '#4a5a3a'); P(26, 6, 4, 9, '#2a2f22'); P(3, 7, 3, 7, '#1a1a1a'); P(11, 13, 5, 4, o.skin); P(21, 13, 5, 4, o.skin); }
    } else if (o.weapon === 'blade') {
      P(5, 16, 5, 10, o.shirt); P(23, 16, 5, 10, o.shirt);
      if (kind === 'aim') { P(25, 1, 3, 24, '#d9dde6'); P(23, 24, 7, 3, '#8a6a2a'); }
      else { P(25, 8, 3, 18, '#d9dde6'); P(23, 24, 7, 3, '#8a6a2a'); }
      P(24, 25, 5, 4, o.skin);
    } else { P(4, 15, o.fist ? 6 : 5, 12, o.shirt); P(23, 15, o.fist ? 6 : 5, 12, o.shirt); P(3, 26, o.fist ? 7 : 5, 5, o.skin); P(23, 26, o.fist ? 7 : 5, 5, o.skin); }
    // head
    P(12, 4, 9, 9, o.skin);
    P(14, 8, 2, 2, o.eyes || '#111'); P(18, 8, 2, 2, o.eyes || '#111');
    if (o.mouth) P(14, 11, 5, 1, o.mouth);
    const ht = o.hatType;
    if (ht === 'cap') { P(11, 2, 11, 4, o.hat); P(11, 6, 13, 1, o.hat); }
    else if (ht === 'cowboy') { P(6, 4, 21, 2, o.hat); P(11, -1, 11, 6, o.hat); P(11, 3, 11, 1, '#c9a24a'); }
    else if (ht === 'helmet') { P(11, 1, 11, 6, o.hat); P(11, 6, 11, 1, '#222'); }
    else if (ht === 'bun') { P(14, -2, 6, 4, o.hair); P(11, 3, 11, 3, o.hair); P(10, 6, 2, 6, o.hair); P(21, 6, 2, 6, o.hair); }
    else if (ht === 'hood') { P(10, 2, 13, 5, o.hat); P(10, 6, 3, 8, o.hat); P(20, 6, 3, 8, o.hat); }
    else if (ht === 'wrap') { P(11, 1, 11, 5, o.hat); P(21, 3, 4, 3, o.hat); P(13, -1, 7, 3, o.hat); }
    else if (o.hair) P(12, 3, 9, 3, o.hair);
    // outline pass, then fill
    g.fillStyle = '#0a0a0d';
    parts.forEach((q) => g.fillRect(q[0] - 1, q[1] - 1, q[2] + 2, q[3] + 2));
    parts.forEach((q) => { g.fillStyle = q[4]; g.fillRect(q[0], q[1], q[2], q[3]); });
    return c;
  }
  const stand = frame('a'), walk = frame('b'), aim = frame('aim');
  const sil = (src) => { const c = mk(w, h), g = c.getContext('2d'); g.drawImage(src, 0, 0); g.globalCompositeOperation = 'source-in'; g.fillStyle = '#fff'; g.fillRect(0, 0, w, h); return c; };
  const dead = mk(h, w), dg = dead.getContext('2d'); dg.translate(h / 2, w / 2); dg.rotate(Math.PI / 2); dg.drawImage(stand, -w / 2, -h / 2); dg.setTransform(1, 0, 0, 1, 0, 0);
  const dim = mk(h, w), mg = dim.getContext('2d'); mg.drawImage(dead, 0, 0); mg.globalCompositeOperation = 'source-atop'; mg.fillStyle = 'rgba(0,0,0,0.45)'; mg.fillRect(0, 0, h, w);
  return { stand, walk, aim, hurt: sil(stand), hurtAim: sil(aim), dead: dim, w, h, scale: o.scale || 1 };
}
const FIG = {
  guard: makeFigure({ skin: '#8d5524', shirt: '#3b5d3a', pants: '#2a2a30', hat: '#a33030', hatType: 'cap', weapon: 'rifle' }),
  gang: makeFigure({ skin: '#6b4226', shirt: '#17171b', pants: '#33333a', hat: '#101014', hatType: 'cap', weapon: 'rifle', trim: '#c9a24a', eyes: '#222' }),
  brute: makeFigure({ skin: '#7a8f70', shirt: '#4a3f3a', pants: '#3a3a3a', zombie: true, eyes: '#c33', mouth: '#300', scale: 1.28 }),
  yaku: makeFigure({ skin: '#9bb08a', shirt: '#5a4a44', pants: '#2c3a3a', zombie: true, eyes: '#e8e070', mouth: '#300', scale: 0.95 }),
  prisoner: makeFigure({ skin: '#8a8f86', shirt: '#c4701f', pants: '#c4701f', zombie: true, eyes: '#ddd', mouth: '#300' }),
  sniper: makeFigure({ skin: '#8d5524', shirt: '#6b6a3c', pants: '#3a3a2a', hat: '#2f2f22', hatType: 'hood', weapon: 'long', scale: 1.05 }),
  flanm: makeFigure({ skin: '#7a4a2e', shirt: '#b3202a', dress: '#8a1822', hat: '#e24a3a', hatType: 'wrap', weapon: 'rifle', trim: '#ffd34a', eyes: '#fff', scale: 1.12 }),
  woch: makeFigure({ skin: '#5a3a22', shirt: '#14161a', vest: '#2f3b4a', pants: '#2a2f38', hat: '#6a7078', hatType: 'helmet', fist: true, scale: 1.38, eyes: '#e33' }),
  manchet: makeFigure({ skin: '#a89f8d', shirt: '#5b1f2a', pants: '#3a1a22', trim: '#d4af37', hat: '#3a2410', hatType: 'cowboy', weapon: 'blade', eyes: '#5ff', scale: 1.3 }),
  nwit: makeFigure({ skin: '#5a3a22', shirt: '#1b1030', pants: '#150c24', hair: '#0a0a0a', hatType: 'bun', weapon: 'blade', eyes: '#ffd34a', trim: '#6a3fc0', scale: 1.12 }),
  bazooka: makeFigure({ skin: '#7a4a2e', shirt: '#6b3b1f', pants: '#2a2a30', hat: '#1c1c24', hatType: 'hood', weapon: 'bazooka', trim: '#c9a24a', eyes: '#e33', scale: 1.08 }),
  lost: makeFigure({ skin: '#8a5a3a', shirt: '#c9c9d4', pants: '#5a5a6a', eyes: '#ddd', scale: 0.62, hair: '#111' }),
};
function makeItem(kind) {
  const c = mk(16, 16), g = c.getContext('2d');
  g.fillStyle = '#0a0a0d';
  if (kind === 'ammo') { g.fillRect(1, 5, 14, 10); g.fillStyle = '#d6a32a'; g.fillRect(2, 6, 12, 8); g.fillStyle = '#5a3d0a'; g.fillRect(2, 9, 12, 2); g.fillStyle = '#fff2a0'; g.fillRect(4, 7, 2, 1); }
  else if (kind === 'bottle') { g.fillRect(5, 1, 6, 14); g.fillStyle = '#35c26b'; g.fillRect(6, 5, 4, 9); g.fillStyle = '#8f6a2a'; g.fillRect(6, 2, 4, 3); g.fillStyle = '#c8ffd9'; g.fillRect(7, 6, 1, 5); }
  else { g.fillRect(3, 3, 10, 10); g.fillStyle = '#ffd34a'; g.fillRect(4, 4, 8, 8); g.fillStyle = '#a07a10'; g.fillRect(7, 5, 2, 6); }
  return c;
}
const ITEM = { ammo: makeItem('ammo'), bottle: makeItem('bottle'), coin: makeItem('coin') };

/* ---------- levels ---------- */
function grid(w, h) { const g = []; for (let y = 0; y < h; y++) g.push(new Uint8Array(w)); g.w = w; g.h = h; return g; }
function rect(g, x, y, w, h, v) { for (let j = y; j < y + h; j++) for (let i = x; i < x + w; i++) if (g[j] && i >= 0 && i < g.w) g[j][i] = v; }
function border(g, v) { rect(g, 0, 0, g.w, 1, v); rect(g, 0, g.h - 1, g.w, 1, v); rect(g, 0, 0, 1, g.h, v); rect(g, g.w - 1, 0, 1, g.h, v); }
function hwall(g, y, x0, x1, v, gaps) { for (let x = x0; x <= x1; x++) if (!gaps.includes(x)) g[y][x] = v; }
function vwall(g, x, y0, y1, v, gaps) { for (let y = y0; y <= y1; y++) if (!gaps.includes(y)) g[y][x] = v; }
function dots(g, v, pts) { pts.forEach(([x, y]) => { g[y][x] = v; }); }

const LEVELS = [
  { name: 'THE YARD', sub: 'Level 1. The compound at night. Guards on patrol.', boss: 'sniper', bossName: 'THE SNIPER', w: 30, h: 24, start: [3.5, 20.5],
    theme: { sky: [14, 10, 36, 40, 30, 80], floor: [58, 50, 44], fog: [10, 8, 20], tint: [1, 1, 1.05], maxD: 22, music: 'stage1', amb: 'wind' },
    spawn: { guard: 4, gang: 5 }, pick: { ammo: 3, bottle: 2 },
    build(g) { border(g, 1); rect(g, 2, 2, 6, 6, 2); rect(g, 12, 5, 3, 2, 4); rect(g, 20, 4, 2, 2, 4); rect(g, 21, 16, 4, 3, 3); rect(g, 25, 7, 4, 10, 2);
      dots(g, 3, [[8, 12], [9, 12], [14, 14], [15, 14], [14, 15], [6, 16], [18, 10], [24, 9], [11, 19], [17, 19], [4, 12], [13, 9]]); } },
  { name: 'INSIDE THE HOUSE', sub: 'Level 2. A dark maze lit by candles. Granmet Nwit hides in the walls.', boss: 'nwit', bossName: 'GRANMET NWIT', w: 28, h: 24, start: [2.5, 21.5],
    theme: { sky: null, ceil: [24, 14, 10], floor: [48, 30, 22], fog: [14, 6, 4], tint: [1.25, 0.95, 0.75], maxD: 14, music: 'stage2', amb: 'room' },
    spawn: { gang: 6, prisoner: 4, yaku: 2 }, pick: { ammo: 3, bottle: 3 },
    build(g) { border(g, 6); hwall(g, 8, 1, 26, 6, [5, 13, 21]); hwall(g, 16, 1, 26, 6, [9, 17, 24]); vwall(g, 9, 1, 7, 6, [4]); vwall(g, 18, 1, 7, 6, [3]);
      vwall(g, 9, 9, 15, 6, [12]); vwall(g, 18, 9, 15, 6, [11]); vwall(g, 13, 17, 22, 6, [20]); dots(g, 3, [[4, 4], [14, 4], [23, 4], [5, 12], [14, 12], [23, 12], [5, 20], [22, 20]]); } },
  { name: 'THE PARTY WING', sub: 'Level 3. Ground floor. Madanm Flanm and her zombies. The Lost drift past: never shoot them.', boss: 'flanm', bossName: 'MADANM FLANM', w: 32, h: 24, start: [3.5, 21.5],
    theme: { sky: null, ceil: [26, 10, 38], floor: [40, 26, 52], fog: [18, 6, 28], tint: [1.1, 0.8, 1.3], maxD: 18, music: 'stage3', amb: 'room' },
    spawn: { gang: 4, prisoner: 8, yaku: 4, brute: 2, lost: 4 }, pick: { ammo: 4, bottle: 3 },
    build(g) { border(g, 5); vwall(g, 7, 1, 22, 5, [6, 7, 15, 16, 20]); rect(g, 13, 1, 6, 2, 4); rect(g, 26, 8, 2, 8, 6);
      dots(g, 5, [[11, 6], [11, 12], [11, 18], [17, 6], [17, 12], [17, 18], [23, 6], [23, 18], [20, 12], [14, 9], [14, 15]]); } },
  { name: 'THE GLASS GALLERY', sub: 'Level 4. First floor. Tet Woch holds the east penthouse. The Lost walk the rooms.', boss: 'woch', bossName: 'TET WOCH', w: 34, h: 18, start: [2.5, 15.5],
    theme: { sky: null, ceil: [14, 24, 30], floor: [34, 66, 76], fog: [8, 18, 24], tint: [0.85, 1.15, 1.3], maxD: 20, music: 'stage4', amb: 'room' },
    spawn: { gang: 8, prisoner: 6, yaku: 4, brute: 2, lost: 4 }, pick: { ammo: 4, bottle: 3 },
    build(g) { border(g, 4); vwall(g, 10, 1, 16, 4, [8, 9, 14]); vwall(g, 23, 1, 16, 4, [8, 9, 3]); dots(g, 5, [[14, 4], [14, 13], [19, 4], [19, 13], [16, 8], [17, 9], [28, 4], [28, 13], [5, 5], [4, 12]]); rect(g, 26, 7, 3, 3, 6); } },
  { name: 'THE ALTAR HALL', sub: 'Level 5. Second floor. The Sword Man guards the altar. He takes a thousand bullets in the story. Here, sixty.', boss: 'manchet', bossName: 'ZONBI MANCHET', w: 32, h: 24, start: [2.5, 21.5],
    theme: { sky: null, ceil: [20, 8, 20], floor: [44, 24, 40], fog: [16, 4, 16], tint: [1.3, 0.8, 0.95], maxD: 18, music: 'stage5', amb: 'room' },
    spawn: { prisoner: 8, yaku: 6, brute: 4 }, pick: { ammo: 5, bottle: 3 },
    build(g) { border(g, 5); vwall(g, 14, 1, 22, 5, [4, 12, 19]); hwall(g, 8, 1, 13, 5, [3, 10]); hwall(g, 15, 1, 13, 5, [6, 12]); vwall(g, 7, 1, 7, 5, [3]); vwall(g, 7, 16, 22, 5, [19]);
      rect(g, 21, 10, 3, 3, 3); dots(g, 5, [[18, 4], [18, 19], [27, 4], [27, 19], [18, 8], [18, 15], [27, 8], [27, 15]]); } },
  { name: "NWIT'S QUARTERS", sub: 'Level 6. Third floor. At six o\'clock the dead rise and join him.', boss: 'nwit', bossName: 'GRANMET NWIT', w: 30, h: 22, start: [2.5, 19.5],
    theme: { sky: null, ceil: [22, 18, 8], floor: [56, 44, 24], fog: [20, 14, 4], tint: [1.3, 1.05, 0.7], maxD: 15, music: 'stage6', amb: 'room' },
    spawn: { gang: 6, prisoner: 6, yaku: 4, brute: 3, lost: 3, flanm_z: 1, woch_z: 1 }, pick: { ammo: 4, bottle: 4 },
    build(g) { border(g, 6); hwall(g, 7, 1, 28, 6, [4, 12, 20, 26]); hwall(g, 14, 1, 28, 6, [7, 15, 23]); vwall(g, 10, 1, 6, 6, [3]); vwall(g, 20, 1, 6, 6, [3]); vwall(g, 14, 8, 13, 6, [10]);
      rect(g, 4, 9, 4, 3, 4); rect(g, 22, 16, 4, 3, 5); dots(g, 5, [[8, 17], [12, 18], [16, 17], [26, 10], [24, 4], [6, 4]]); } },
  { name: 'THE ROOFTOP', sub: 'Level 7. Granmet Nwit, weakened, beside his candle ring. End it.', boss: 'nwitfinal', bossName: 'GRANMET NWIT', w: 24, h: 20, start: [12.5, 17.5],
    theme: { sky: [6, 8, 30, 70, 40, 90], floor: [50, 52, 60], fog: [8, 8, 22], tint: [0.95, 0.95, 1.15], maxD: 22, music: 'stage7', amb: 'wind' },
    spawn: { gang: 6, prisoner: 5, yaku: 4, brute: 2 }, pick: { ammo: 4, bottle: 3 },
    build(g) { border(g, 4); rect(g, 1, 1, 4, 4, 2); dots(g, 5, [[12, 5], [14, 6], [15, 8], [14, 10], [12, 11], [10, 10], [9, 8], [10, 6]]); dots(g, 3, [[5, 14], [19, 14], [6, 9], [18, 9]]); } },
];

// Low obstacles (sandbags, crates) per level: [x, y, width, height]. Only free cells are used, and a rectangle is skipped if it would cut off any part of the level.
const COVERS = [
  [[10, 10, 2, 1], [16, 12, 1, 2], [6, 18, 2, 1], [19, 20, 2, 1], [13, 16, 2, 1], [22, 12, 2, 1], [9, 5, 1, 2], [17, 3, 2, 1], [4, 14, 1, 2], [12, 21, 2, 1], [16, 8, 2, 1], [26, 19, 2, 1]],
  [[3, 5, 2, 1], [12, 3, 2, 1], [22, 5, 2, 1], [4, 10, 2, 1], [12, 13, 2, 1], [22, 11, 2, 1], [4, 19, 2, 1], [16, 19, 1, 2], [22, 18, 2, 1], [7, 21, 2, 1], [14, 10, 2, 1]],
  [[9, 4, 2, 1], [9, 14, 1, 2], [12, 9, 2, 1], [15, 20, 2, 1], [20, 3, 1, 2], [21, 15, 2, 1], [24, 20, 2, 1], [29, 5, 1, 2], [29, 18, 1, 2], [3, 10, 2, 1], [3, 16, 1, 2], [16, 14, 2, 1]],
  [[3, 3, 2, 1], [5, 10, 1, 2], [13, 6, 2, 1], [12, 11, 2, 1], [17, 7, 1, 2], [21, 5, 1, 2], [20, 12, 2, 1], [26, 4, 2, 1], [30, 6, 1, 2], [29, 13, 2, 1], [31, 10, 1, 2], [7, 6, 2, 1]],
  [[17, 6, 2, 1], [17, 12, 1, 2], [20, 3, 2, 1], [24, 6, 2, 1], [24, 17, 2, 1], [28, 11, 1, 2], [20, 20, 2, 1], [28, 3, 1, 2], [21, 16, 2, 1], [3, 12, 2, 1], [10, 11, 2, 1], [4, 19, 2, 1], [10, 19, 2, 1], [4, 4, 2, 1], [10, 4, 1, 2]],
  [[3, 3, 2, 1], [14, 3, 2, 1], [24, 3, 2, 1], [4, 10, 2, 1], [17, 10, 2, 1], [25, 10, 1, 2], [3, 17, 2, 1], [20, 19, 2, 1], [22, 17, 2, 1], [10, 17, 2, 1], [27, 17, 1, 2]],
  [[4, 12, 2, 1], [17, 12, 2, 1], [8, 15, 2, 1], [15, 15, 2, 1], [6, 7, 1, 2], [18, 6, 1, 2], [11, 13, 2, 1], [3, 17, 1, 2], [20, 16, 1, 2], [9, 3, 2, 1], [14, 3, 2, 1]],
];
function placeCover() {
  const list = COVERS[G.lv]; if (!list) return; const g = G.map, st = LEVELS[G.lv].start; let before = reachable().length;
  for (const [x, y, w, h] of list) {
    const cells = []; for (let j = y; j < y + h; j++) for (let i = x; i < x + w; i++) if (g[j] && i > 0 && i < g.w - 1 && j > 0 && j < g.h - 1 && g[j][i] === 0 && Math.hypot(i + 0.5 - st[0], j + 0.5 - st[1]) > 2.8) cells.push([i, j]);
    if (!cells.length) continue;
    cells.forEach(([i, j]) => { g[j][i] = COVER; });
    const after = reachable().length;
    if (after < before - cells.length) cells.forEach(([i, j]) => { g[j][i] = 0; }); else before = after;
  }
}

/* ---------- game state ---------- */
const G = { state: 'menu', lv: 0, map: null, ents: [], items: [], player: null, kills: 0, weight: 0, earnedT: 0, t: 0, flow: null, flowT: 0, flowCell: -1,
  boss: null, bossSpawned: false, banner: null, msg: null, msgT: 0, flash: 0, shake: 0, hitmark: 0, endKind: '', paused: false, deadT: 0, freed: 0, nades: [], rockets: [], fxs: [], invKills: 0, rktT: 30 };
const keys = {};
let mouseDown = false, locked = false;
const ui = { buttons: [], sel: 0 };

function newPlayer() {
  const L = LEVELS[G.lv], gun = {}; for (const k in WP) if (WP[k].mag) gun[k] = { mag: WP[k].mag, res: WP[k].res };
  return { x: L.start[0], y: L.start[1], a: -Math.PI / 2, pitch: 0, hp: 100, bottles: 1, wpn: 'ak', gun, nades: 3, akAuto: true, reload: 0, reloadK: 'ak', cd: 0, kick: 0, vx: 0, vy: 0, bob: 0, walk: 0, crouch: 0, inv: 0, stab: 0, trig: false, buf: 0, ohT: 0, flashT: 0 };
}
const eyeOf = (p) => 0.5 - 0.2 * p.crouch;
function showMsg(t, s = 2.2) { G.msg = t; G.msgT = s; }
function isWall(x, y) { const xi = x | 0, yi = y | 0; if (xi < 0 || yi < 0 || xi >= G.map.w || yi >= G.map.h) return true; return G.map[yi][xi] > 0; }
function isOpq(x, y) { const xi = x | 0, yi = y | 0; if (xi < 0 || yi < 0 || xi >= G.map.w || yi >= G.map.h) return true; const v = G.map[yi][xi]; return v > 0 && v !== COVER; }
function los(x0, y0, x1, y1) { const dx = x1 - x0, dy = y1 - y0, d = Math.hypot(dx, dy), n = Math.ceil(d / 0.2); for (let i = 1; i < n; i++) { if (isOpq(x0 + dx * i / n, y0 + dy * i / n)) return false; } return true; }
// true when the player is crouched and a low obstacle stands between him and the point (sx, sy), right in front of him
function covered(sx, sy) {
  const p = G.player; if (p.crouch < 0.8) return false;
  const dx = sx - p.x, dy = sy - p.y, d = Math.hypot(dx, dy) || 1;
  for (let s = 0.15; s <= Math.min(1.7, d); s += 0.12) { const x = (p.x + dx / d * s) | 0, y = (p.y + dy / d * s) | 0; if (G.map[y] && G.map[y][x] === COVER) return true; }
  return false;
}
function coverNear() { const p = G.player; for (let k = -3; k <= 3; k++) { const a = p.a + k * 0.3; for (let s = 0.3; s <= 1.6; s += 0.3) { const x = (p.x + Math.cos(a) * s) | 0, y = (p.y + Math.sin(a) * s) | 0; if (G.map[y] && G.map[y][x] === COVER) return true; } } return false; }
function tryMove(e, dx, dy, r) {
  if (!isWall(e.x + dx + Math.sign(dx) * r, e.y - r) && !isWall(e.x + dx + Math.sign(dx) * r, e.y + r)) e.x += dx;
  if (!isWall(e.x - r, e.y + dy + Math.sign(dy) * r) && !isWall(e.x + r, e.y + dy + Math.sign(dy) * r)) e.y += dy;
}
function buildFlow() {
  const p = G.player, w = G.map.w, h = G.map.h; if (!G.flow || G.flow.length !== w * h) G.flow = new Int16Array(w * h);
  const f = G.flow; f.fill(-1); const sx = p.x | 0, sy = p.y | 0; if (isWall(p.x, p.y)) return;
  const q = [sy * w + sx]; f[q[0]] = 0;
  for (let qi = 0; qi < q.length; qi++) {
    const c = q[qi], cx = c % w, cy = (c / w) | 0;
    for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) {
      if (!dx && !dy) continue; const nx = cx + dx, ny = cy + dy;
      if (nx < 0 || ny < 0 || nx >= w || ny >= h || G.map[ny][nx] > 0 || f[ny * w + nx] >= 0) continue;
      if (dx && dy && (G.map[cy][nx] > 0 || G.map[ny][cx] > 0)) continue;
      f[ny * w + nx] = f[c] + 1; q.push(ny * w + nx);
    }
  }
}
function flowStep(e) {
  const w = G.map.w, h = G.map.h, cx = e.x | 0, cy = e.y | 0, f = G.flow; let best = f[cy * w + cx] < 0 ? 9999 : f[cy * w + cx], bx = -1, by = -1;
  for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) {
    if (!dx && !dy) continue; const nx = cx + dx, ny = cy + dy; if (nx < 0 || ny < 0 || nx >= w || ny >= h) continue;
    const v = f[ny * w + nx]; if (v < 0 || G.map[ny][nx] > 0) continue;
    if (dx && dy && (G.map[cy][nx] > 0 || G.map[ny][cx] > 0)) continue;
    if (v < best) { best = v; bx = nx; by = ny; }
  }
  return bx < 0 ? null : [bx + 0.5, by + 0.5];
}
function reachable() {
  const L = G.map, w = L.w, h = L.h, seen = new Uint8Array(w * h), out = [], s = LEVELS[G.lv].start, q = [(s[1] | 0) * w + (s[0] | 0)]; seen[q[0]] = 1;
  for (let i = 0; i < q.length; i++) { const c = q[i], x = c % w, y = (c / w) | 0; out.push([x + 0.5, y + 0.5]);
    [[1, 0], [-1, 0], [0, 1], [0, -1]].forEach(([dx, dy]) => { const nx = x + dx, ny = y + dy; if (nx >= 0 && ny >= 0 && nx < w && ny < h && !L[ny][nx] && !seen[ny * w + nx]) { seen[ny * w + nx] = 1; q.push(ny * w + nx); } }); }
  return out;
}

/* ---------- enemies ---------- */
const T = {
  guard: { name: 'Guard', hp: 3, sp: 1.3, sight: 12, fire: [1.1, 2.0], dmg: 9, acc: 0.55, pref: 5, rng: 11, fig: 'guard', rise: true },
  gang: { name: 'Gang shooter', hp: 4, sp: 1.7, sight: 13, fire: [0.8, 1.5], dmg: 9, acc: 0.6, pref: 5.5, rng: 12, fig: 'gang', rise: true },
  brute: { name: 'Brute', hp: 8, sp: 1.2, sight: 10, melee: 1, dmg: 16, fig: 'brute', zombie: true },
  yaku: { name: 'Yaku', hp: 3, sp: 2.3, sight: 10, melee: 1, dmg: 7, fig: 'yaku', zombie: true },
  prisoner: { name: 'Prisoner', hp: 2, sp: 1.5, sight: 10, melee: 1, dmg: 5, fig: 'prisoner', zombie: true },
  bazooka: { name: 'Bazooka bandit', hp: 4, sp: 1.4, sight: 22, fire: [5.5, 7.5], acc: 1, pref: 9, rng: 20, fig: 'bazooka', rocket: true, aimT: 1.3 },
  sniper: { name: 'The Sniper', hp: 8, sp: 1.2, sight: 30, fire: [2.0, 2.6], dmg: 26, acc: 0.9, pref: 11, rng: 30, fig: 'sniper', boss: true, aimT: 0.9, headCh: 0.3 },
  flanm: { name: 'Madanm Flanm', hp: 10, sp: 1.0, sight: 20, fire: [2.0, 2.6], dmg: 9, acc: 0.62, pref: 7, rng: 16, fig: 'flanm', boss: true, aimT: 0.4, burst: 3, headKill: true },
  woch: { name: 'Tet Woch', hp: 20, sp: 2.0, sight: 20, melee: 1.1, dmg: 20, fig: 'woch', boss: true, headKill: true },
  manchet: { name: 'Zonbi Manchet', hp: 60, sp: 1.6, sight: 20, melee: 1.3, dmg: 18, fig: 'manchet', boss: true, headMul: 2 },
  nwit: { name: 'Granmet Nwit', hp: 14, sp: 1.8, sight: 20, melee: 1.1, dmg: 15, fig: 'nwit', boss: true, vanish: true },
  nwitfinal: { name: 'Granmet Nwit', hp: 18, sp: 1.9, sight: 20, melee: 1.1, dmg: 20, fig: 'nwit', boss: true, vanish: true, kneel: true },
  flanm_z: { name: 'Risen Flanm', hp: 12, sp: 1.2, sight: 12, melee: 1, dmg: 14, fig: 'flanm', zombie: true, eyesTint: true },
  woch_z: { name: 'Risen Woch', hp: 16, sp: 1.4, sight: 12, melee: 1, dmg: 18, fig: 'woch', zombie: true },
  lost: { name: 'Lost', hp: 99, sp: 0.8, sight: 0, fig: 'lost', lost: true },
};
function spawnEnt(type, x, y) {
  const d = T[type], e = { type, d, x, y, hp: d.hp, maxhp: d.hp, alert: false, dead: false, cd: rand(0.5, 1.5), aim: 0, burst: 0, burstT: 0, hurt: 0, flash: 0, walkT: Math.random() * 6, tx: x, ty: y, wait: rand(0, 2), sd: Math.random() < 0.5 ? 1 : -1, sdT: rand(1, 2.5), swing: 0, vanish: 0, expose: 0, kneel: false, risen: false, riseAt: 0, rising: false, deadT: 0, called: false };
  G.ents.push(e); return e;
}
function spawnWave() {
  const L = LEVELS[G.lv], cells = reachable().filter(([x, y]) => Math.hypot(x - L.start[0], y - L.start[1]) > 7);
  const use = []; const take = () => { if (!cells.length) return [L.start[0] + 3, L.start[1]]; const i = (Math.random() * cells.length) | 0, c = cells.splice(i, 1)[0]; return c; };
  for (const k in L.spawn) for (let i = 0; i < L.spawn[k]; i++) { const [x, y] = take(); spawnEnt(k, x, y); }
  for (const k in L.pick) for (let i = 0; i < L.pick[k]; i++) { const c = take(); G.items.push({ kind: k, x: c[0], y: c[1] }); }
  void use;
}
function spawnBoss() {
  const L = LEVELS[G.lv], p = G.player;
  const cells = reachable().filter(([x, y]) => Math.hypot(x - p.x, y - p.y) > 9 && los(x, y, x, y));
  const c = cells.length ? pick(cells) : [L.start[0], L.start[1] - 8];
  const e = spawnEnt(L.boss, c[0], c[1]); e.alert = true; e.isBoss = true; G.boss = e; G.bossSpawned = true;
  G.banner = { text: 'BOSS: ' + L.bossName, t: 3 }; A.sfx('roar'); M.play(G.lv === 6 ? 'boss_final' : 'boss');
}
function aliveCount() { return G.ents.filter((e) => !e.dead && !e.d.lost && !e.isBoss).length + G.ents.filter((e) => e.dead && e.rising).length; }

function playerHurt(n) {
  const p = G.player; if (G.state !== 'play') return;
  if (p.inv > 0) { A.sfx('shield', 0.6); return; }
  p.hp -= n; G.flash = 0.35; G.shake = 0.25; A.sfx('hurt', 0.6);
  if (p.ohT <= 0) { A.sfx('oh', 0.9); p.ohT = 0.35; }
  if (p.hp <= 0) { p.hp = 0; die(); }
}
function die() {
  M.rate = 1; G.state = 'dead'; G.deadT = 0; Acct.useChance().then(() => Acct.flush()); M.play('lose', false);
  if (document.exitPointerLock) document.exitPointerLock();
}
function startInv() { const p = G.player; p.inv = INV_TIME; M.rate = 1.55; A.sfx('power'); G.banner = { text: 'INVINCIBLE ' + INV_TIME + 's', t: 2.2, col: '#ffd34a' }; }
function launchRocket(e) {
  const p = G.player, dx = p.x - e.x, dy = p.y - e.y, d = Math.hypot(dx, dy) || 1;
  G.rockets.push({ x: e.x + dx / d * 0.5, y: e.y + dy / d * 0.5, vx: dx / d * 6, vy: dy / d * 6, t: 0 });
  A.sfx('rocket', 0.9, clamp(Math.sin(Math.atan2(e.y - p.y, e.x - p.x) - p.a + Math.PI / 2), -1, 1) * 0.6);
}
function explode(x, y) { G.fxs.push({ x, y, t: 0, life: 0.55 }); A.sfx('boom', 1); const p = G.player, d = Math.hypot(p.x - x, p.y - y); if (d < 7) G.shake = Math.max(G.shake, 0.3); return d; }
function rocketBlast(r) {
  const d = explode(r.x, r.y), p = G.player;
  if (d < 2.2) {
    if (p.inv > 0) { A.sfx('shield'); return; }
    if (covered(r.x, r.y)) { showMsg('You hid from the rocket.', 1.8); return; }
    playerHurt(ROCKET_HIT); showMsg('Rocket! You lost half of your health.', 2.2);
  }
}
function updateRockets(dt) {
  const p = G.player;
  for (let i = G.rockets.length - 1; i >= 0; i--) {
    const r = G.rockets[i]; r.t += dt; let boom = false; const steps = Math.max(1, Math.ceil(Math.hypot(r.vx, r.vy) * dt / 0.1));
    for (let s = 0; s < steps; s++) {
      const nx = r.x + r.vx * dt / steps, ny = r.y + r.vy * dt / steps;
      if (isWall(nx, ny)) { boom = true; break; }
      r.x = nx; r.y = ny;
      if (Math.hypot(p.x - r.x, p.y - r.y) < 0.45) { boom = true; break; }
    }
    if (boom || r.t > 8) { G.rockets.splice(i, 1); rocketBlast(r); if (G.state !== 'play') return; }
  }
}
function nadeBlast(n) {
  const d = explode(n.x, n.y), R = 3.2, p = G.player;
  for (const e of G.ents.slice()) { if (e.dead || e.d.lost) continue; if (Math.hypot(e.x - n.x, e.y - n.y) < R && los(n.x, n.y, e.x, e.y)) damageEnt(e, 6, false); }
  if (d < 2.2 && G.state === 'play' && los(n.x, n.y, p.x, p.y) && !covered(n.x, n.y)) playerHurt(30);
}
function updateNades(dt) {
  for (let i = G.nades.length - 1; i >= 0; i--) {
    const n = G.nades[i]; n.t += dt; n.fuse -= dt; const f = Math.max(0, 1 - 1.6 * dt); n.vx *= f; n.vy *= f;
    const nx = n.x + n.vx * dt, ny = n.y + n.vy * dt;
    if (isWall(nx, n.y)) { n.vx = -n.vx * 0.5; A.sfx('clink', 0.4); } else n.x = nx;
    if (isWall(n.x, ny)) { n.vy = -n.vy * 0.5; } else n.y = ny;
    if (n.fuse <= 0) { G.nades.splice(i, 1); nadeBlast(n); if (G.state !== 'play') return; }
  }
}
function spawnBazooka() {
  if (G.ents.some((e) => !e.dead && e.type === 'bazooka')) return;
  const p = G.player, cells = reachable().filter(([x, y]) => { const d = Math.hypot(x - p.x, y - p.y); return d > 7 && d < 18; });
  if (!cells.length) return;
  const sight = cells.filter(([x, y]) => los(x, y, p.x, p.y)), c = pick(sight.length ? sight : cells);
  const e = spawnEnt('bazooka', c[0], c[1]); e.alert = true; e.cd = 1.5;
  G.banner = { text: 'BAZOOKA! HOLD Z AND HIDE', t: 2.5, col: '#ffb040' }; A.sfx('warn', 0.8);
}

function killEnt(e, head) {
  e.dead = true; e.deadT = 0; e.hurt = 0;
  A.sfx(e.d.zombie ? 'groan' : 'die', 0.9, 0);
  if (e.d.lost) return;
  G.kills++; G.weight++; G.invKills++; if (G.invKills % INV_EVERY === 0) startInv(); Acct.earn(KILL_T); if (Acct.signedIn) G.earnedT += KILL_T; A.sfx('coin', 0.7);
  if (Math.random() < 0.16) G.items.push({ kind: Math.random() < 0.55 ? 'ammo' : 'bottle', x: e.x, y: e.y });
  if (e.d.rise && !e.risen && Math.random() < 0.4) { e.rising = true; e.riseAt = rand(4, 7); }
  if (e.isBoss) { A.sfx('boom', 0.8); if (G.state === 'play') bossDown(e); }
}
function bossDown(e) {
  G.ents.forEach((z) => { if (!z.dead && !z.d.lost) { z.dead = true; z.rising = false; z.deadT = 0; } });
  const L = LEVELS[G.lv];
  if (G.lv === 6) { endGame(e.kneelKilled ? 'killed' : 'killed'); return; }
  G.state = 'clear'; G.clearT = 0; M.play('win', false); save.level = Math.max(save.level, G.lv + 2); persist(); Acct.flush(); void L;
  if (document.exitPointerLock) document.exitPointerLock();
}
function endGame(kind) { G.state = 'ending'; G.endKind = kind; G.clearT = 0; M.play('win', false); save.level = 1; save.best = Math.max(save.best, G.weight); persist(); Acct.flush(); if (document.exitPointerLock) document.exitPointerLock(); }

function damageEnt(e, n, head) {
  if (e.dead || e.d.lost) return;
  const d = e.d;
  if (d.vanish && (e.vanish > 0 || e.kneel)) { if (!e.kneel) return; }
  let dmg = n * (head ? (d.headMul || (d.boss ? 1 : 3)) : 1);
  if (head && d.headKill) dmg = e.hp;
  e.hp -= dmg; e.hurt = 0.14; e.alert = true;
  A.sfx(head ? 'head' : 'hit', 0.8); G.hitmark = 0.15;
  if (d.vanish && !e.kneel && e.hp > 0) {
    if (e.hp <= e.maxhp * 0.2 && d.kneel) { e.kneel = true; showMsg('He drops the knife and kneels. [E] hand him over, or finish him.', 4); return; }
    e.vanish = 1.1; A.sfx('woy'); showMsg('"Woy!" He slips into the walls.', 1.4);
  }
  if (d.kneel && e.kneel && e.hp > 0 && n > 0) { /* shooting a kneeling man */ }
  if (d.boss && e.type === 'flanm' && !e.called && e.hp <= e.maxhp * 0.5) { e.called = true; for (let i = 0; i < 3; i++) { const z = spawnEnt('prisoner', e.x + rand(-1, 1), e.y + rand(-1, 1)); if (isWall(z.x, z.y)) { z.x = e.x; z.y = e.y; } z.alert = true; } showMsg('Flanm calls her zombies!', 2); }
  if (e.hp <= 0) { if (e.kneel) { e.kneelKilled = true; } killEnt(e, head); }
}
function alertNear(x, y, r) { G.ents.forEach((e) => { if (!e.dead && !e.d.lost && !e.alert && Math.hypot(e.x - x, e.y - y) < r) e.alert = true; }); }

function updateEnt(e, dt) {
  const p = G.player, d = e.d;
  if (e.dead) { e.deadT += dt; if (e.rising && e.deadT > e.riseAt) { e.rising = false; riseAgain(e); } return; }
  e.hurt = Math.max(0, e.hurt - dt); e.flash = Math.max(0, e.flash - dt); e.swing = Math.max(0, e.swing - dt);
  const dx = p.x - e.x, dy = p.y - e.y, dist = Math.hypot(dx, dy), ang = Math.atan2(dy, dx);
  if (d.lost) { // the Lost drift slowly; they are woken by name, never shot
    e.wait -= dt; if (e.wait <= 0) { const a = Math.random() * TAU; e.tx = e.x + Math.cos(a) * 2; e.ty = e.y + Math.sin(a) * 2; e.wait = rand(1.5, 4); }
    const tdx = e.tx - e.x, tdy = e.ty - e.y, td = Math.hypot(tdx, tdy); if (td > 0.2) tryMove(e, tdx / td * 0.5 * dt, tdy / td * 0.5 * dt, 0.2); e.walkT += dt * 2; return;
  }
  if (e.vanish > 0) { e.vanish -= dt; if (e.vanish <= 0) { // reappear near the player, often behind them
      const cells = reachable().filter(([x, y]) => { const dd = Math.hypot(x - p.x, y - p.y); return dd > 3 && dd < 8 && los(x, y, p.x, p.y); });
      const c = cells.length ? pick(cells) : [p.x, p.y - 4]; e.x = c[0]; e.y = c[1]; e.expose = 0; e.aim = 0.5; A.sfx('woy', 0.7); }
    return; }
  if (e.kneel) return;
  const see = dist < d.sight && los(e.x, e.y, p.x, p.y);
  if (!e.alert && see) e.alert = true;
  e.walkT += dt * 3;
  if (!e.alert) { // patrol / shamble
    e.wait -= dt; if (e.wait <= 0) { const cs = reachable(); const c = pick(cs); e.tx = c[0]; e.ty = c[1]; e.wait = rand(4, 9); e.near = null; }
    if (d.zombie) { const a = Math.random() * TAU; tryMove(e, Math.cos(a) * 0.2 * dt, Math.sin(a) * 0.2 * dt, 0.25); return; }
    const t = flowToward(e, e.tx, e.ty); if (t) tryMove(e, t[0] * d.sp * 0.5 * dt, t[1] * d.sp * 0.5 * dt, 0.25); return;
  }
  // alert
  e.sdT -= dt; if (e.sdT <= 0) { e.sd = -e.sd; e.sdT = rand(1, 2.5); }
  const speedUp = e.type === 'manchet' ? 1 + (1 - e.hp / e.maxhp) * 1.1 : (e.type === 'woch' && e.hp < e.maxhp * 0.5 ? 1.3 : 1);
  const sp = d.sp * speedUp;
  if (d.melee) {
    const reach = d.melee;
    if (dist > reach * 0.85) { const dirv = (see && dist < 4) ? [dx / dist, dy / dist] : stepDir(e); if (dirv) tryMove(e, dirv[0] * sp * dt, dirv[1] * sp * dt, 0.25); }
    e.cd -= dt;
    if (dist <= reach + 0.2 && e.cd <= 0 && e.aim <= 0) { e.aim = e.isBoss ? 0.35 : 0.22; }
    if (e.aim > 0) { e.aim -= dt; if (e.aim <= 0) { e.swing = 0.25; e.flash = 0.2; e.cd = e.isBoss ? 0.8 : 0.9; if (Math.hypot(p.x - e.x, p.y - e.y) <= reach + 0.45) { if (covered(e.x, e.y)) A.sfx('ricochet', 0.5); else playerHurt(d.dmg); } } }
    if (e.type === 'nwit' || e.type === 'nwitfinal') { e.expose = Math.max(0, e.expose - dt); }
  } else {
    // ranged
    const wantFar = d.pref || 5;
    if (!see || dist > wantFar + 1.5) { const dirv = see ? [dx / dist, dy / dist] : stepDir(e); if (dirv) tryMove(e, dirv[0] * sp * dt, dirv[1] * sp * dt, 0.25); }
    else if (dist < wantFar - 2) { tryMove(e, -dx / dist * sp * dt * 0.8, -dy / dist * sp * dt * 0.8, 0.25); }
    else { tryMove(e, -dy / dist * e.sd * sp * dt * 0.6, dx / dist * e.sd * sp * dt * 0.6, 0.25); }
    e.cd -= dt;
    if (see && dist < d.rng && e.cd <= 0 && e.aim <= 0 && e.burst <= 0) { e.aim = d.aimT || 0.3; if (e.type === 'sniper' || d.rocket) A.sfx('warn', d.rocket ? 0.8 : 0.5); }
    if (e.aim > 0) { e.aim -= dt; if (e.aim <= 0) { e.burst = d.burst || 1; e.burstT = 0; e.cd = rand(d.fire[0], d.fire[1]); } }
    if (e.burst > 0) { e.burstT -= dt; if (e.burstT <= 0) { e.burst--; e.burstT = 0.16; e.flash = 0.12; fireAt(e, dist); } }
  }
  // lunge for the final Nwit: surge forward after telegraph handled by melee branch
}
function fireAt(e, dist) {
  const p = G.player, d = e.d; if (d.rocket) { launchRocket(e); return; }
  const pan = clamp(Math.sin(Math.atan2(e.y - p.y, e.x - p.x) - p.a + Math.PI / 2), -1, 1) * 0.6;
  A.sfx('enemy', clamp(1.2 - dist / 25, 0.25, 1), pan);
  const still = Math.hypot(p.vx, p.vy) < 0.4; let acc = clamp(d.acc * (1 - dist / (d.rng * 1.5)) + 0.12, 0.1, 0.9);
  if (e.type === 'sniper') acc = still ? 0.9 : 0.4;
  if (Math.random() < acc && los(e.x, e.y, p.x, p.y)) {
    if (covered(e.x, e.y)) { A.sfx('ricochet', 0.7, pan); return; }
    playerHurt(Math.random() < (d.headCh == null ? 0.1 : d.headCh) ? HEAD_HIT : BULLET_HIT);
  }
}
function flowToward(e, tx, ty) {
  const dx = tx - e.x, dy = ty - e.y, d = Math.hypot(dx, dy); if (d < 0.3) return null;
  if (los(e.x, e.y, tx, ty)) return [dx / d, dy / d];
  // wander with wall sliding when no path field to the target: try perpendicular
  return [dx / d, dy / d];
}
function stepDir(e) { const t = flowStep(e); if (!t) return null; const dx = t[0] - e.x, dy = t[1] - e.y, d = Math.hypot(dx, dy); return d < 0.05 ? null : [dx / d, dy / d]; }
function riseAgain(e) {
  const k = Math.random(); const type = k < 0.5 ? 'prisoner' : k < 0.8 ? 'yaku' : 'brute'; const d = T[type];
  e.type = type; e.d = d; e.hp = d.hp; e.maxhp = d.hp; e.dead = false; e.risen = true; e.alert = true; e.cd = 1; e.aim = 0; e.burst = 0; A.sfx('groan', 0.9);
}

/* ---------- player actions ---------- */
function shootRay(ox, oy, headOK) { // one bullet along the crosshair, shifted by (ox, oy) screen pixels
  const p = G.player, hz = H / 2 + p.pitch, eye = eyeOf(p); let best = null, bd = 1e9;
  const col = clamp(((W / 2) + ox) | 0, 0, W - 1), wallD = zbuf[col], aimX = W / 2 + ox, aimY = H / 2 + oy;
  const dirx = Math.cos(p.a), diry = Math.sin(p.a), plx = -diry * 0.66, ply = dirx * 0.66, inv = 1 / (plx * diry - dirx * ply);
  for (const e of G.ents) {
    if (e.dead || e.vanish > 0) continue;
    const sx = e.x - p.x, sy = e.y - p.y, tx = inv * (diry * sx - dirx * sy), ty = inv * (-ply * sx + plx * sy);
    if (ty < 0.15 || ty > wallD || ty > bd) continue;
    const fg = FIG[e.d.fig], hgt = (H / ty) * 0.8 * fg.scale, wd = hgt * 32 / 52, scrX = (W / 2) * (1 + tx / ty);
    const bottom = hz + (H / ty) * eye, top = bottom - hgt, pad = TOUCH.on ? 14 : 5; // a wider hit zone on touch screens
    if (Math.abs(aimX - scrX) <= wd * 0.32 + pad && aimY >= top - 2 && aimY <= bottom) {
      let blocked = false; for (let i = 0; i < lowN[col]; i++) if (lowDist[col * 4 + i] < ty && aimY > lowTop[col * 4 + i]) blocked = true;
      if (!blocked) { best = { e, head: headOK && aimY < top + hgt * 0.26 }; bd = ty; }
    }
  }
  return best;
}
function fire() { // returns true when a shot or throw happened
  const p = G.player; if (G.state !== 'play' || p.cd > 0 || p.reload > 0) return false;
  const k = p.wpn, w = WP[k];
  if (w.grenade) return throwNade();
  if (p.crouch > 0.5 && !CROUCH_CAN_SHOOT) { if (G.msgT <= 0) showMsg('Stand up to shoot: release Z.', 1.2); p.cd = 0.25; return false; }
  if (w.melee) return stab();
  const g = p.gun[k];
  if (g.mag <= 0) { if (g.res > 0) startReload(); else { A.sfx('click'); p.cd = 0.3; } return false; }
  g.mag--; const auto = k === 'ak' && p.akAuto; p.cd = (k === 'ak' && !p.akAuto) ? w.cdSingle : w.cd; p.kick = w.kick; p.flashT = 0.07; A.sfx(w.snd, 0.9);
  alertNear(p.x, p.y, 16);
  const n = w.pellets || 1, lost = new Set();
  for (let i = 0; i < n; i++) {
    const ox = n > 1 ? rand(-1, 1) * w.spread : (auto ? rand(-5, 5) : 0), oy = n > 1 ? rand(-1, 1) * w.spread * 0.6 : (auto ? rand(-3, 3) : 0);
    const b = shootRay(ox, oy, n === 1); if (!b) continue;
    if (b.e.d.lost) lost.add(b.e); else damageEnt(b.e, w.dmg, b.head);
  }
  if (lost.size) { showMsg('Never shoot the Lost! Say their name: [E]', 2.5); G.weight += 2; Acct.penalize(5); G.flash = 0.2; A.sfx('hurt', 0.5); }
  return true;
}
function stab() {
  const p = G.player; p.cd = WP.knife.cd; p.stab = 1; A.sfx('knife');
  let best = null, bd = 1.8;
  for (const e of G.ents) {
    if (e.dead || e.d.lost || e.vanish > 0) continue;
    const dx = e.x - p.x, dy = e.y - p.y, d = Math.hypot(dx, dy); if (d > bd) continue;
    let da = Math.atan2(dy, dx) - p.a; da = Math.atan2(Math.sin(da), Math.cos(da)); if (Math.abs(da) < 0.7) { best = e; bd = d; }
  }
  if (best) { A.sfx('stab'); damageEnt(best, 2, false); }
  return true;
}
function throwNade() {
  const p = G.player; if (p.nades <= 0) { showMsg('No grenades left.', 1.2); p.cd = 0.3; return false; }
  p.nades--; p.cd = 0.9; A.sfx('throw'); alertNear(p.x, p.y, 12);
  const sp = 6.2 + clamp(p.pitch, -70, 70) / 70 * 2.5;
  G.nades.push({ x: p.x + Math.cos(p.a) * 0.4, y: p.y + Math.sin(p.a) * 0.4, vx: Math.cos(p.a) * sp, vy: Math.sin(p.a) * sp, fuse: 1.7, t: 0 });
  return true;
}
function selectWeapon(k) {
  const p = G.player; if (G.state !== 'play' || !WP[k] || p.wpn === k) return;
  p.wpn = k; p.reload = 0; p.cd = Math.max(p.cd, 0.2); A.sfx('click'); showMsg(WP[k].name + (k === 'ak' ? (p.akAuto ? ': AUTO' : ': SINGLE SHOT') : ''), 1.2);
  if (WP[k].mag && p.gun[k].mag <= 0) startReload();
}
function startReload() {
  const p = G.player, k = p.wpn, w = WP[k]; if (!w.mag || p.reload > 0) return;
  const g = p.gun[k]; if (g.res <= 0 || g.mag >= w.mag) return;
  p.reload = w.rl; p.reloadK = k; A.sfx('reload');
}
function useKey() {
  const p = G.player; if (G.state !== 'play') return;
  const k = G.ents.find((e) => e.kneel && !e.dead && Math.hypot(e.x - p.x, e.y - p.y) < 3.2);
  if (k) { endGame('captured'); return; }
  let near = null, nd = 3; G.ents.forEach((e) => { if (e.d.lost && !e.freed) { const d = Math.hypot(e.x - p.x, e.y - p.y); if (d < nd) { nd = d; near = e; } } });
  if (near) { near.freed = true; near.dead = true; near.deadT = 99; G.weight = Math.max(0, G.weight - 1); G.freed++; Acct.earn(2); A.sfx('name'); showMsg('You call the name. A Lost one wakes and walks out. Weight -1.', 2.5); return; }
}
function drink() { const p = G.player; if (G.state !== 'play' || p.bottles <= 0 || p.hp >= 100) return; p.bottles--; p.hp = 100; A.sfx('drink'); showMsg('You drink. Health restored.', 1.4); }

/* ---------- level flow ---------- */
function startLevel(i) {
  G.lv = i; const L = LEVELS[i]; G.map = grid(L.w, L.h); L.build(G.map); placeCover();
  G.nades = []; G.rockets = []; G.fxs = []; G.rktT = rand(20, 40); M.rate = 1;
  G.ents = []; G.items = []; G.boss = null; G.bossSpawned = false; G.player = newPlayer(); G.kills = 0; G.freed = 0; G.earnedT = 0; G.banner = null; G.msg = null; G.flash = 0;
  if (i === 0) { G.weight = 0; G.invKills = 0; }
  spawnWave(); buildFlow(); G.flowT = 0;
  G.state = 'intro'; G.introT = 0; M.stop(); A.init(); A.ambience(L.theme.amb);
  G.map.dist = null;
}
function beginPlay() { G.state = 'play'; A.resume(); M.play(LEVELS[G.lv].theme.music); lock(); try { if (navigator.wakeLock) navigator.wakeLock.request('screen').catch(() => {}); } catch (e) {} if (G.lv === 0) showMsg('Hold Z to hide behind sandbags. [1-6] weapons, [B] AK auto or single.', 5); }
function lock() { try { const r = canvas.requestPointerLock && canvas.requestPointerLock(); if (r && r.catch) r.catch(() => {}); } catch (e) {} }

/* ---------- main update ---------- */
let zbuf = new Float32Array(W);
// low obstacles seen in each screen column (up to 4, nearest first): distance and the screen row of their top edge
const lowN = new Uint8Array(W), lowDist = new Float32Array(W * 4), lowTop = new Float32Array(W * 4), lowD = new Float32Array(4), lowS = new Uint8Array(4), lowT = new Uint8Array(4);
function update(dt) {
  G.t += dt;
  if (G.msgT > 0) G.msgT -= dt; if (G.flash > 0) G.flash -= dt; if (G.shake > 0) G.shake -= dt; if (G.hitmark > 0) G.hitmark -= dt;
  if (G.banner) { G.banner.t -= dt; if (G.banner.t <= 0) G.banner = null; }
  const st = G.state;
  if (st === 'dead') { G.deadT += dt; return; }
  if (st === 'clear' || st === 'ending' || st === 'intro') { G.clearT = (G.clearT || 0) + dt; G.introT += dt; if (st === 'ending' || st === 'clear') G.ents.forEach((e) => updateEntSoft(e, dt)); return; }
  if (st !== 'play') return;
  const p = G.player;
  // look with arrows (mouse look is added by the mouse handler)
  if (keys.ArrowLeft) p.a -= 2.2 * dt; if (keys.ArrowRight) p.a += 2.2 * dt;
  if (keys.ArrowUp) p.pitch = clamp(p.pitch + 140 * dt, -70, 70); if (keys.ArrowDown) p.pitch = clamp(p.pitch - 140 * dt, -70, 70);
  let mx = 0, my = 0; const dirx = Math.cos(p.a), diry = Math.sin(p.a);
  if (keys.KeyW) { mx += dirx; my += diry; } if (keys.KeyS) { mx -= dirx; my -= diry; } if (keys.KeyD) { mx -= diry; my += dirx; } if (keys.KeyA) { mx += diry; my -= dirx; }
  let tmag = 0; if (TOUCH.on && (TOUCH.mx || TOUCH.my)) { const f = -TOUCH.my, sd = TOUCH.mx; mx += dirx * f - diry * sd; my += diry * f + dirx * sd; tmag = Math.min(1, Math.hypot(TOUCH.mx, TOUCH.my)); }
  p.crouch = clamp(p.crouch + clamp((keys.KeyZ ? 1 : 0) - p.crouch, -dt * 5, dt * 5), 0, 1);
  const ml = Math.hypot(mx, my), spd = ((keys.ShiftLeft || keys.ShiftRight || tmag > 0.92) ? 4.8 : 3.3) * (1 - 0.5 * p.crouch);
  if (ml > 0) { const an = (tmag > 0 && !(keys.KeyW || keys.KeyA || keys.KeyS || keys.KeyD)) ? Math.max(0.4, tmag) : 1; mx = mx / ml * spd * an; my = my / ml * spd * an; }
  if (TOUCH.on && !TOUCH.looking) p.pitch *= Math.max(0, 1 - 2.5 * dt);
  p.vx = mx; p.vy = my; tryMove(p, mx * dt, my * dt, 0.22); p.walk += ml > 0 ? dt * spd * 1.8 : 0; p.bob = Math.sin(p.walk) * (ml > 0 ? 1 : 0);
  p.cd = Math.max(0, p.cd - dt); p.kick = Math.max(0, p.kick - dt * 8); p.stab = Math.max(0, p.stab - dt * 4); p.ohT = Math.max(0, p.ohT - dt); p.flashT = Math.max(0, p.flashT - dt);
  if (p.inv > 0) { p.inv -= dt; if (p.inv <= 0) { p.inv = 0; M.rate = 1; A.sfx('powerdown'); showMsg('Invincibility is over.', 1.6); } }
  if (p.reload > 0) { p.reload -= dt; if (p.reload <= 0) { const k = p.reloadK, g = p.gun[k]; if (g) { const take = Math.min(WP[k].mag - g.mag, g.res); g.mag += take; g.res -= take; } } }
  const trig = mouseDown || keys.Space || TOUCH.fire, autoOn = p.wpn === 'ak' && p.akAuto;
  if (trig && !p.trig) p.buf = 0.2; p.trig = trig; p.buf = Math.max(0, p.buf - dt);
  if ((trig && autoOn) || p.buf > 0) { if (fire()) p.buf = 0; }
  // flow field for pathing
  G.flowT -= dt; const cell = (p.y | 0) * G.map.w + (p.x | 0); if (G.flowT <= 0 || cell !== G.flowCell) { buildFlow(); G.flowT = 0.35; G.flowCell = cell; }
  for (const e of G.ents) updateEnt(e, dt);
  // soft separation
  for (let i = 0; i < G.ents.length; i++) { const a = G.ents[i]; if (a.dead) continue; for (let j = i + 1; j < G.ents.length; j++) { const b = G.ents[j]; if (b.dead) continue; const dx = b.x - a.x, dy = b.y - a.y, d = Math.hypot(dx, dy); if (d > 0 && d < 0.55) { const push = (0.55 - d) * 0.5; tryMove(b, dx / d * push * 0.5, dy / d * push * 0.5, 0.2); tryMove(a, -dx / d * push * 0.5, -dy / d * push * 0.5, 0.2); } } const dx = a.x - p.x, dy = a.y - p.y, d = Math.hypot(dx, dy); if (d < 0.5 && d > 0) tryMove(a, dx / d * 0.05, dy / d * 0.05, 0.2); }
  // pickups
  for (let i = G.items.length - 1; i >= 0; i--) { const it = G.items[i]; if (Math.hypot(it.x - p.x, it.y - p.y) < 0.7) {
    if (it.kind === 'ammo') { let got = false; for (const k in p.gun) { const w = WP[k], g = p.gun[k], cap = Math.round(w.res * 1.6); if (g.res < cap) { g.res = Math.min(cap, g.res + w.pk); got = true; } } if (p.nades < 6) { p.nades++; got = true; } if (!got) continue; showMsg('Ammo and a grenade picked up.', 1.4); }
    else if (it.kind === 'bottle') { if (p.bottles >= 3) continue; p.bottles++; showMsg('Bottle picked up. [H] to drink.', 1.6); }
    A.sfx('pickup'); G.items.splice(i, 1); } }
  // grenades, rockets, blasts, and the random bandit with a bazooka
  updateNades(dt); if (G.state !== 'play') return; updateRockets(dt); if (G.state !== 'play') return;
  for (let i = G.fxs.length - 1; i >= 0; i--) { G.fxs[i].t += dt; if (G.fxs[i].t > G.fxs[i].life) G.fxs.splice(i, 1); }
  G.rktT -= dt; if (G.rktT <= 0) { G.rktT = rand(35, 70); spawnBazooka(); }
  // boss appears when the level is cleaned
  if (!G.bossSpawned && aliveCount() === 0) spawnBoss();
}
function updateEntSoft(e, dt) { if (e.dead) e.deadT += dt; }

/* ---------- rendering ---------- */
function rgb(r, g, b) { return (255 << 24) | (clamp(b, 0, 255) << 16) | (clamp(g, 0, 255) << 8) | clamp(r, 0, 255); }
function drawObj(o, scrX, unit, bottom, ty) {
  const xi = clamp(scrX | 0, 0, W - 1); if (ty >= zbuf[xi] + (o.k === 'x' ? 0.6 : 0.1)) return;
  if (o.k === 'n') { const r = Math.max(2, unit * 0.07), y = bottom - unit * 0.1 - Math.abs(Math.sin(o.t * 7)) * unit * 0.08 * Math.max(0, 1 - o.t); ctx.fillStyle = '#0a0a0d'; ctx.beginPath(); ctx.arc(scrX, y, r + 1, 0, TAU); ctx.fill(); ctx.fillStyle = Math.floor(o.t * 6) % 2 ? '#7aa06a' : '#3d5a3a'; ctx.beginPath(); ctx.arc(scrX, y, r, 0, TAU); ctx.fill(); }
  else if (o.k === 'r') { const rr = Math.max(3, unit * 0.09), y = bottom - unit * 0.45; ctx.globalAlpha = 0.4; ctx.fillStyle = '#ff9a2a'; ctx.beginPath(); ctx.arc(scrX, y, rr * 2.2, 0, TAU); ctx.fill(); ctx.globalAlpha = 1; ctx.fillStyle = '#0a0a0d'; ctx.beginPath(); ctx.arc(scrX, y, rr + 1, 0, TAU); ctx.fill(); ctx.fillStyle = '#5a6a44'; ctx.beginPath(); ctx.arc(scrX, y, rr, 0, TAU); ctx.fill(); ctx.fillStyle = '#ffd34a'; ctx.beginPath(); ctx.arc(scrX, y, rr * 0.45, 0, TAU); ctx.fill(); }
  else { const k = o.t / o.life, r = unit * (0.4 + 1.6 * Math.sqrt(k)), y = bottom - unit * 0.4; ctx.globalAlpha = clamp(1 - k, 0, 1) * 0.85; ctx.fillStyle = k < 0.3 ? '#fff2a0' : '#ff8a2a'; ctx.beginPath(); ctx.arc(scrX, y, r, 0, TAU); ctx.fill(); ctx.fillStyle = '#c0301a'; ctx.beginPath(); ctx.arc(scrX, y, r * 0.6, 0, TAU); ctx.fill(); ctx.globalAlpha = 1; }
}
function renderWorld() {
  const L = LEVELS[G.lv], th = L.theme, p = G.player, eye = eyeOf(p), hz = Math.round(H / 2 + p.pitch + (G.shake > 0 ? rand(-3, 3) : 0) + p.bob * 1.5);
  // floor and ceiling bands
  for (let y = 0; y < H; y++) {
    let col;
    if (y < hz) { const t = clamp(y / Math.max(1, hz), 0, 1); if (th.sky) { const s = th.sky; col = rgb(s[0] + (s[3] - s[0]) * t, s[1] + (s[4] - s[1]) * t, s[2] + (s[5] - s[2]) * t); } else { const k = 0.5 + 0.5 * t; col = rgb(th.ceil[0] * k, th.ceil[1] * k, th.ceil[2] * k); } }
    else { const t = clamp((y - hz) / (H / 2), 0, 1.2), k = 0.35 + 0.65 * Math.min(1, t * 1.6), f = th.floor, fg = th.fog; col = rgb(fg[0] + (f[0] - fg[0]) * k, fg[1] + (f[1] - fg[1]) * k, fg[2] + (f[2] - fg[2]) * k); }
    buf.fill(col, y * W, y * W + W);
  }
  const dirx = Math.cos(p.a), diry = Math.sin(p.a), plx = -diry * 0.66, ply = dirx * 0.66, fog = th.fog, tint = th.tint, maxD = th.maxD, texC = TEX[COVER];
  for (let x = 0; x < W; x++) {
    const cam = 2 * x / W - 1, rx = dirx + plx * cam, ry = diry + ply * cam;
    let mapX = p.x | 0, mapY = p.y | 0; const ddx = Math.abs(1 / (rx || 1e-9)), ddy = Math.abs(1 / (ry || 1e-9));
    let stepX, stepY, sdx, sdy;
    if (rx < 0) { stepX = -1; sdx = (p.x - mapX) * ddx; } else { stepX = 1; sdx = (mapX + 1 - p.x) * ddx; }
    if (ry < 0) { stepY = -1; sdy = (p.y - mapY) * ddy; } else { stepY = 1; sdy = (mapY + 1 - p.y) * ddy; }
    let side = 0, hit = 0, guard = 0, nlow = 0;
    while (!hit && guard++ < 64) {
      if (sdx < sdy) { sdx += ddx; mapX += stepX; side = 0; } else { sdy += ddy; mapY += stepY; side = 1; }
      if (mapX < 0 || mapY < 0 || mapX >= G.map.w || mapY >= G.map.h) { hit = 1; mapX = clamp(mapX, 0, G.map.w - 1); mapY = clamp(mapY, 0, G.map.h - 1); break; }
      hit = G.map[mapY][mapX];
      if (hit === COVER) { // a low obstacle: remember it, the ray goes on over it
        if (nlow < 4) { const dd = Math.max(0.05, side === 0 ? sdx - ddx : sdy - ddy); lowD[nlow] = dd; lowS[nlow] = side; let wx2 = side === 0 ? p.y + dd * ry : p.x + dd * rx; wx2 -= Math.floor(wx2); let t2 = (wx2 * TS) | 0; if ((side === 0 && rx > 0) || (side === 1 && ry < 0)) t2 = TS - 1 - t2; lowT[nlow] = t2; nlow++; }
        hit = 0;
      }
    }
    const dist = Math.max(0.05, side === 0 ? sdx - ddx : sdy - ddy); zbuf[x] = dist;
    let wx = side === 0 ? p.y + dist * ry : p.x + dist * rx; wx -= Math.floor(wx);
    let tx = (wx * TS) | 0; if ((side === 0 && rx > 0) || (side === 1 && ry < 0)) tx = TS - 1 - tx;
    const lh = H / dist, y0 = hz - lh * (1 - eye), y1 = hz + lh * eye, tex = TEX[hit] || TEX[1];
    const k = clamp(1 - dist / maxD, 0, 1) * (side ? 0.72 : 1), a = 1 - k;
    const ys = Math.max(0, y0 | 0), ye = Math.min(H - 1, y1 | 0), step = TS / lh; let ty = (ys - y0) * step;
    const fr = fog[0] * a, fgc = fog[1] * a, fb = fog[2] * a;
    for (let y = ys; y <= ye; y++) { const i = (((ty | 0) & 63) * TS + tx) * 3; buf[y * W + x] = rgb(tex[i] * tint[0] * k + fr, tex[i + 1] * tint[1] * k + fgc, tex[i + 2] * tint[2] * k + fb); ty += step; }
    // low obstacles of this column, far to near
    lowN[x] = nlow;
    for (let i = nlow - 1; i >= 0; i--) {
      const d = lowD[i], lh2 = H / d, fl = hz + lh2 * eye, tp = fl - lh2 * COVER_H; lowDist[x * 4 + i] = d; lowTop[x * 4 + i] = tp;
      const kk = clamp(1 - d / maxD, 0, 1) * (lowS[i] ? 0.72 : 1), aa = 1 - kk, stp = TS / (fl - tp), ys2 = Math.max(0, tp | 0), ye2 = Math.min(H - 1, fl | 0); let ty2 = (ys2 - tp) * stp;
      const fr2 = fog[0] * aa, fg2 = fog[1] * aa, fb2 = fog[2] * aa;
      for (let y = ys2; y <= ye2; y++) { const ii = (((ty2 | 0) & 63) * TS + lowT[i]) * 3; buf[y * W + x] = rgb(texC[ii] * tint[0] * kk + fr2, texC[ii + 1] * tint[1] * kk + fg2, texC[ii + 2] * tint[2] * kk + fb2); ty2 += stp; }
    }
  }
  ctx.putImageData(img, 0, 0);
  // moon on outdoor levels
  if (th.sky) { ctx.fillStyle = '#e8e6d0'; ctx.beginPath(); ctx.arc(W * 0.8 - p.a * 40 % W, 40 + p.pitch * 0.4, 12, 0, TAU); ctx.fill(); }
  // sprites
  const inv = 1 / (plx * diry - dirx * ply), list = [];
  for (const e of G.ents) list.push({ e, x: e.x, y: e.y, t: 'e' });
  for (const it of G.items) list.push({ it, x: it.x, y: it.y, t: 'i' });
  for (const n of G.nades) list.push({ o: { k: 'n', t: n.t }, x: n.x, y: n.y, t: 'o' });
  for (const r of G.rockets) list.push({ o: { k: 'r', t: r.t }, x: r.x, y: r.y, t: 'o' });
  for (const f of G.fxs) list.push({ o: { k: 'x', t: f.t, life: f.life }, x: f.x, y: f.y, t: 'o' });
  list.forEach((s) => { const sx = s.x - p.x, sy = s.y - p.y; s.tx = inv * (diry * sx - dirx * sy); s.ty = inv * (-ply * sx + plx * sy); });
  list.sort((a, b) => b.ty - a.ty);
  for (const s of list) {
    if (s.ty < 0.2) continue;
    const scrX = (W / 2) * (1 + s.tx / s.ty), unit = H / s.ty, bottom = hz + unit * eye, alpha = clamp(1.25 - s.ty / maxD, 0.15, 1);
    if (s.t === 'o') { drawObj(s.o, scrX, unit, bottom, s.ty); continue; }
    let src, sw, sh, dw, dh, dyy;
    if (s.t === 'i') { src = ITEM[s.it.kind]; dh = unit * 0.28; dw = dh; sw = 16; sh = 16; dyy = bottom - dh - unit * 0.08 + Math.sin(G.t * 3 + s.x) * unit * 0.02; }
    else {
      const e = s.e, fg = FIG[e.d.fig]; if (e.vanish > 0) continue;
      if (e.dead) { if (e.deadT > 30 && !e.rising) continue; src = fg.dead; sw = fg.h; sh = fg.w; dh = unit * 0.3 * fg.scale; dw = dh * sw / sh; dyy = bottom - dh; }
      else {
        const aiming = e.aim > 0 || e.flash > 0 || e.swing > 0; src = aiming ? fg.aim : (Math.floor(e.walkT) % 2 ? fg.walk : fg.stand);
        if (e.hurt > 0) src = aiming ? fg.hurtAim : fg.hurt; sw = fg.w; sh = fg.h; dh = unit * 0.8 * fg.scale; dw = dh * sw / sh; dyy = bottom - dh;
        if (e.rising) continue;
      }
      if (e.dead && e.rising && Math.floor(G.t * 8) % 2) continue;
    }
    const left = scrX - dw / 2, x0 = Math.max(0, Math.floor(left)), x1 = Math.min(W - 1, Math.ceil(left + dw));
    ctx.globalAlpha = alpha; ctx.imageSmoothingEnabled = false;
    // the part of the sprite below the top of a nearer low obstacle is hidden by it
    const cutAt = (x) => { let c = Infinity; for (let i = 0; i < lowN[x]; i++) if (lowDist[x * 4 + i] < s.ty) c = Math.min(c, lowTop[x * 4 + i]); return c >= dyy + dh ? Infinity : c; };
    const draw = (xs, xe, cut) => {
      const sx0 = ((xs - left) / dw) * sw, sx1 = ((xe - left) / dw) * sw; let ddh = dh, ssh = sh;
      if (cut !== Infinity) { const vh = cut - dyy; if (vh <= 0) return; ddh = vh; ssh = sh * vh / dh; }
      ctx.drawImage(src, clamp(sx0, 0, sw - 0.01), 0, Math.max(0.5, Math.min(sw - sx0, sx1 - sx0)), ssh, xs, dyy, xe - xs, ddh);
    };
    let runStart = -1, runCut = Infinity;
    for (let x = x0; x <= x1 + 1; x++) {
      const vis = x <= x1 && s.ty < zbuf[x], cut = vis ? cutAt(x) : Infinity;
      if (runStart >= 0 && (!vis || cut !== runCut)) { draw(runStart, x, runCut); runStart = -1; }
      if (vis && runStart < 0) { runStart = x; runCut = cut; }
    }
    ctx.globalAlpha = 1;
    if (s.t === 'e' && s.e.isBoss && s.e.aim > 0 && s.e.type === 'sniper') { ctx.fillStyle = 'rgba(255,40,40,0.9)'; ctx.fillRect(scrX - 1, dyy + dh * 0.3, 2, 6); }
    if (s.t === 'e' && s.e.type === 'bazooka' && s.e.aim > 0 && !s.e.dead) { ctx.fillStyle = 'rgba(255,170,40,0.95)'; ctx.fillRect(scrX - 1, dyy - 7, 2, 5); }
  }
}
function drawWeapon() {
  const p = G.player, k = p.wpn, w = WP[k], bob = Math.sin(p.walk) * 4, kick = p.kick * 14;
  const reloadDip = p.reload > 0 && w.rl ? Math.sin(Math.min(1, (w.rl - p.reload) / w.rl) * Math.PI) * 60 : 0, hide = p.crouch * 130;
  if (w.grenade && p.cd > 0.35) return;
  ctx.save(); ctx.translate(W * 0.66 + bob * 1.2, H + 10 + Math.abs(bob) * 0.8 + kick + reloadDip + hide); ctx.rotate(-0.1);
  const P = (pts, col) => { ctx.fillStyle = col; ctx.beginPath(); ctx.moveTo(pts[0][0], pts[0][1]); for (let i = 1; i < pts.length; i++) ctx.lineTo(pts[i][0], pts[i][1]); ctx.closePath(); ctx.fill(); ctx.strokeStyle = '#08080a'; ctx.lineWidth = 2; ctx.stroke(); };
  const hands = () => { P([[-30, -30], [-6, -30], [-6, 50], [-60, 60]], '#d98d4f'); P([[2, -26], [28, -20], [34, 50], [6, 56]], '#15151a'); };
  let fy = null;
  if (k === 'ak') {
    P([[-12, -190], [-12, -150], [-6, -150], [-6, -190]], '#1a1a1f');                // barrel
    P([[-16, -198], [-2, -198], [-2, -186], [-16, -186]], '#222');                    // front sight
    P([[-26, -150], [18, -150], [22, -60], [-30, -60]], '#6e3417');                   // wooden handguard
    P([[-22, -150], [-14, -150], [-16, -60], [-26, -60]], '#8a4a24');
    P([[-34, -60], [26, -60], [34, 20], [-40, 20]], '#2a2b30');                       // receiver
    P([[-12, 20], [26, 20], [40, 110], [20, 118]], '#18181c');                        // magazine
    P([[-46, -78], [-14, -78], [-12, -20], [-52, -14]], '#15151a');                   // left glove
    P([[-64, -50], [-30, -50], [-34, 60], [-84, 60]], '#d98d4f');                     // forearm
    P([[18, -40], [60, -20], [64, 80], [22, 80]], '#15151a');                         // right hand
    fy = -200;
  } else if (k === 'glock') {
    P([[-10, -120], [14, -120], [14, -70], [-10, -70]], '#1b1b1f'); P([[-14, -70], [18, -70], [18, 0], [-14, 0]], '#2a2b30'); P([[-6, 0], [16, 0], [22, 70], [2, 74]], '#18181c'); hands(); fy = -124;
  } else if (k === 'magnum') {
    P([[-14, -160], [16, -160], [16, -80], [-14, -80]], '#2c2c33'); P([[-10, -168], [12, -168], [12, -158], [-10, -158]], '#1b1b1f');
    P([[-20, -80], [24, -80], [24, -14], [-20, -14]], '#41414b'); P([[-16, -70], [20, -70], [20, -60], [-16, -60]], '#222');
    P([[-24, -14], [28, -14], [28, 6], [-24, 6]], '#2c2c33'); P([[-6, 6], [24, 6], [32, 84], [6, 90]], '#5a3a1e');
    P([[-40, -40], [-14, -40], [-14, 50], [-70, 60]], '#d98d4f'); P([[8, -24], [36, -18], [40, 70], [10, 74]], '#15151a'); fy = -170;
  } else if (k === 'shotgun') {
    P([[-16, -230], [0, -230], [0, -40], [-16, -40]], '#26262c'); P([[2, -230], [18, -230], [18, -40], [2, -40]], '#303038');
    P([[-24, -130], [26, -130], [28, -60], [-26, -60]], '#6e3417'); P([[-30, -40], [32, -40], [38, 40], [-36, 40]], '#2a2b30');
    P([[-20, 40], [30, 40], [46, 120], [10, 130]], '#6e3417'); P([[-56, -90], [-24, -90], [-26, -20], [-84, -12]], '#d98d4f'); P([[22, -20], [60, 0], [64, 90], [24, 90]], '#15151a'); fy = -232;
  } else if (k === 'knife') {
    ctx.translate(-p.stab * 14, -p.stab * 80); ctx.rotate(p.stab * 0.3);
    P([[-4, -150], [10, -100], [10, -30], [-8, -30]], '#d9dde6'); P([[-4, -150], [0, -146], [0, -30], [-8, -30]], '#f4f6fa');
    P([[-14, -30], [16, -30], [16, -20], [-14, -20]], '#8a6a2a'); P([[-8, -20], [12, -20], [14, 40], [-6, 44]], '#2a1a10');
    P([[-22, -18], [24, -20], [30, 60], [-28, 64]], '#d98d4f');
  } else {
    P([[-24, -60], [22, -60], [28, -10], [24, 40], [-26, 40], [-30, -10]], '#3d5a3a'); P([[-8, -76], [10, -76], [10, -60], [-8, -60]], '#777');
    ctx.strokeStyle = '#ccc'; ctx.lineWidth = 2; ctx.beginPath(); ctx.arc(16, -78, 8, 0, TAU); ctx.stroke();
    P([[-52, -20], [-22, -20], [-22, 60], [-72, 60]], '#d98d4f'); P([[22, -24], [56, -10], [60, 70], [26, 70]], '#15151a');
  }
  if (p.flashT > 0 && fy !== null) { const big = (k === 'shotgun' || k === 'magnum') ? 1.5 : 1; ctx.fillStyle = '#ffd34a'; ctx.beginPath(); for (let i = 0; i < 10; i++) { const r = (i % 2 ? 10 : 26) * big, a = i / 10 * TAU; ctx.lineTo(-9 + Math.cos(a) * r, fy + Math.sin(a) * r); } ctx.closePath(); ctx.fill(); ctx.fillStyle = '#fff'; ctx.beginPath(); ctx.arc(-9, fy, 7 * big, 0, TAU); ctx.fill(); }
  ctx.restore();
}
function text(s, x, y, size, color, align = 'left', shadow = true) {
  ctx.font = 'bold ' + size + 'px ui-monospace, Menlo, Consolas, monospace'; ctx.textAlign = align; ctx.textBaseline = 'top';
  if (shadow) { ctx.fillStyle = '#000'; ctx.fillText(s, x + 1, y + 1); } ctx.fillStyle = color; ctx.fillText(s, x, y);
}
function wrap(s, x, y, maxW, size, color, lh, align = 'center') {
  ctx.font = 'bold ' + size + 'px ui-monospace, Menlo, Consolas, monospace'; const words = s.split(' '); let line = '', yy = y;
  for (const w of words) { const t = line ? line + ' ' + w : w; if (ctx.measureText(t).width > maxW && line) { text(line, x, yy, size, color, align); yy += lh; line = w; } else line = t; }
  if (line) text(line, x, yy, size, color, align); return yy + lh;
}
const coins = (t) => (t / 10).toFixed(1);
function drawHUD() {
  const p = G.player, L = LEVELS[G.lv];
  // crosshair and hit marker
  ctx.strokeStyle = G.hitmark > 0 ? '#ff4a4a' : '#e8ffe8'; ctx.lineWidth = 1.5; const cx = W / 2, cy = H / 2;
  ctx.beginPath(); ctx.moveTo(cx - 9, cy); ctx.lineTo(cx - 3, cy); ctx.moveTo(cx + 3, cy); ctx.lineTo(cx + 9, cy); ctx.moveTo(cx, cy - 9); ctx.lineTo(cx, cy - 3); ctx.moveTo(cx, cy + 3); ctx.lineTo(cx, cy + 9); ctx.stroke();
  if (G.flash > 0) { ctx.fillStyle = 'rgba(200,0,0,' + clamp(G.flash * 1.4, 0, 0.5) + ')'; ctx.fillRect(0, 0, W, H); }
  if (p.hp < 30) { ctx.fillStyle = 'rgba(160,0,0,' + (0.12 + 0.08 * Math.sin(G.t * 6)) + ')'; ctx.fillRect(0, 0, W, H); }
  // health
  ctx.fillStyle = '#000a'; ctx.fillRect(8, H - 30, 124, 22); ctx.fillStyle = '#2a0a0a'; ctx.fillRect(12, H - 26, 116, 8);
  ctx.fillStyle = p.hp > 50 ? '#3ed16a' : p.hp > 25 ? '#e6b030' : '#e23a3a'; ctx.fillRect(12, H - 26, 116 * p.hp / 100, 8);
  text('HP ' + Math.ceil(p.hp) + '   BOTTLES ' + p.bottles + ' [H]', 12, H - 17, 9, '#fff');
  // ammo
  const wk = p.wpn, wp = WP[wk];
  const ammoStr = wp.melee ? 'KNIFE' : wp.grenade ? 'GRENADES ' + p.nades : (p.reload > 0 ? 'RELOADING' : p.gun[wk].mag + ' / ' + p.gun[wk].res);
  const ax = TOUCH.on ? W - 72 : W - 10; // on touch screens the buttons cover the right edge
  text(ammoStr, ax, H - 24, 13, '#ffe27a', 'right'); text(wp.name + (wk === 'ak' ? (p.akAuto ? '  AUTO' + (TOUCH.on ? '' : ' [B]') : '  SINGLE' + (TOUCH.on ? '' : ' [B]')) : ''), ax, H - 40, 9, '#fff', 'right');
  const ab = { knife: 'KNF', glock: 'GLK', magnum: 'MAG', shotgun: 'SHG', ak: 'AK', grenade: 'GRN' };
  if (!TOUCH.on) WORDER.forEach((id, i) => { const x = 138 + i * 44; ctx.fillStyle = id === wk ? '#7B5CF6' : '#000a'; ctx.fillRect(x, H - 24, 41, 16); text((i + 1) + ' ' + ab[id], x + 20, H - 21, 8, id === wk ? '#fff' : '#aaa', 'center'); });
  // top bar
  ctx.fillStyle = '#000a'; ctx.fillRect(0, 0, W, 20);
  text('LV ' + (G.lv + 1) + ' ' + L.name, 8, 5, 10, '#fff');
  const left = aliveCount(); text(G.bossSpawned ? 'BOSS FIGHT' : 'ENEMIES LEFT ' + left, W / 2, 5, 10, G.bossSpawned ? '#ff6a6a' : '#fff', 'center');
  text((Acct.signedIn ? 'LIONCOIN ' + coins(Acct.coinsT()) : 'GUEST') + '   CHANCES ' + Acct.chancesLeft() + '/' + DAILY, W - (TOUCH.on ? 46 : 8), 5, 10, '#ffd34a', 'right');
  text('WEIGHT ' + G.weight, 8, 24, 9, '#c9a8ff');
  // boss bar
  if (G.boss && !G.boss.dead) { const b = G.boss; ctx.fillStyle = '#000b'; ctx.fillRect(W / 2 - 110, 38, 220, 12); ctx.fillStyle = '#7a1010'; ctx.fillRect(W / 2 - 108, 40, 216, 8); ctx.fillStyle = '#e83a3a'; ctx.fillRect(W / 2 - 108, 40, 216 * clamp(b.hp / b.maxhp, 0, 1), 8); text(b.d.name.toUpperCase(), W / 2, 51, 9, '#fff', 'center'); }
  // minimap
  const cs = 3, mx0 = W - G.map.w * cs - 6, my0 = 24; ctx.globalAlpha = 0.75; ctx.fillStyle = '#000'; ctx.fillRect(mx0 - 2, my0 - 2, G.map.w * cs + 4, G.map.h * cs + 4);
  for (let y = 0; y < G.map.h; y++) for (let x = 0; x < G.map.w; x++) if (G.map[y][x]) { ctx.fillStyle = '#6a6a80'; ctx.fillRect(mx0 + x * cs, my0 + y * cs, cs, cs); }
  G.ents.forEach((e) => { if (e.dead) return; ctx.fillStyle = e.d.lost ? '#9fe' : e.isBoss ? '#ff0' : '#f33'; ctx.fillRect(mx0 + e.x * cs - 1, my0 + e.y * cs - 1, 2, 2); });
  ctx.fillStyle = '#fa3'; G.rockets.forEach((r) => ctx.fillRect(mx0 + r.x * cs - 1, my0 + r.y * cs - 1, 3, 3));
  ctx.fillStyle = '#3f3'; ctx.fillRect(mx0 + p.x * cs - 1, my0 + p.y * cs - 1, 3, 3); ctx.globalAlpha = 1;
  if (G.banner) { const a = clamp(G.banner.t, 0, 1); ctx.fillStyle = 'rgba(0,0,0,' + 0.5 * a + ')'; ctx.fillRect(0, H / 2 - 50, W, 40); text(G.banner.text, W / 2, H / 2 - 42, 20, G.banner.col || '#ff5a5a', 'center'); }
  if (p.crouch > 0.5) { const cv = coverNear(); text(cv ? 'HIDING BEHIND COVER' : 'CROUCHED: NO COVER IN FRONT', W / 2, H - 92, 10, cv ? '#7dff9a' : '#ffb040', 'center'); }
  if (p.inv > 0) { ctx.strokeStyle = 'rgba(255,211,74,' + (0.5 + 0.3 * Math.sin(G.t * 12)) + ')'; ctx.lineWidth = 5; ctx.strokeRect(2, 2, W - 4, H - 4); text('INVINCIBLE ' + p.inv.toFixed(1) + 's', W / 2, 24, 11, '#ffd34a', 'center'); }
  if (G.msgT > 0 && G.msg) { ctx.fillStyle = '#000a'; ctx.fillRect(W / 2 - 200, H - 70, 400, 22); text(G.msg, W / 2, H - 66, 10, '#fff', 'center'); }
}
function panel(h = 200) { ctx.fillStyle = 'rgba(4,4,12,0.82)'; ctx.fillRect(40, (H - h) / 2, W - 80, h); ctx.strokeStyle = '#7B5CF6'; ctx.lineWidth = 2; ctx.strokeRect(40, (H - h) / 2, W - 80, h); }
function button(label, x, y, w, h, enabled, fn) {
  ui.buttons.push({ x, y, w, h, fn, enabled }); const sel = ui.buttons.length - 1 === ui.sel;
  ctx.fillStyle = !enabled ? '#2a2a38' : sel ? '#7B5CF6' : '#3a2f78'; ctx.fillRect(x, y, w, h); ctx.strokeStyle = sel ? '#fff' : '#9a86ff'; ctx.lineWidth = 1; ctx.strokeRect(x + 0.5, y + 0.5, w, h);
  text(label, x + w / 2, y + h / 2 - 6, 11, enabled ? '#fff' : '#777', 'center');
}
function countdown() { const n = new Date(), m = new Date(Date.UTC(n.getUTCFullYear(), n.getUTCMonth(), n.getUTCDate() + 1)); const s = Math.max(0, Math.floor((m - n) / 1000)); return String(Math.floor(s / 3600)).padStart(2, '0') + ':' + String(Math.floor(s / 60) % 60).padStart(2, '0') + ':' + String(s % 60).padStart(2, '0'); }
function drawScreens() {
  ui.buttons = [];
  Acct.refreshDay();
  if (G.state === 'chances' || G.state === 'tomorrow') { if (Acct.chancesLeft() > 0) { G.state = 'menu'; ui.sel = 0; } }
  const s = G.state, L = LEVELS[G.lv];
  if (s === 'menu') {
    ctx.fillStyle = '#05050fcc'; ctx.fillRect(0, 0, W, H);
    text('WOUT SAN RETOU', W / 2, 24, 30, '#fff', 'center'); text('A R C A D E', W / 2, 60, 18, '#7B5CF6', 'center');
    text('Seven levels, from the yard to the rooftop. Clean each level, then put its boss down.', W / 2, 90, 9, '#ccc', 'center');
    text('Every kill earns 0.1 LionCoin in your OK Music wallet. ' + DAILY + ' chances a day.', W / 2, 104, 9, '#ffd34a', 'center');
    text(Acct.signedIn ? 'Signed in. Every kill pays 0.1 LionCoin.' : 'Guest mode: sign in to OK Music to earn and spend LionCoin.', W / 2, 117, 8, '#999', 'center');
    text((Acct.signedIn ? 'LIONCOIN ' + coins(Acct.coinsT()) : 'LIONCOIN: GUEST') + '    CHANCES TODAY ' + Acct.chancesLeft() + '/' + DAILY, W / 2, 134, 12, '#ffd34a', 'center');
    if (Acct.chancesLeft() <= 0) {
      button('OUT OF CHANCES: OPTIONS', W / 2 - 110, 158, 220, 24, true, () => { G.state = 'chances'; ui.sel = 0; });
    } else {
      const cont = save.level > 1 ? save.level : 0;
      button(cont ? 'CONTINUE: LEVEL ' + cont : 'START GAME', W / 2 - 110, 158, 220, 24, true, () => { A.init(); A.resume(); startLevel(cont ? cont - 1 : 0); });
      if (cont) button('NEW GAME', W / 2 - 110, 186, 220, 20, true, () => { A.init(); A.resume(); save.level = 1; persist(); startLevel(0); });
    }
    if (!Acct.signedIn) button('SIGN IN TO EARN LIONCOIN', W / 2 - 110, 210, 220, 20, true, () => { window.open('/', '_blank', 'noopener'); });
    if (TOUCH.on) {
      text('Left thumb: move.  Right thumb: look.  FIRE: shoot.', W / 2, 234, 8, '#aaa', 'center');
      text('Hold HIDE behind sandbags and crates: they stop bullets and rockets.', W / 2, 245, 8, '#7dff9a', 'center');
      text('Six weapons at the bottom. R reload, GRN grenade, KNF knife, E use, H drink.', W / 2, 256, 8, '#aaa', 'center');
      text('Every 20 kills: 10 seconds of invincibility. Best in landscape.', W / 2, 267, 8, '#888', 'center');
    } else {
      text('WASD move   Shift run   Mouse aim   Click fire   1-6 weapons   B auto/single', W / 2, 234, 8, '#aaa', 'center');
      text('Hold Z: hide behind sandbags and crates (they stop bullets and rockets)', W / 2, 245, 8, '#7dff9a', 'center');
      text('R reload   F knife   G grenade   H drink   E name a Lost one   M music   Esc pause', W / 2, 256, 8, '#aaa', 'center');
      text('Every 20 kills: 10 seconds of invincibility. Mouse not working? Arrows turn, Space fires.', W / 2, 267, 8, '#888', 'center');
    }
    text('Fictional story. Gunfire and horror imagery. Teenagers and older.', W / 2, 280, 8, '#777', 'center');
  } else if (s === 'intro') {
    ctx.fillStyle = '#000'; ctx.fillRect(0, 0, W, H);
    text('LEVEL ' + (G.lv + 1), W / 2, 70, 26, '#7B5CF6', 'center'); text(L.name, W / 2, 104, 24, '#fff', 'center');
    wrap(L.sub, W / 2, 144, 380, 10, '#ccc', 14); text('Boss: ' + L.bossName, W / 2, 190, 12, '#ff6a6a', 'center');
    text(G.introT > 0.4 ? (TOUCH.on ? 'TAP TO START' : 'CLICK OR PRESS ENTER TO START') : '', W / 2, 232, 12, '#ffd34a', 'center');
    ui.buttons.push({ x: 0, y: 0, w: W, h: H, enabled: true, fn: () => { if (G.introT > 0.4) beginPlay(); } });
  } else if (s === 'paused') {
    ctx.fillStyle = 'rgba(0,0,0,0.6)'; ctx.fillRect(0, 0, W, H); text('PAUSED', W / 2, 110, 26, '#fff', 'center'); text(TOUCH.on ? 'TAP TO RESUME' : 'CLICK TO RESUME', W / 2, 150, 12, '#ffd34a', 'center');
    ui.buttons.push({ x: 0, y: 0, w: W, h: H, enabled: true, fn: () => { G.state = 'play'; lock(); } });
  } else if (s === 'dead') {
    ctx.fillStyle = 'rgba(90,0,0,' + clamp(G.deadT * 0.8, 0, 0.7) + ')'; ctx.fillRect(0, 0, W, H);
    if (G.deadT > 1) {
      panel(170); text('YOU FELL', W / 2, 88, 24, '#ff5a5a', 'center');
      text('Chances left today: ' + Acct.chancesLeft() + ' / ' + DAILY, W / 2, 124, 11, '#fff', 'center');
      if (Acct.chancesLeft() > 0) button('TRY LEVEL ' + (G.lv + 1) + ' AGAIN', W / 2 - 100, 154, 200, 26, true, () => startLevel(G.lv));
      else button('NO CHANCES LEFT: OPTIONS', W / 2 - 110, 154, 220, 26, true, () => { G.state = 'chances'; ui.sel = 0; });
    }
  } else if (s === 'chances') {
    ctx.fillStyle = '#05050f'; ctx.fillRect(0, 0, W, H); panel(210);
    text('NO CHANCES LEFT TODAY', W / 2, 72, 18, '#ff5a5a', 'center'); text('You used all ' + DAILY + ' chances.', W / 2, 102, 10, '#fff', 'center');
    text(Acct.signedIn ? 'Your LionCoin: ' + coins(Acct.coinsT()) : 'Guest: sign in to OK Music to use LionCoin', W / 2, 120, 12, '#ffd34a', 'center');
    const can = Acct.signedIn && Acct.coinsT() >= CONTINUE_COST && !G.paying;
    button(G.paying ? 'PAYING...' : 'PAY 2 LIONCOIN FOR ' + DAILY + ' MORE CHANCES', W / 2 - 150, 146, 300, 26, can, () => {
      G.paying = true; G.payMsg = '';
      Acct.payChances().then((ok) => { G.paying = false; if (ok) { A.sfx('coin'); startLevel(G.lv); } else G.payMsg = 'Payment failed. Check your balance and connection.'; });
    });
    button('COME BACK TOMORROW', W / 2 - 150, 180, 300, 26, true, () => { G.state = 'tomorrow'; ui.sel = 0; });
    if (G.payMsg) text(G.payMsg, W / 2, 214, 8, '#ff8a8a', 'center');
    else if (!Acct.signedIn) button('SIGN IN TO OK MUSIC', W / 2 - 80, 212, 160, 18, true, () => { window.open('/', '_blank', 'noopener'); });
    else if (!can && !G.paying) text('You need 2.0 LionCoin. Earn 0.1 for every kill.', W / 2, 214, 8, '#999', 'center');
  } else if (s === 'tomorrow') {
    ctx.fillStyle = '#05050f'; ctx.fillRect(0, 0, W, H); panel(180);
    text('SEE YOU TOMORROW', W / 2, 84, 20, '#fff', 'center'); text('New chances in', W / 2, 122, 11, '#ccc', 'center'); text(countdown(), W / 2, 140, 28, '#ffd34a', 'center');
    text(Acct.signedIn ? 'Your LionCoin (' + coins(Acct.coinsT()) + ') is in your OK Music wallet.' : 'Sign in to OK Music to keep what you earn.', W / 2, 184, 9, '#aaa', 'center');
    button('BACK', W / 2 - 60, 206, 120, 22, true, () => { G.state = Acct.chancesLeft() > 0 ? 'menu' : 'chances'; ui.sel = 0; });
  } else if (s === 'clear') {
    ctx.fillStyle = 'rgba(0,0,0,0.65)'; ctx.fillRect(0, 0, W, H); panel(200);
    text('LEVEL ' + (G.lv + 1) + ' CLEAN', W / 2, 74, 24, '#7dff9a', 'center'); text(LEVELS[G.lv].bossName + ' IS DOWN', W / 2, 104, 12, '#fff', 'center');
    text(Acct.signedIn ? 'Kills ' + G.kills + '   LionCoin earned +' + coins(G.earnedT) + '   Total ' + coins(Acct.coinsT()) : 'Kills ' + G.kills + '   Sign in to OK Music to earn LionCoin', W / 2, 128, 10, '#ffd34a', 'center');
    text('Weight carried: ' + G.weight, W / 2, 146, 10, '#c9a8ff', 'center');
    button('GO UP: LEVEL ' + (G.lv + 2), W / 2 - 90, 172, 180, 26, G.clearT > 1, () => startLevel(G.lv + 1));
  } else if (s === 'ending') {
    ctx.fillStyle = '#05050f'; ctx.fillRect(0, 0, W, H);
    text('THE END', W / 2, 30, 28, '#fff', 'center');
    const cap = G.endKind === 'captured'; text(cap ? 'Granmet Nwit is handed over to the helicopter.' : 'Granmet Nwit falls. The roof goes quiet.', W / 2, 74, 10, '#ccc', 'center');
    const w = G.weight; const line = w < 70 ? 'You carried it lightly. It was never nothing.' : w < 140 ? 'The weight is heavy. It will walk with you down the stairs.' : 'The weight is crushing. Every one of them was somebody.';
    wrap(line, W / 2, 98, 380, 10, '#c9a8ff', 14);
    wrap(cap ? 'You chose the law over revenge. That is the road of no return that leads somewhere.' : 'You finished it yourself. The road of no return does not give that back.', W / 2, 130, 380, 9, '#aaa', 13);
    text('Weight ' + G.weight + '   Freed ' + G.freed + (Acct.signedIn ? '   LionCoin ' + coins(Acct.coinsT()) : ''), W / 2, 176, 11, '#ffd34a', 'center');
    button('PLAY AGAIN FROM LEVEL 1', W / 2 - 110, 206, 220, 26, G.clearT > 1, () => { save.level = 1; persist(); startLevel(0); });
    text('Thanks for playing. Source: github.com/tigerlionx/wout-san-retou', W / 2, 250, 8, '#777', 'center');
  }
}
function render() {
  if (G.map && ['play', 'paused', 'dead', 'clear', 'ending'].includes(G.state)) { renderWorld(); if (G.state === 'play' || G.state === 'paused' || G.state === 'dead') drawWeapon(); if (G.state !== 'ending') drawHUD(); }
  else if (!G.map || G.state === 'menu' || G.state === 'chances' || G.state === 'tomorrow') { if (!G.map) { ctx.fillStyle = '#080814'; ctx.fillRect(0, 0, W, H); } else { renderWorld(); } }
  drawScreens();
}

/* ---------- input ---------- */
function canvasPos(ev) { const r = canvas.getBoundingClientRect(); return [(ev.clientX - r.left) * W / r.width, (ev.clientY - r.top) * H / r.height]; }
function clickUI(ev) {
  const [x, y] = canvasPos(ev);
  for (let i = ui.buttons.length - 1; i >= 0; i--) { const b = ui.buttons[i]; if (b.enabled && x >= b.x && x <= b.x + b.w && y >= b.y && y <= b.y + b.h) { ui.sel = i; b.fn(); return true; } }
  return false;
}
canvas.addEventListener('mousedown', (ev) => {
  A.init(); A.resume();
  if (G.state === 'play') { if (!locked) lock(); mouseDown = true; ev.preventDefault(); return; }
  clickUI(ev);
});
window.addEventListener('mouseup', () => { mouseDown = false; });
canvas.addEventListener('wheel', (ev) => { if (G.state !== 'play') return; ev.preventDefault(); const i = WORDER.indexOf(G.player.wpn); selectWeapon(WORDER[(i + (ev.deltaY > 0 ? 1 : WORDER.length - 1)) % WORDER.length]); }, { passive: false });
document.addEventListener('pointerlockchange', () => { locked = document.pointerLockElement === canvas; if (!locked && G.state === 'play') { G.state = 'paused'; mouseDown = false; } });
document.addEventListener('mousemove', (ev) => { if (locked && G.state === 'play') { const p = G.player; p.a += ev.movementX * 0.0032; p.pitch = clamp(p.pitch - ev.movementY * 0.45, -70, 70); } });
window.addEventListener('keydown', (ev) => {
  if (['Space', 'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'].includes(ev.code)) ev.preventDefault();
  keys[ev.code] = true; A.init(); A.resume();
  if (ev.code === 'KeyM') { A.setMuted(!A.muted); showMsg(A.muted ? 'Sound off' : 'Sound on', 1); }
  if (G.state === 'play') {
    const wi = ['Digit1', 'Digit2', 'Digit3', 'Digit4', 'Digit5', 'Digit6'].indexOf(ev.code); if (wi >= 0) selectWeapon(WORDER[wi]);
    if (ev.code === 'KeyB') { const p = G.player; p.akAuto = !p.akAuto; showMsg('AK-47: ' + (p.akAuto ? 'AUTOMATIC' : 'SINGLE SHOT'), 1.4); A.sfx('click'); }
    if (ev.code === 'KeyF' && !ev.repeat && G.player.cd <= 0 && G.player.crouch < 0.5) stab();
    if (ev.code === 'KeyG' && !ev.repeat && G.player.cd <= 0) throwNade();
    if (ev.code === 'KeyR') startReload(); if (ev.code === 'KeyH') drink(); if (ev.code === 'KeyE') useKey();
    if (ev.code === 'Escape' || ev.code === 'KeyP') { G.state = 'paused'; if (document.exitPointerLock) document.exitPointerLock(); }
  } else if (ev.code === 'Enter' || ev.code === 'Space') {
    if (G.state === 'intro') { if (G.introT > 0.4) beginPlay(); }
    else if (G.state === 'paused') { G.state = 'play'; lock(); }
    else { const b = ui.buttons[ui.sel] || ui.buttons.find((q) => q.enabled); if (b && b.enabled) b.fn(); }
  } else if (ev.code === 'ArrowDown' || ev.code === 'ArrowUp') {
    const n = ui.buttons.length; if (n) { ui.sel = (ui.sel + (ev.code === 'ArrowDown' ? 1 : n - 1)) % n; }
  }
});
window.addEventListener('keyup', (ev) => { keys[ev.code] = false; });
window.addEventListener('blur', () => { Object.keys(keys).forEach((k) => { keys[k] = false; }); if (G.state === 'play') { G.state = 'paused'; mouseDown = false; } });
document.addEventListener('visibilitychange', () => { if (document.hidden && G.state === 'play') G.state = 'paused'; });

/* ---------- touch controls (phones and tablets) ---------- */
const TOUCH = { on: false, fire: false, mx: 0, my: 0, looking: false, reset: null, sync: null };
const TCSS = `
body.touch { overscroll-behavior: none; -webkit-user-select: none; user-select: none; -webkit-touch-callout: none; -webkit-tap-highlight-color: transparent; }
body.touch canvas { touch-action: none; }
body.touch #stage { padding: 0; max-width: none; }
body.touch #wrap { border-radius: 0; }
@media (orientation: landscape) {
  body.touch { justify-content: center; }
  body.touch header, body.touch .bar, body.touch .note { display: none !important; }
  body.touch #wrap { width: min(100vw, calc(100vh * 1.6)); width: min(100vw, calc(100dvh * 1.6)); border: 0; }
}
@media (orientation: portrait) { body.touch #wrap { width: 100vw; border-left: 0; border-right: 0; } body.touch.playing header, body.touch.playing .bar, body.touch.playing .note { display: none; } body.touch #touch #tw { bottom: 170px; } }
#touch { display: none; position: fixed; left: 0; top: 0; right: 0; bottom: 0; z-index: 30; pointer-events: none; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; }
body.touch.playing #touch { display: block; }
#touch .tz { position: absolute; top: 0; bottom: 0; pointer-events: auto; touch-action: none; }
#tzl { left: 0; width: 45%; } #tzr { right: 0; width: 55%; }
#stick { position: fixed; width: 110px; height: 110px; margin: -55px 0 0 -55px; border-radius: 50%; border: 2px solid rgba(255,255,255,0.35); background: rgba(255,255,255,0.08); display: none; pointer-events: none; }
#stick i { position: absolute; left: 50%; top: 50%; width: 46px; height: 46px; margin: -23px 0 0 -23px; border-radius: 50%; background: rgba(255,255,255,0.4); }
#touch .tb { position: absolute; pointer-events: auto; touch-action: none; margin: 0; padding: 0; width: 50px; height: 50px; border-radius: 50%; border: 2px solid rgba(255,255,255,0.55); background: rgba(40,30,90,0.6); color: #fff; font-weight: 800; font-size: 12px; line-height: 1.1; display: flex; align-items: center; justify-content: center; text-align: center; }
#touch .tb.big { width: 84px; height: 84px; background: rgba(200,40,40,0.6); font-size: 15px; }
#touch .tb.mid { width: 66px; height: 66px; background: rgba(30,120,70,0.6); }
#touch .tb.on, #touch .tb.sel { background: rgba(123,92,246,0.9); border-color: #fff; }
#tw { position: absolute; left: 50%; bottom: 6px; transform: translateX(-50%); display: flex; gap: 4px; pointer-events: none; }
#tw .tb { position: static; width: 38px; height: 34px; border-radius: 8px; font-size: 9px; }
#rot { display: none; position: absolute; left: 0; right: 0; bottom: 212px; text-align: center; font-size: 12px; font-weight: 700; color: #ffd34a; text-shadow: 0 1px 2px #000; pointer-events: none; }
@media (orientation: portrait) { #rot { display: block; } }
`;
function initTouch() {
  if (TOUCH.on) return; TOUCH.on = true;
  const st = document.createElement('style'); st.textContent = TCSS; document.head.appendChild(st);
  document.body.classList.add('touch');
  const root = document.createElement('div'); root.id = 'touch';
  root.innerHTML = '<div class="tz" id="tzl"></div><div class="tz" id="tzr"></div><div id="stick"><i></i></div><div id="rot">Turn your phone sideways for a bigger screen</div><div id="tw"></div>';
  document.body.appendChild(root);
  root.addEventListener('contextmenu', (ev) => ev.preventDefault());
  const zl = root.querySelector('#tzl'), zr = root.querySelector('#tzr'), stickEl = root.querySelector('#stick'), thumb = stickEl.firstChild, tw = root.querySelector('#tw');
  let sId = null, sx = 0, sy = 0, lId = null, lx = 0, ly = 0, fireN = 0; const opt = { passive: false };
  zl.addEventListener('touchstart', (ev) => { ev.preventDefault(); if (sId !== null) return; const t = ev.changedTouches[0]; sId = t.identifier; sx = t.clientX; sy = t.clientY; stickEl.style.display = 'block'; stickEl.style.left = sx + 'px'; stickEl.style.top = sy + 'px'; thumb.style.transform = ''; }, opt);
  zl.addEventListener('touchmove', (ev) => { ev.preventDefault(); for (const t of ev.changedTouches) if (t.identifier === sId) { const dx = t.clientX - sx, dy = t.clientY - sy, d = Math.hypot(dx, dy) || 1, R = 50, k = Math.min(1, d / R); TOUCH.mx = dx / d * k; TOUCH.my = dy / d * k; thumb.style.transform = 'translate(' + (dx / d * Math.min(d, R)) + 'px,' + (dy / d * Math.min(d, R)) + 'px)'; } }, opt);
  const sEnd = (ev) => { ev.preventDefault(); for (const t of ev.changedTouches) if (t.identifier === sId) { sId = null; TOUCH.mx = 0; TOUCH.my = 0; stickEl.style.display = 'none'; } };
  zl.addEventListener('touchend', sEnd, opt); zl.addEventListener('touchcancel', sEnd, opt);
  zr.addEventListener('touchstart', (ev) => { ev.preventDefault(); if (lId !== null) return; const t = ev.changedTouches[0]; lId = t.identifier; lx = t.clientX; ly = t.clientY; TOUCH.looking = true; }, opt);
  zr.addEventListener('touchmove', (ev) => { ev.preventDefault(); for (const t of ev.changedTouches) if (t.identifier === lId) { if (G.state === 'play') { const p = G.player; p.a += (t.clientX - lx) * 0.0085; p.pitch = clamp(p.pitch - (t.clientY - ly) * 0.55, -70, 70); } lx = t.clientX; ly = t.clientY; } }, opt);
  const lEnd = (ev) => { ev.preventDefault(); for (const t of ev.changedTouches) if (t.identifier === lId) { lId = null; TOUCH.looking = false; } };
  zr.addEventListener('touchend', lEnd, opt); zr.addEventListener('touchcancel', lEnd, opt);
  const btn = (parent, cls, label, style, down, up) => {
    const b = document.createElement('button'); b.type = 'button'; b.className = 'tb ' + cls; b.innerHTML = label; Object.assign(b.style, style);
    const on = (ev) => { ev.preventDefault(); ev.stopPropagation(); b.classList.add('on'); if (down) down(); };
    const off = (ev) => { ev.preventDefault(); b.classList.remove('on'); if (up) up(); };
    b.addEventListener('touchstart', on, opt); b.addEventListener('touchend', off, opt); b.addEventListener('touchcancel', off, opt);
    parent.appendChild(b); return b;
  };
  const fireDown = () => { fireN++; TOUCH.fire = true; }, fireUp = () => { fireN = Math.max(0, fireN - 1); TOUCH.fire = fireN > 0; };
  const ready = () => G.state === 'play' && G.player.cd <= 0;
  btn(root, 'big', 'FIRE', { right: '14px', bottom: '14px' }, fireDown, fireUp);
  btn(root, 'big', 'FIRE', { left: '14px', bottom: '14px', width: '70px', height: '70px' }, fireDown, fireUp);
  btn(root, 'mid', 'HIDE', { right: '106px', bottom: '18px' }, () => { keys.KeyZ = true; }, () => { keys.KeyZ = false; });
  btn(root, '', 'R', { right: '14px', bottom: '106px' }, () => { if (G.state === 'play') startReload(); });
  btn(root, '', 'GRN', { right: '70px', bottom: '106px' }, () => { if (ready()) throwNade(); });
  btn(root, '', 'KNF', { right: '126px', bottom: '106px' }, () => { if (ready() && G.player.crouch < 0.5) stab(); });
  btn(root, '', 'E', { left: '14px', bottom: '92px' }, () => { if (G.state === 'play') useKey(); });
  btn(root, '', 'H', { left: '70px', bottom: '92px' }, () => { if (G.state === 'play') drink(); });
  const autoBtn = btn(root, '', 'AUTO', { left: '126px', bottom: '92px' }, () => { if (G.state === 'play') { const p = G.player; p.akAuto = !p.akAuto; showMsg('AK-47: ' + (p.akAuto ? 'AUTOMATIC' : 'SINGLE SHOT'), 1.4); } });
  btn(root, '', 'II', { right: '4px', top: '4px', width: '34px', height: '34px' }, () => { if (G.state === 'play') { G.state = 'paused'; mouseDown = false; } });
  const ab = { knife: 'KNF', glock: 'GLK', magnum: 'MAG', shotgun: 'SHG', ak: 'AK', grenade: 'GRN' };
  WORDER.forEach((id, i) => { const b = btn(tw, '', (i + 1) + '<br>' + ab[id], {}, () => selectWeapon(id)); b.dataset.w = id; });
  TOUCH.reset = () => { sId = null; lId = null; fireN = 0; TOUCH.fire = false; TOUCH.mx = 0; TOUCH.my = 0; TOUCH.looking = false; keys.KeyZ = false; stickEl.style.display = 'none'; };
  let lastKey = '';
  TOUCH.sync = () => { const p = G.player; if (!p) return; const k = p.wpn + (p.akAuto ? 'a' : 's'); if (k === lastKey) return; lastKey = k; tw.querySelectorAll('button').forEach((b) => b.classList.toggle('sel', b.dataset.w === p.wpn)); autoBtn.textContent = p.akAuto ? 'AUTO' : 'SINGLE'; };
  window.addEventListener('touchend', () => { A.init(); A.resume(); }, { passive: true });
}
let wasPlay = false;
function syncPlaying() { const pl = G.state === 'play'; if (pl !== wasPlay) { wasPlay = pl; document.body.classList.toggle('playing', pl); if (!pl && TOUCH.reset) TOUCH.reset(); } if (pl && TOUCH.sync) TOUCH.sync(); }
if (window.matchMedia && matchMedia('(pointer: coarse)').matches) initTouch(); else window.addEventListener('touchstart', initTouch, { once: true, passive: true });

/* ---------- loop ---------- */
let last = performance.now();
function frame(now) {
  const dt = Math.min(0.05, (now - last) / 1000); last = now;
  try { update(dt); render(); syncPlaying(); } catch (e) { console.error(e); }
  requestAnimationFrame(frame);
}
loadSave();
Acct.init();
const q = new URLSearchParams(location.search);
if (q.get('level')) { A.init(); save.level = clamp(+q.get('level'), 1, 7); }
window.WSR = { G, save, Acct, LEVELS, T, WP, spawnEnt, startLevel, beginPlay, damageEnt, killEnt, update, render, fire, A, M, keys, FIG, covered, startInv, spawnBazooka, launchRocket, throwNade, stab, selectWeapon, playerHurt, TOUCH, initTouch };
requestAnimationFrame(frame);
})();
