"""Build the game's audio (needs ffmpeg and numpy: pip install numpy).

Usage: WSR_AUDIO_SRC=/path/to/sources python3 tools/build_audio.py [sfx] [synth] [music]

Third-party sources (kept out of git): gunshots, knife, ambience, the voodoo drum track.
  -> assets/audio/local/
Our own synthesized effects (safe to commit):
  -> assets/audio/synth/
"""
import os, subprocess, sys, wave
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
# A folder holding copies of the source files under these short names:
#   ak.mp3 pistol.mp3 hmg.mp3 knife.mp3 pack.mp3 fight.mp3 voodoo.mp3
SRC = os.environ.get("WSR_AUDIO_SRC", os.path.join(HERE, "audio_src"))
OUT = os.path.join(HERE, "..", "assets", "audio")
LOCAL = os.path.join(OUT, "local")
SYNTH = os.path.join(OUT, "synth")
os.makedirs(LOCAL, exist_ok=True)
os.makedirs(SYNTH, exist_ok=True)
SR = 44100
rng = np.random.default_rng(7)


# ------------------------------------------------------------------ io
def ffload(path, ss=None, t=None, af=None, ch=2):
    cmd = ["ffmpeg", "-v", "error"]
    if ss is not None:
        cmd += ["-ss", str(ss)]
    if t is not None:
        cmd += ["-t", str(t)]
    cmd += ["-i", path]
    chain = "aresample=%d" % SR
    if af:
        chain += "," + af
    cmd += ["-af", chain, "-ac", str(ch), "-ar", str(SR), "-f", "f32le", "-"]
    raw = subprocess.run(cmd, capture_output=True).stdout
    x = np.frombuffer(raw, dtype=np.float32).reshape(-1, ch).copy()
    return x


def save(path, x, peak=0.89):
    x = np.asarray(x, dtype=np.float64)
    if x.ndim == 1:
        x = x[:, None]
    m = np.abs(x).max()
    if m > 0:
        x = x * (peak / m)
    pcm = (np.clip(x, -1, 1) * 32767).astype("<i2")
    with wave.open(path, "wb") as w:
        w.setnchannels(pcm.shape[1])
        w.setsampwidth(2)
        w.setframerate(SR)
        w.writeframes(pcm.tobytes())
    print("wrote", os.path.relpath(path, OUT), f"{len(x)/SR:.2f}s", f"{os.path.getsize(path)/1e6:.2f}MB")


def mono(x):
    return x.mean(axis=1)


# -------------------------------------------------------------- dsp helpers
def fft_filter(x, resp):
    n = len(x)
    f = np.fft.rfftfreq(n, 1 / SR)
    out = np.empty_like(x)
    H = resp(f)
    if x.ndim == 1:
        return np.fft.irfft(np.fft.rfft(x) * H, n)
    for c in range(x.shape[1]):
        out[:, c] = np.fft.irfft(np.fft.rfft(x[:, c]) * H, n)
    return out


def lowpass(x, fc, order=4):
    return fft_filter(x, lambda f: 1 / np.sqrt(1 + (f / fc) ** (2 * order)))


def highpass(x, fc, order=4):
    return fft_filter(x, lambda f: 1 / np.sqrt(1 + (fc / np.maximum(f, 1e-3)) ** (2 * order)))


def bandpass(x, lo, hi):
    return highpass(lowpass(x, hi), lo)


def reverb_ir(rt60, tilt=3500.0, length=None):
    n = int((length or rt60 * 1.2) * SR)
    t = np.arange(n) / SR
    ir = rng.standard_normal(n) * np.exp(-6.9 * t / rt60)
    ir = lowpass(ir, tilt, 2)
    ir[0] = 0
    return ir / np.abs(ir).max()


