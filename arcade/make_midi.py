#!/usr/bin/env python3
"""Writes the arcade game's music as tiny Standard MIDI files (no dependencies).

Run:  python3 make_midi.py   ->  music/*.mid

All tunes are original, generated from a seed, so they stay a few KB each.
Channels: 0 bass (program 38), 1 arpeggio (program 80), 2 lead (program 81), 3 pad (program 89), 9 drums.
"""
import os
import random

PPQ = 480
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "music")

MODES = {
    "minor": [0, 2, 3, 5, 7, 8, 10],
    "phrygian": [0, 1, 3, 5, 7, 8, 10],
    "harmonic": [0, 2, 3, 5, 7, 8, 11],
    "locrian": [0, 1, 3, 5, 6, 8, 10],
    "dorian": [0, 2, 3, 5, 7, 9, 10],
    "major": [0, 2, 4, 5, 7, 9, 11],
}


def vlq(n):
    out = [n & 0x7F]
    n >>= 7
    while n:
        out.append((n & 0x7F) | 0x80)
        n >>= 7
    return bytes(reversed(out))


class Track:
    def __init__(self):
        self.events = []  # (tick, order, bytes)

    def add(self, tick, data, order=1):
        self.events.append((tick, order, data))

    def note(self, ch, tick, dur, pitch, vel=90):
        pitch = max(0, min(127, int(pitch)))
        self.add(tick, bytes([0x90 | ch, pitch, vel]), 1)
        self.add(tick + max(1, dur - 4), bytes([0x80 | ch, pitch, 0]), 0)

    def program(self, ch, prog):
        self.add(0, bytes([0xC0 | ch, prog]), 0)

    def bytes(self):
        self.events.sort(key=lambda e: (e[0], e[1]))
        body = b""
        last = 0
        for tick, _, data in self.events:
            body += vlq(tick - last) + data
            last = tick
        body += vlq(0) + b"\xFF\x2F\x00"
        return b"MTrk" + len(body).to_bytes(4, "big") + body


def write_midi(name, tracks, bpm):
    meta = Track()
    meta.add(0, b"\xFF\x51\x03" + int(60_000_000 / bpm).to_bytes(3, "big"), 0)
    meta.add(0, b"\xFF\x58\x04\x04\x02\x18\x08", 0)
    allt = [meta] + tracks
    head = b"MThd" + (6).to_bytes(4, "big") + (1).to_bytes(2, "big") + len(allt).to_bytes(2, "big") + PPQ.to_bytes(2, "big")
    data = head + b"".join(t.bytes() for t in allt)
    os.makedirs(OUT, exist_ok=True)
    with open(os.path.join(OUT, name + ".mid"), "wb") as f:
        f.write(data)
    return len(data)


def scale_note(root, mode, degree):
    m = MODES[mode]
    octave, idx = divmod(degree, 7)
    return root + 12 * octave + m[idx]


def drums(tr, bars, style, density=1.0, rnd=None):
    step = PPQ // 4  # 16th
    for bar in range(bars):
        for s in range(16):
            t = (bar * 16 + s) * step
            if style == "four":  # kick on every beat, snare on 2 and 4
                if s % 4 == 0:
                    tr.note(9, t, step, 36, 110)
                if s in (4, 12):
                    tr.note(9, t, step, 38, 100)
                if s % 2 == 0:
                    tr.note(9, t, step, 42, 55)
            elif style == "drive":  # kick pushes, hats run
                if s in (0, 6, 10) or (bar % 4 == 3 and s in (13, 14, 15)):
                    tr.note(9, t, step, 36, 112)
                if s in (4, 12):
                    tr.note(9, t, step, 38, 105)
                tr.note(9, t, step, 42 if s % 2 == 0 else 44, 50 if s % 2 else 66)
            elif style == "break":  # syncopated
                if s in (0, 3, 8, 11):
                    tr.note(9, t, step, 36, 108)
                if s in (4, 12, 14):
                    tr.note(9, t, step, 38, 100)
                if s % 2 == 0:
                    tr.note(9, t, step, 42, 58)
                if bar % 4 == 3 and s in (14, 15):
                    tr.note(9, t, step, 38, 90)
            elif style == "slow":
                if s in (0, 8):
                    tr.note(9, t, step, 36, 100)
                if s == 8 and bar % 2:
                    tr.note(9, t, step, 38, 80)
                if s % 4 == 2:
                    tr.note(9, t, step, 42, 45)
            elif style == "boss":
                if s % 4 == 0 or s in (7, 15):
                    tr.note(9, t, step, 36, 118)
                if s in (4, 12):
                    tr.note(9, t, step, 38, 112)
                tr.note(9, t, step, 42, 60)
                if s in (6, 14):
                    tr.note(9, t, step, 46, 70)


