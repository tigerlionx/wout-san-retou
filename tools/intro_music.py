#!/usr/bin/env python3
"""Remix a ritual drum track into the quiet, hypnotic bed that plays under the introduction cards.

Needs Python 3 and ffmpeg only.   Usage:
    python3 tools/intro_music.py /path/to/track.mp3 [start_seconds]
Writes assets/audio/local/intro_mystery.wav (a seamless loop; the game repeats it until ENTER starts the game).

What it does, and why (see the notes at the end of this file for what the research did and did not support):
  1. takes a steady 52 s stretch of the track and crossfades its end into its start, so it loops with no seam;
  2. slows it by 15 % and drops it about 2.8 semitones: the drums become heavier and further away;
  3. darkens it (low-pass 3.8 kHz), adds a slow pitch wobble and a long tape echo (0.48 / 1.0 / 2.0 s);
  4. blends in a big dark reverb (a synthetic 6 s room: noise that decays) that is about 5 dB under the dry sound;
  5. adds a drone: A1 with a fifth and a sour tritone that swells in and out, a very soft 6 Hz binaural pair,
     and a thin minor-second shimmer high up;
  6. a slow "breathing" swell (about one breath every 10 s) and a slow auto-pan, so the sound moves around the head;
  7. sets the loudness to about -26 LUFS (the game's music plays at about -17), quiet enough to read by.

Everything periodic (vibrato, swells, auto-pan, drone) has a whole number of cycles per loop and the processing runs on
a 3x tiled copy of which the middle copy is kept, so the loop point is seamless.
"""
import math, os, re, shutil, subprocess, sys, tempfile, wave

HERE = os.path.dirname(os.path.abspath(__file__))
LOCAL = os.path.join(HERE, "..", "assets", "audio", "local")
OUT = os.path.join(LOCAL, "intro_mystery.wav")
SR = 44100
LOOP = 52.0       # seconds of the source that make the loop
XFADE = 4.0       # seconds of crossfade at the seam
RATE = 0.85       # slow-down factor (and pitch: 12*log2(0.85) = -2.8 semitones)
TARGET_LUFS = -26.0


def ff(args):
    if "-c:a" not in args and args[-1].endswith(".wav"):
        args = args[:-1] + ["-c:a", "pcm_f32le", args[-1]]   # float intermediates: no quantisation noise in the quiet reverb
    r = subprocess.run(["ffmpeg", "-hide_banner", "-nostats", "-y", *args], capture_output=True, text=True)
    if r.returncode:
        sys.exit(r.stderr[-2500:])
    return r.stderr


def lufs(path):
    err = ff(["-i", path, "-af", "ebur128=peak=true", "-f", "null", "-"])
    return float(re.findall(r"I:\s+(-?[\d.]+) LUFS", err)[-1])