def convolve(x, ir):
    n = len(x) + len(ir)
    nfft = 1 << (n - 1).bit_length()
    IR = np.fft.rfft(ir, nfft)
    if x.ndim == 1:
        return np.fft.irfft(np.fft.rfft(x, nfft) * IR, nfft)[: len(x) + len(ir) - 1]
    return np.stack([np.fft.irfft(np.fft.rfft(x[:, c], nfft) * IR, nfft)[: len(x) + len(ir) - 1] for c in range(x.shape[1])], axis=1)


def reverb(x, rt60, wet=0.3, tilt=3500.0):
    ir = reverb_ir(rt60, tilt)
    y = convolve(x, ir)
    y = y[: len(x)] if len(y) >= len(x) else np.pad(y, ((0, len(x) - len(y)),) + ((0, 0),) * (x.ndim - 1))
    ref = np.abs(x).max() + 1e-9
    y = y / (np.abs(y).max() + 1e-9) * ref
    return x * (1 - wet) + y * wet


def softclip(x, drive=2.0):
    return np.tanh(x * drive) / np.tanh(drive)


def fade_in(x, ms=2.0):
    n = int(ms / 1000 * SR)
    x = x.copy()
    x[:n] *= np.linspace(0, 1, n)
    return x


def onset_refine(x, t, back=0.02, ahead=0.03):
    """Move t to the real start of the transient near it."""
    s, e = int((t - back) * SR), int((t + ahead) * SR)
    seg = np.abs(x[max(s, 0):e])
    pk = seg.max()
    i = int(np.argmax(seg > 0.25 * pk))
    return (max(s, 0) + i) / SR


def next_onset(x, t, min_gap=0.05, rise_db=12):
    """Time of the next transient after t (None if there is none within a second)."""
    hop = int(0.002 * SR)
    start = int((t + min_gap) * SR)
    seg = np.abs(x[start : start + SR])
    n = len(seg) // hop
    if n < 3:
        return None
    e = 20 * np.log10(seg[: n * hop].reshape(n, hop).max(axis=1) + 1e-6)
    ref = 20 * np.log10(np.abs(x[int(t * SR) : int((t + 0.03) * SR)]).max() + 1e-6)
    for i in range(2, n):
        if e[i] > ref - 14 and e[i] - e[i - 2] > rise_db:
            return (start + i * hop) / SR
    return None


