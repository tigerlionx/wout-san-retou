"""Render a busy firefight with the game's real gains and check headroom and balance."""
import os, sys, numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from meter import read, aw, db, A

SR = 44100
GAIN = {"ak": (0.48, 0.30), "pistol": (0.48, 0.25), "m50": (0.52, 0.30), "knife_hit": (0.70, 0.05),
        "sniper_shot": (0.72, 0.70), "explosion": (0.78, 1.0), "hit": (0.70, 0.40)}
MUSIC = 0.78
TENSE_TRIM = 0.7


def load(name):
    d = "synth" if name in ("explosion", "hit") else "local"
    x, sr = read(os.path.join(A, d, name + ".wav"))
    return x.mean(axis=1)


def render(seconds, events, intensity, label):
    n = int(seconds * SR)
    sfx = np.zeros(n)
    duck = np.zeros(n)
    for (t, name, vol) in events:
        x = load(name if not name.startswith(("ak", "pistol", "m50")) else name)
        key = name.rsplit("_", 1)[0] if name[-1].isdigit() else name
        g, dk = GAIN[key]
        i = int(t * SR)
        seg = x[: n - i] * g * vol
        sfx[i : i + len(seg)] += seg
        # a ducking impulse that releases at 2.4 per second
        k = np.arange(n - i) / SR
        duck[i:] = np.maximum(duck[i:], np.maximum(0, dk * vol - 2.4 * k))
    calm, _ = read(os.path.join(A, "local", "music_calm.wav"))
    tense, _ = read(os.path.join(A, "local", "music_tense.wav"))
    reps = int(np.ceil(n / len(calm)))
    calm = np.tile(calm, (reps, 1))[:n]
    tense = np.tile(tense, (reps, 1))[:n]
    i_t = intensity(np.arange(n) / SR)
    a = i_t * np.pi / 2
    mus = (calm * (0.4 + 0.6 * np.cos(a))[:, None] + tense * (np.sin(a) * 0.9 * TENSE_TRIM)[:, None]) * (MUSIC * (1 - 0.5 * duck))[:, None]
    mix = mus + sfx[:, None]
    clipped = (np.abs(mix) > 1.0).sum() / mix.size
    print(f"[{label}] peak={np.abs(mix).max():.2f}  clipped samples={clipped*100:.4f}%  music-only peak={np.abs(mus).max():.2f}")
    return mix, mus, sfx


def loud(x, a, b):
    seg = x[int(a * SR) : int(b * SR)]
    y = aw(seg if seg.ndim == 2 else seg[:, None], SR)
    return db(np.sqrt((y**2).mean()))


if __name__ == "__main__":
    ev = []
    ev += [(2.0 + k / 15.0, f"m50_{1 + k % 4}", 1.0) for k in range(int(3 * 15))]            # Ro's M50, 3 s
    ev += [(2.0 + k / 4.5, f"ak_{1 + k % 5}", 0.6) for k in range(int(6 * 4.5))]              # the boss's rifle, 6 s
    ev += [(1.0 + k * 1.0, f"pistol_{1 + k % 4}", 0.5) for k in range(10)]                   # two guards
    ev += [(2.5 + k * 0.7, f"ak_{1 + k % 5}", 1.0) for k in range(8)]                        # Ro's AK, 5 s
    ev += [(6.0, "explosion", 1.0), (3.0, "hit", 1.0), (5.0, "hit", 1.0), (7.0, "hit", 1.0), (9.0, "knife_hit", 1.0), (4.0, "sniper_shot", 0.7)]
    mix, mus, sfx = render(12, ev, lambda t: np.clip((t - 1.0) / 1.5, 0, 1), "firefight, everything at once")
    print(f"  A-weighted: music at start {loud(mus,0,1):.1f} dB | music in the fight {loud(mus,3,5):.1f} dB | effects in the fight {loud(sfx[:, None],3,5):.1f} dB")
    print(f"  effects over music (fight): {loud(sfx[:, None],3,5) - loud(mus,3,5):+.1f} dB   explosion window: {loud(mix,6.0,6.6):.1f} dB vs fight {loud(mix,3,5):.1f} dB")
    # calm exploration: just music and the odd step-less ambience: check the level is even across the crossfade
    mix2, mus2, _ = render(30, [], lambda t: np.clip((t - 5) / 10, 0, 1), "music only: crossfade calm -> tense over 10 s")
    print("  music A-weighted by 5 s windows:", [f"{loud(mus2, a, a+5):.1f}" for a in range(0, 30, 5)])