def frames(path):
    out = subprocess.run(["ffprobe", "-v", "error", "-select_streams", "a:0", "-show_entries", "stream=duration_ts",
                          "-of", "csv=p=0", path], capture_output=True, text=True).stdout
    return int(out.strip())


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    track = sys.argv[1]
    start = float(sys.argv[2]) if len(sys.argv) > 2 else 840.0
    tmp = tempfile.mkdtemp(prefix="intro_music_")
    p = lambda n: os.path.join(tmp, n)

    # 1. a seamless loop of the source
    ff(["-ss", str(start), "-t", str(LOOP + XFADE), "-i", track, "-ar", str(SR), "-ac", "2", p("excerpt.wav")])
    ff([
        "-i", p("excerpt.wav"), "-filter_complex",
        f"[0:a]atrim=0:{XFADE},asetpts=PTS-STARTPTS[h];[0:a]atrim={XFADE}:{LOOP},asetpts=PTS-STARTPTS[b];"
        f"[0:a]atrim={LOOP}:{LOOP + XFADE},asetpts=PTS-STARTPTS[t];"
        f"[t][h]acrossfade=d={XFADE}:c1=qsin:c2=qsin[x];[x][b]concat=n=2:v=0:a=1[o]",
        "-map", "[o]", p("loop.wav"),
    ])

    # 2. slower and lower
    ff(["-i", p("loop.wav"), "-af", f"asetrate={SR * RATE},aresample={SR}", p("slow.wav")])
    n = frames(p("slow.wav"))
    secs = n / SR
    print(f"loop is {secs:.2f} s ({n} samples)")

    # 4. a synthetic room: two decorrelated noises that fade out over 6 s, dark
    ir = lambda seed: f"anoisesrc=d=6:c=white:r={SR}:seed={seed}:a=0.9,volume='exp(-t*1.05)':eval=frame,lowpass=f=2800,highpass=f=140"
    ff(["-f", "lavfi", "-i", ir(7), "-f", "lavfi", "-i", ir(11), "-filter_complex", "[0][1]join=inputs=2:channel_layout=stereo", p("ir.wav")])

    # 5. the drone, on whole cycles per loop
    snap = lambda f: max(1, round(f * secs)) / secs
    voices = [  # (left Hz, right Hz, amplitude, swell cycles per loop (0 = steady))
        (55.0, 55.0, 0.20, 0),
        (82.5, 82.5, 0.10, 0),
        (77.8, 77.8, 0.07, 3),          # the sour tritone, swelling
        (110.0, 116.0, 0.045, 0),       # a soft 6 Hz binaural pair
        (1760.0, 1864.7, 0.010, 4),     # thin minor-second shimmer
    ]
    def expr(side):
        parts = []
        for fl, fr, a, sw in voices:
            f = snap(fl if side == 0 else fr)
            env = f"(0.55+0.45*sin(2*PI*{sw}*t/{secs}))" if sw else "1"
            parts.append(f"{a}*{env}*sin(2*PI*{f}*t)")
        return "+".join(parts)
    ff(["-f", "lavfi", "-i", f"aevalsrc={expr(0)}|{expr(1)}:s={SR}:d={secs}", p("drone.wav")])

    # tile the loop and the drone three times; the processing below leaves the middle copy seamless
    for name in ("slow", "drone"):
        ff(["-stream_loop", "2", "-i", p(f"{name}.wav"), "-t", f"{3 * secs}", p(f"{name}3.wav")])

    # 3-6. dark, wobbling, echoing, reverberant, breathing, panning
    vib = round(0.35 * secs) / secs         # ~0.35 Hz pitch wobble
    breath = round(0.1 * secs)              # ~0.1 Hz: one breath per 10 s
    pan = round(0.07 * secs) / secs         # ~0.07 Hz auto-pan
    base = (f"highpass=f=45,lowpass=f=3800,vibrato=f={vib}:d=0.15,"
            f"aecho=0.8:0.82:480|1010|2020:0.38|0.28|0.18")
    ff(["-i", p("slow3.wav"), "-af", base, p("dark3.wav")])
    # reverb level: wet about 5 dB under the dry sound
    ff(["-i", p("dark3.wav"), "-i", p("ir.wav"), "-filter_complex", "[0][1]afir=dry=0:wet=1", p("wet3.wav")])
    gain = lufs(p("dark3.wav")) - 5.0 - lufs(p("wet3.wav"))
    print(f"reverb gain {gain:+.1f} dB")
    ff([
        "-i", p("dark3.wav"), "-i", p("wet3.wav"), "-i", p("drone3.wav"), "-filter_complex",
        f"[1]volume={gain}dB[w];[0][w]amix=inputs=2:weights=1 1:normalize=0[m];"
        f"[m][2]amix=inputs=2:weights=1 0.55:normalize=0,"
        f"volume='0.78+0.22*sin(2*PI*{breath}*t/{3 * secs})':eval=frame,"
        f"apulsator=mode=sine:hz={pan}:amount=0.28[o]",
        "-map", "[o]", p("mix3.wav"),
    ])

    # set the level on the middle copy, limit the peaks on the whole tiled signal (the limiter delays its start),
    # then cut the middle copy
    ff(["-i", p("mix3.wav"), "-af", f"atrim=start_sample={n}:end_sample={2 * n},asetpts=PTS-STARTPTS", p("mid.wav")])
    g = TARGET_LUFS - lufs(p("mid.wav"))
    ff(["-i", p("mix3.wav"), "-af",
        f"volume={g}dB,alimiter=limit=0.5:level=disabled,atrim=start_sample={n}:end_sample={2 * n},asetpts=PTS-STARTPTS",
        "-ar", str(SR), "-c:a", "pcm_s16le", OUT])
    print(f"wrote {OUT}: {lufs(OUT):.1f} LUFS")
    shutil.rmtree(tmp, ignore_errors=True)


if __name__ == "__main__":
    main()

# Notes on the research (2026-11-25). Sources found: Lehne and Koelsch 2015 (tension arises from conflict, instability,
# dissonance and uncertainty); Trevor et al. 2023 (terrifying music is loud and dense, anxiety is varied, with sounds
# sprinkled in unexpected places); game-audio articles on drones (low sustained sounds convey space and ambiguity),
# delay and reverb for murk and distance, and near-inaudible sounds as subliminal cues; silence and contrast.
# Not found: any study giving a loudness target, a reverb time or an echo setting for suspense, and nothing solid for
# "brain vibration" claims (binaural beats at 4-8 Hz, infrasound). The numbers above are craft choices, kept quiet so
# the text can be read. Adjust by ear: RATE, the reverb gain, TARGET_LUFS.