def clip_at(x, t, length, pre=0.004, fade=0.012):
    s = int((t - pre) * SR)
    seg = x[max(s, 0): max(s, 0) + int(length * SR)].copy()
    f = min(int(fade * SR), len(seg) // 2)
    seg[-f:] *= np.linspace(1, 0, f) if seg.ndim == 1 else np.linspace(1, 0, f)[:, None]
    fi = int(0.0015 * SR)   # a 1.5 ms fade-in so a cut mid-waveform cannot click
    seg[:fi] *= np.linspace(0, 1, fi) if seg.ndim == 1 else np.linspace(0, 1, fi)[:, None]
    return seg


# ===================================================================== SFX
def build_third_party_sfx():
    ak = mono(ffload(f"{SRC}/ak.mp3"))
    for i, t in enumerate([0.99, 1.11, 1.39, 1.63, 1.92], 1):
        save(f"{LOCAL}/ak_{i}.wav", clip_at(ak, onset_refine(ak, t), 0.118))
    pistol = mono(ffload(f"{SRC}/pistol.mp3"))
    for i, t in enumerate([1.05, 1.35, 1.71, 2.22], 1):
        save(f"{LOCAL}/pistol_{i}.wav", clip_at(pistol, onset_refine(pistol, t), 0.15))
    save(f"{LOCAL}/pistol_tail.wav", clip_at(pistol, onset_refine(pistol, 3.48), 0.72, fade=0.08))
    hmg = mono(ffload(f"{SRC}/hmg.mp3"))
    for i, t in enumerate([0.88, 1.45, 1.88, 2.23], 1):
        save(f"{LOCAL}/m50_{i}.wav", clip_at(hmg, onset_refine(hmg, t), 0.14))
    kn = mono(ffload(f"{SRC}/knife.mp3"))
    save(f"{LOCAL}/knife_hit.wav", clip_at(kn, 1.05, 0.62, fade=0.06))
    save(f"{LOCAL}/knife_swish.wav", clip_at(kn, 1.03, 0.2, fade=0.05))
    # The heaviest shot in the pack (big low-frequency crack), given a long tail: the sniper rifle.
    pack = mono(ffload(f"{SRC}/pack.mp3"))
    t0 = onset_refine(pack, 26.82)
    nxt = next_onset(pack, t0)
    heavy = clip_at(pack, t0, min(0.36, (nxt - t0 - 0.01) if nxt else 0.36), fade=0.03)
    heavy = np.pad(heavy, (0, int(0.8 * SR)))
    heavy = reverb(heavy, 0.9, 0.35, 2500)
    save(f"{LOCAL}/sniper_shot.wav", heavy)
    # Distant gunfight bed, looped with an equal-power crossfade.
    fight = ffload(f"{SRC}/fight.mp3", ss=8, t=44)
    fight = lowpass(fight, 3200, 2) * 0.8
    xf = int(2.0 * SR)
    a, b = fight[:-xf], fight[-xf:]
    w = np.linspace(0, np.pi / 2, xf)[:, None]
    mix = b * np.cos(w) + fight[:xf] * np.sin(w)
    loop = np.concatenate([mix, fight[xf:-xf]]) if len(fight) > 2 * xf else fight
    save(f"{LOCAL}/fight_ambience.wav", loop, peak=0.7)


def synth_sfx():
    t = lambda d: np.arange(int(d * SR)) / SR
    # --- explosion: a thump, a roaring noise sliding down, crackle and debris
    d = 3.0
    tt = t(d)
    noise = rng.standard_normal(len(tt))
    out = np.zeros(len(tt))
    # time-varying one-pole low-pass: cutoff slides from 7 kHz to 180 Hz
    y = 0.0
    for i in range(len(tt)):
        fc = 180 + 6800 * np.exp(-tt[i] / 0.35)
        a = 1 - np.exp(-2 * np.pi * fc / SR)
        y += a * (noise[i] - y)
        out[i] = y
    roar = out * np.exp(-tt / 0.9) * (1 - np.exp(-tt / 0.004))
    f = 85 * np.exp(-tt / 0.18) + 24
    thump = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-tt / 0.45) * 1.4
    crack = np.zeros(len(tt))
    for _ in range(260):
        p = rng.exponential(0.5)
        if p < d - 0.05:
            i = int(p * SR)
            crack[i : i + 40] += rng.standard_normal(40) * rng.uniform(0.1, 0.6) * np.exp(-p / 0.6)
    boom = softclip(thump + roar * 2.2 + highpass(crack, 900), 1.4)
    boom = reverb(boom, 1.4, 0.25, 3000)
    boom *= np.minimum(1, (d - tt) / 0.4)
    save(f"{SYNTH}/explosion.wav", boom)

    # --- reload: two metallic clicks with a slide between
    d = 0.7
    tt = t(d)
    r = np.zeros(len(tt))
    def click(at, f1, f2, gain):
        i = int(at * SR)
        n = len(tt) - i
        tn = np.arange(n) / SR
        v = (np.sin(2 * np.pi * f1 * tn) + 0.7 * np.sin(2 * np.pi * f2 * tn)) * np.exp(-tn / 0.012)
        v += highpass(rng.standard_normal(n), 3000) * 0.5 * np.exp(-tn / 0.004)
        r[i:] += v * gain
    click(0.0, 2100, 3400, 0.8)
    click(0.33, 1700, 2900, 1.0)
    slide = bandpass(rng.standard_normal(len(tt)), 800, 4500) * np.exp(-((tt - 0.17) / 0.05) ** 2) * 0.25
    save(f"{SYNTH}/reload.wav", fade_in(r + slide))

    # --- Nwit's cue: two soft knocks, "plip, plip"
    d = 0.5
    tt = t(d)
    p = np.zeros(len(tt))
    for at in (0.0, 0.17):
        i = int(at * SR)
        n = len(tt) - i
        tn = np.arange(n) / SR
        fr = 700 * np.exp(-tn / 0.03) + 380
        p[i:] += np.sin(2 * np.pi * np.cumsum(fr) / SR) * np.exp(-tn / 0.05)
    save(f"{SYNTH}/plip.wav", fade_in(reverb(p, 0.5, 0.3, 4000)))

    # --- Ro hit: a short dull thud
    d = 0.35
    tt = t(d)
    f = 130 * np.exp(-tt / 0.05) + 55
    thud = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-tt / 0.09)
    thud += lowpass(rng.standard_normal(len(tt)), 700, 2) * np.exp(-tt / 0.04) * 0.6
    # a mid-range slap so it carries on small speakers
    thud += bandpass(rng.standard_normal(len(tt)), 300, 2800) * np.exp(-tt / 0.022) * 1.1
    save(f"{SYNTH}/hit.wav", fade_in(thud))

    # --- helicopter loop: rotor chop over a turbine rumble (exact whole cycles so it loops)
    d = 4.0
    tt = t(d)
    chop_hz = 11.0
    chop = (0.5 + 0.5 * np.abs(np.sin(np.pi * chop_hz * tt))) ** 3
    body = bandpass(rng.standard_normal(len(tt)), 180, 2400) * chop
    rumble = (np.sin(2 * np.pi * 70 * tt) + 0.5 * np.sin(2 * np.pi * 140 * tt)) * 0.5 * (0.7 + 0.3 * chop)
    turbine = highpass(rng.standard_normal(len(tt)), 3500) * 0.08
    heli = body * 1.2 + rumble + turbine
    xf = int(0.25 * SR)
    w = np.linspace(0, np.pi / 2, xf)
    heli[:xf] = heli[:xf] * np.sin(w) + heli[-xf:] * np.cos(w)
    save(f"{SYNTH}/heli_loop.wav", heli[:-xf], peak=0.8)