def tune(name, seed, root, mode, bpm, prog_degrees, style, bars=16, melody_density=0.55, arp_pattern=(0, 2, 4, 2), lead_prog=81):
    rnd = random.Random(seed)
    bass, arp, lead, pad, dr = Track(), Track(), Track(), Track(), Track()
    bass.program(0, 38)
    arp.program(1, 80)
    lead.program(2, lead_prog)
    pad.program(3, 89)
    step = PPQ // 4
    # motif of 4 bars, repeated and varied
    motif = []
    deg = rnd.choice([0, 2, 4])
    for i in range(16):
        if rnd.random() < melody_density:
            deg = max(-2, min(9, deg + rnd.choice([-2, -1, -1, 0, 1, 1, 2, 3])))
            motif.append(deg)
        else:
            motif.append(None)
    for bar in range(bars):
        chord = prog_degrees[(bar // 2) % len(prog_degrees)]
        base_t = bar * 16 * step
        # pad: sustained chord (root, third, fifth)
        for k in (0, 2, 4):
            pad.note(3, base_t, 16 * step, scale_note(root + 12, mode, chord + k), 42)
        # bass: driving eighths on the chord root
        for s in range(0, 16, 2):
            n = scale_note(root - 12, mode, chord)
            if s in (6, 14) and rnd.random() < 0.5:
                n += 12 if rnd.random() < 0.5 else 7
            bass.note(0, base_t + s * step, 2 * step, n, 100 if s % 8 == 0 else 84)
        # arpeggio: 16ths over the chord
        for s in range(16):
            k = arp_pattern[s % len(arp_pattern)]
            arp.note(1, base_t + s * step, step, scale_note(root + 12, mode, chord + k), 54 if s % 4 else 70)
        # lead: from bar 4, motif transposed by the chord
        if bar >= 4:
            for s in range(16):
                m = motif[(s + (bar % 4) * 4) % 16] if bar % 4 else motif[s]
                if m is not None and (s % 2 == 0 or rnd.random() < 0.4):
                    ln = 2 * step if rnd.random() < 0.6 else step
                    lead.note(2, base_t + s * step, ln, scale_note(root + 24, mode, m + chord // 2), 78)
    drums(dr, bars, style)
    return write_midi(name, [bass, arp, lead, pad, dr], bpm)


def fanfare(name, rising=True):
    lead, bass, dr = Track(), Track(), Track()
    lead.program(2, 81)
    bass.program(0, 38)
    step = PPQ // 2
    if rising:
        notes = [(60, 1), (64, 1), (67, 1), (72, 2), (71, 1), (72, 1), (76, 2), (79, 4)]
    else:
        notes = [(67, 2), (65, 2), (63, 2), (60, 2), (58, 2), (55, 6)]
    t = 0
    for p, d in notes:
        lead.note(2, t, d * step, p, 96)
        bass.note(0, t, d * step, p - 24, 90)
        t += d * step
    if rising:
        for s in range(0, t, step):
            dr.note(9, s, step, 36 if (s // step) % 2 == 0 else 38, 90)
    return write_midi(name, [bass, lead, dr], 120 if rising else 70)


if __name__ == "__main__":
    sizes = {}
    # Title: slow, dark
    sizes["title"] = tune("title", 1, 45, "minor", 84, [0, 5, 3, 4], "slow", bars=16, melody_density=0.4, arp_pattern=(0, 2, 4, 7))
    # Seven stages: each with its own key, mode, tempo and drum style
    stages = [
        (2, 38, "minor", 118, [0, 5, 3, 4], "four", (0, 2, 4, 2)),       # 1 yard: steady, tense
        (3, 41, "phrygian", 108, [0, 1, 0, 4], "slow", (0, 4, 2, 4)),   # 2 house maze: creeping
        (4, 43, "dorian", 126, [0, 3, 4, 0], "break", (0, 2, 4, 6)),    # 3 party wing: dance-floor menace
        (5, 40, "harmonic", 132, [0, 5, 6, 4], "drive", (0, 2, 4, 2)),  # 4 glass gallery: sharp
        (6, 37, "locrian", 120, [0, 1, 4, 1], "slow", (0, 3, 6, 3)),    # 5 altar hall: ritual
        (7, 42, "phrygian", 138, [0, 1, 5, 4], "drive", (0, 4, 2, 6)),  # 6 quarters: urgent
        (8, 44, "harmonic", 146, [0, 4, 5, 3], "break", (0, 2, 4, 7)),  # 7 roof: final push
    ]
    for i, (seed, root, mode, bpm, prog, style, arp) in enumerate(stages, 1):
        sizes[f"stage{i}"] = tune(f"stage{i}", seed, root, mode, bpm, prog, style, bars=16, arp_pattern=arp)
    # Boss themes: faster, heavier
    sizes["boss"] = tune("boss", 20, 41, "locrian", 162, [0, 1, 0, 6], "boss", bars=16, melody_density=0.75, arp_pattern=(0, 3, 6, 3))
    sizes["boss_final"] = tune("boss_final", 21, 40, "harmonic", 172, [0, 5, 1, 4], "boss", bars=16, melody_density=0.8, arp_pattern=(0, 2, 5, 7))
    sizes["win"] = fanfare("win", True)
    sizes["lose"] = fanfare("lose", False)
    total = sum(sizes.values())
    for k, v in sizes.items():
        print(f"{k:11s} {v:6d} bytes")
    print(f"total       {total:6d} bytes")
