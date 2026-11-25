"""Measure the game's sounds: A-weighted loudness over the active part, true peak, DC, and clicks."""
import glob, os, sys, wave
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
A = os.path.join(HERE, "..", "assets", "audio")


def read(path):
    w = wave.open(path)
    x = np.frombuffer(w.readframes(w.getnframes()), dtype="<i2").astype(np.float64) / 32768
    x = x.reshape(-1, w.getnchannels())
    return x, w.getframerate()


def a_weight(f):
    f = np.maximum(f, 1e-3)
    ra = (12194**2 * f**4) / ((f**2 + 20.6**2) * np.sqrt((f**2 + 107.7**2) * (f**2 + 737.9**2)) * (f**2 + 12194**2))
    f1 = 1000.0
    ra1 = (12194**2 * f1**4) / ((f1**2 + 20.6**2) * np.sqrt((f1**2 + 107.7**2) * (f1**2 + 737.9**2)) * (f1**2 + 12194**2))
    return ra / ra1


def aw(x, sr):
    n = len(x)
    H = a_weight(np.fft.rfftfreq(n, 1 / sr))
    return np.stack([np.fft.irfft(np.fft.rfft(x[:, c]) * H, n) for c in range(x.shape[1])], axis=1)


def db(v):
    return 20 * np.log10(v + 1e-9)


def measure(path):
    x, sr = read(path)
    m = x.mean(axis=1, keepdims=True)
    y = aw(m, sr)[:, 0]
    peak = np.abs(x).max()
    # the active part: from the first sample above -40 dB of the peak for up to 150 ms (shots) or the whole clip (beds)
    dur = len(x) / sr
    if dur < 1.5:
        s = int(np.argmax(np.abs(m[:, 0]) > peak * 0.01))
        seg = y[s : s + int(0.15 * sr)]
        what = "first 150ms"
    else:
        seg = y
        what = "whole"
    loud = db(np.sqrt((seg**2).mean()))
    clicks = np.abs(np.diff(m[:, 0])).max()
    return dict(dur=dur, peak=db(peak), loud=loud, what=what, dc=float(m.mean()), start=float(abs(m[0, 0])), end=float(abs(m[-1, 0])), click=float(clicks))


if __name__ == "__main__":
    files = sorted(glob.glob(os.path.join(A, "*", "*.wav")))
    print(f"{'file':28s} {'dur':>5s} {'peak':>6s} {'A-loud':>7s}  measured over   start/end amp  max step")
    for p in files:
        if "showcase" in p:
            continue
        r = measure(p)
        print(f"{os.path.relpath(p, A):28s} {r['dur']:5.2f} {r['peak']:6.1f} {r['loud']:7.1f}  {r['what']:12s} {r['start']:.3f}/{r['end']:.3f}  {r['click']:.2f}")