# ==================================================================== MUSIC
def fold_beat(x_low, period_s, sr_env=200):
    """Onset-envelope folded at a beat period; returns (score, phase_s)."""
    e = np.abs(x_low)
    hop = SR // sr_env
    e = e[: len(e) // hop * hop].reshape(-1, hop).mean(axis=1)
    d = np.maximum(0, np.diff(e))
    t = np.arange(len(d)) / sr_env
    bins = 40
    idx = ((t % period_s) / period_s * bins).astype(int) % bins
    hist = np.bincount(idx, weights=d, minlength=bins)
    return hist.max() / (hist.mean() + 1e-9), int(np.argmax(hist)) / bins * period_s


def estimate_grid(x_mono, lo=0.48, hi=0.62, band=None):
    low = lowpass(x_mono, 220, 2) if band is None else bandpass(x_mono, band[0], band[1])
    best = (0, 0.55, 0)
    for p in np.arange(lo, hi, 0.0005):
        sc, ph = fold_beat(low, p)
        if sc > best[0]:
            best = (sc, p, ph)
    return best  # score, beat period, phase


def stretched_segment(start, bars, target_bpm, pitch):  # noqa
    """Cut `bars` bars (4/4) from the voodoo track at its own grid and resample to target_bpm."""
    path = f"{SRC}/voodoo.mp3"
    probe = mono(ffload(path, ss=start, t=bars * 2.4 + 6))
    score, beat, phase = estimate_grid(probe)
    seg_len = bars * 4 * beat
    # start on the first strong beat after `phase`
    s0 = start + phase
    target_len = bars * 4 * 60.0 / target_bpm
    speed = seg_len / target_len          # < 1 = slower
    af = f"asetrate={SR*pitch},aresample={SR},atempo={speed/pitch}"
    x = ffload(path, ss=s0, t=seg_len + 0.05, af=af)
    n = int(target_len * SR)
    x = x[:n] if len(x) >= n else np.pad(x, ((0, n - len(x)), (0, 0)))
    print(f"  segment @{start:.0f}s grid beat={beat:.4f}s ({60/beat:.1f} bpm) score={score:.1f} phase={phase:.3f}s -> {target_bpm} bpm, {target_len:.1f}s")
    return x, target_bpm


def beat_pulse(n, target_bpm, every_beats, pattern):
    """Heartbeat-like pulses locked to the grid. pattern = [(beat_offset, gain)]."""
    beat = 60.0 / target_bpm * SR
    out = np.zeros(n)
    b = 0
    while b * beat < n:
        for off, g in pattern:
            i = int((b + off) * beat)
            if i + 6000 < n:
                tn = np.arange(6000) / SR
                f = 62 * np.exp(-tn / 0.03) + 38
                out[i : i + 6000] += np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-tn / 0.07) * g
        b += every_beats
    return out


