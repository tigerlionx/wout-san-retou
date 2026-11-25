#!/usr/bin/env python3
"""Make the music scarier: dread layers on top of the existing calm/tense loops.

Needs only Python 3 and ffmpeg (no numpy). Re-runnable: the first run keeps the untouched loops in
assets/audio/local/orig/, later runs always start from those.

Added to both loops (the tense one gets more):
  * a tritone sub drone with harmonics so laptop speakers still hear it,
  * binaural beating (left and right a fraction of a hertz apart: it makes the head feel wrong),
  * a "ghost" pair a minor second apart that swells in and out,
  * breath/whisper noise (band-passed pink noise with irregular swells),
  * slow vibrato (a sour, drunk wobble) and a long echo on the original track.

Everything is built on whole numbers of cycles per loop and processed on a 3x tiled copy
(then the middle copy is cut), so the loop point is seamless and the loops stay the same length.
"""
import array, math, os, shutil, subprocess, sys, wave

HERE = os.path.dirname(os.path.abspath(__file__))
LOCAL = os.path.join(HERE, "..", "assets", "audio", "local")
ORIG = os.path.join(LOCAL, "orig")
SR = 44100


def run(cmd):
    r = subprocess.run(cmd, capture_output=True, text=True)
    if r.returncode:
        sys.exit(r.stderr[-2000:])
    return r.stderr


def loop_seconds(path):
    with wave.open(path) as w:
        return w.getnframes() / w.getframerate()


def snap(freq, secs):
    """Nearest frequency with a whole number of cycles in `secs`."""
    return max(1, round(freq * secs)) / secs


def dread_layers(path, secs, strength):
    n = int(round(secs * SR))
    L = array.array("h", bytes(2 * n))
    R = array.array("h", bytes(2 * n))
    # (frequency, amplitude, right-ear offset in Hz, swell cycles per loop (0 = steady))
    voices = [
        (41.2, 0.30, 0.0, 0), (58.3, 0.22, 0.0, 0),            # tritone sub drone
        (82.4, 0.10, 0.18, 0), (116.5, 0.09, 0.22, 0),         # its octave, audible on small speakers
        (233.0, 0.045, 0.31, 2), (329.6, 0.03, 0.4, 3),        # dark pad
        (1568.0, 0.020 * strength, 0.5, 3), (1661.2, 0.017 * strength, 0.5, 4),   # ghost minor second
        (2349.3, 0.010 * strength, 0.7, 5),                    # thin sour overtone
    ]
    for f, amp, dr, swell in voices:
        fl, fr = snap(f, secs), snap(f + dr, secs)
        wl, wr = 2 * math.pi * fl / SR, 2 * math.pi * fr / SR
        ws = 2 * math.pi * swell / n if swell else 0.0
        for i in range(n):
            g = amp * (0.55 + 0.45 * math.sin(ws * i)) if swell else amp
            L[i] += int(32000 * g * math.sin(wl * i)) if abs(L[i]) < 20000 else 0
            R[i] += int(32000 * g * math.sin(wr * i)) if abs(R[i]) < 20000 else 0
    inter = array.array("h", bytes(4 * n))
    inter[0::2], inter[1::2] = L, R
    with wave.open(path, "wb") as w:
        w.setnchannels(2); w.setsampwidth(2); w.setframerate(SR); w.writeframes(inter.tobytes())


def process(name, strength, vib_depth, echo, whisper_gain, crush):
    src = os.path.join(ORIG, f"{name}.wav")
    secs = loop_seconds(src)
    layers = os.path.join(ORIG, f"_{name}_layers.wav")
    dread_layers(layers, secs, strength)
    f1, f2, f3 = 12 / secs, 7 / secs, 19 / secs       # whole cycles per loop: no jump at the seam
    out = os.path.join(LOCAL, f"{name}.wav")
    tmp = out + ".tmp.wav"
    seam = secs
    total = secs * 3
    noise = lambda seed: (
        f"anoisesrc=d={total}:c=pink:r={SR}:seed={seed}:a=0.6,"
        f"bandpass=f=2800:w=2600,highpass=f=1200,"
        f"tremolo=f={f2:.6f}:d=0.95,tremolo=f={f3:.6f}:d=0.8,volume={whisper_gain}")
    chain = (
        f"[0:a]vibrato=f={f1:.6f}:d={vib_depth},"
        f"aecho=0.8:0.62:{echo}:0.38|0.28|0.2" + (",acrusher=bits=10:mix=0.35:mode=lin" if crush else "") + "[m];"
        f"[1:a]volume=1.0[l];"
        f"{'' }[2:a]pan=mono|c0=c0[w1];[3:a]pan=mono|c0=c0[w2];[w1][w2]join=inputs=2:channel_layout=stereo[w];"
        f"[m][l][w]amix=inputs=3:normalize=0:duration=first,"
        f"atrim=start={seam}:end={2 * seam},asetpts=PTS-STARTPTS,alimiter=limit=0.95[o]"
    )
    cmd = ["ffmpeg", "-y", "-v", "error",
           "-stream_loop", "2", "-i", src,
           "-stream_loop", "2", "-i", layers,
           "-f", "lavfi", "-i", noise(11), "-f", "lavfi", "-i", noise(23),
           "-filter_complex", chain, "-map", "[o]", "-ar", str(SR), "-ac", "2", "-c:a", "pcm_s16le", tmp]
    run(cmd)
    # match the loudness of the untouched loop (so the game's mix stays balanced)
    def peak(p):
        err = run(["ffmpeg", "-i", p, "-af", "volumedetect", "-f", "null", "-"])
        mean = float([l for l in err.splitlines() if "mean_volume" in l][0].split(":")[1].split()[0])
        return mean
    gain = peak(src) - peak(tmp)
    run(["ffmpeg", "-y", "-v", "error", "-i", tmp, "-af", f"volume={gain:.2f}dB,alimiter=limit=0.95", "-c:a", "pcm_s16le", out])
    os.remove(tmp); os.remove(layers)
    print(f"{name}: {secs:.2f} s, loudness matched ({gain:+.1f} dB)")


def main():
    os.makedirs(ORIG, exist_ok=True)
    for name in ("music_calm", "music_tense"):
        keep = os.path.join(ORIG, f"{name}.wav")
        if not os.path.exists(keep):
            shutil.copy(os.path.join(LOCAL, f"{name}.wav"), keep)
    process("music_calm", 1.0, 0.10, "420|730|1130", 0.16, False)
    process("music_tense", 1.6, 0.16, "180|310|470", 0.24, True)


if __name__ == "__main__":
    main()