def drone(n, root=55.0):
    tt = np.arange(n) / SR
    lfo = 0.6 + 0.4 * np.sin(2 * np.pi * 0.07 * tt)
    d = (np.sin(2 * np.pi * root * tt) * 0.8 + np.sin(2 * np.pi * root * 0.5 * tt) * 0.6
         + np.sin(2 * np.pi * root * 1.0595 * tt) * 0.25 * (0.5 + 0.5 * np.sin(2 * np.pi * 0.05 * tt))
         + np.sin(2 * np.pi * root * 1.5 * tt) * 0.2)
    wind = bandpass(rng.standard_normal(n), 250, 900) * 0.08 * (0.5 + 0.5 * np.sin(2 * np.pi * 0.11 * tt + 1.0))
    return d * lfo * 0.35 + wind


def dissonant_pad(n, base=220.0):
    tt = np.arange(n) / SR
    pad = np.zeros(n)
    for ratio in (1.0, 1.0595, 1.498, 1.587):   # root, minor second, fifth, minor sixth: unsettled
        saw = 2 * ((base * ratio * tt) % 1.0) - 1
        pad += saw * 0.25
    pad = lowpass(pad, 900, 3)
    trem = 0.6 + 0.4 * np.sin(2 * np.pi * 5.3 * tt)
    return pad * trem * 0.18


def hat_ticks(n, bpm, pattern16):
    out = np.zeros(n)
    step = 60.0 / bpm / 4 * SR
    k = 0
    while k * step < n:
        g = pattern16[k % len(pattern16)]
        if g:
            i = int(k * step)
            m = min(2400, n - i)
            tn = np.arange(m) / SR
            out[i : i + m] += highpass(rng.standard_normal(m), 6000, 2) * np.exp(-tn / 0.012) * g
        k += 1
    return out


def make_music():
    BPM = 78
    pitch = 0.89  # about two semitones down: darker
    print("music: building segments (drum and bass only, no vocals)")
    plan = [(0.0, 8), (48.0, 6)]
    segs = []
    for start, bars in plan:
        x, _ = stretched_segment(start, bars, BPM, pitch)
        segs.append(x)
    xf = int(0.25 * SR)
    def join(a, b):
        w = np.linspace(0, np.pi / 2, xf)[:, None]
        mid = a[-xf:] * np.cos(w) + b[:xf] * np.sin(w)
        return np.concatenate([a[:-xf], mid, b[xf:]])
    base = join(segs[0], segs[1])
    # trim to a whole number of heartbeat periods so the synthetic layers loop cleanly
    HB = 1.8
    n_beats = int(len(base) / SR // HB)
    n = int(n_beats * HB * SR) + xf
    base = base[:n]
    w = np.linspace(0, np.pi / 2, xf)[:, None]
    base[:xf] = base[:xf] * np.sin(w) + base[-xf:] * np.cos(w)
    base = base[:-xf]
    n = len(base)
    print(f"  loop length {n/SR:.2f}s ({n_beats} heartbeats of {HB}s)")

    # does the stretched drumming have a usable grid? (live drumming often does not)
    sc, per, ph = estimate_grid(mono(base), 0.6, 1.0, band=(400, 5000))
    print(f"  drum grid in the loop: period {per:.3f}s ({60/per:.0f} bpm) score {sc:.2f}")
    grid_ok = sc >= 3.0

    def pulses(period, pattern):
        out = np.zeros(n)
        k = 0
        while k * period * SR < n:
            for off, g in pattern:
                i = int((k * period + off) * SR)
                if i + 8000 < n:
                    tn = np.arange(8000) / SR
                    f = 62 * np.exp(-tn / 0.03) + 38
                    out[i : i + 8000] += np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-tn / 0.08) * g
            k += 1
        return out

    st = lambda v: np.stack([v, v], axis=1)
    d = drone(n)
    hb_slow = pulses(HB, [(0.0, 1.0), (0.42, 0.6)])        # lub-dub
    hb_fast = pulses(HB / 2, [(0.0, 1.0), (0.24, 0.55)])

    calm = reverb(lowpass(base, 420, 3) * 1.1, 2.2, 0.45, 2200)
    # Small speakers cannot play below ~100 Hz: give the calm layer mid-range body too.
    hollow = reverb(bandpass(base, 160, 1600), 2.6, 0.6, 2600)            # the drums, hollow and distant
    d_h = highpass(softclip(d * 4.0, 2.0), 110, 2)                           # the drone's overtones
    tick = highpass(lowpass(hb_slow, 900, 2) * 2.0, 140, 2)                  # the heartbeat's knock
    calm_mix = softclip(
        calm * 0.7 + hollow * 0.9 + st(d) * 0.7 + st(d_h) * 0.5 + st(hb_slow) * 0.5 + st(tick) * 0.35,
        1.2,
    )

    tense = base + highpass(base, 1200, 2) * 0.8
    tense = reverb(softclip(tense * 1.4, 1.6), 1.0, 0.2, 3500)
    pad = dissonant_pad(n)
    tense_mix = tense * 0.85 + np.stack([pad, np.roll(pad, 441)], axis=1) + st(hb_fast) * 0.7 + st(d) * 0.6
    if grid_ok:
        hats = hat_ticks(n, 60.0 / per, [1, 0, 0.5, 0, 1, 0, 0.5, 0.3])
        tense_mix = tense_mix + np.stack([hats, np.roll(hats, 220)], axis=1) * 0.3
    tense_mix = softclip(tense_mix, 1.1)

    save(f"{LOCAL}/music_calm.wav", calm_mix, peak=0.7)
    save(f"{LOCAL}/music_tense.wav", tense_mix, peak=0.78)

    ramp = lambda x, a, b: x * np.linspace(a, b, len(x))[:, None]
    sec = [calm_mix * 0.9,
           calm_mix * 0.9 + ramp(tense_mix, 0.0, 1.0) * 0.8,
           tense_mix,
           ramp(tense_mix, 1.0, 0.0) + ramp(calm_mix, 0.3, 1.0)]
    show = sec[0]
    for s_ in sec[1:]:
        show = join(show, s_)
    fade = int(4 * SR)
    show[-fade:] *= np.linspace(1, 0, fade)[:, None]
    save(f"{LOCAL}/suspense_showcase.wav", show, peak=0.8)


if __name__ == "__main__":
    what = sys.argv[1:] or ["sfx", "synth", "music"]
    if "sfx" in what:
        build_third_party_sfx()
    if "synth" in what:
        synth_sfx()
    if "music" in what:
        make_music()
