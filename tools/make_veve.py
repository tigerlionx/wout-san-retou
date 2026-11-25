#!/usr/bin/env python3
"""Draw the billboard images for the side panel: vèvè-style sigils (cornmeal lines on a dark ground),
the saint-halo emblems of the lwa, snakes, crosses and wangas (charms).

They are our own stylised drawings in the vèvè idiom, not copies of any houngan's or mambo's
sacred designs. Haitian Vodou is a religion, shown here with respect; never as the villain.
Writes assets/veve/*.png (transparent background) and assets/veve/led.png (the billboard grid).
Usage: python3 tools/make_veve.py
"""
import math, os
from PIL import Image, ImageDraw, ImageFilter

S = 1024            # output size
K = 2               # supersampling
N = S * K
OUT = os.path.join(os.path.dirname(__file__), "..", "assets", "veve")
CREAM = (250, 240, 214, 255)
GOLD = (255, 190, 84, 255)
LW = 9 * K          # line width


class Cv:
    def __init__(self):
        self.im = Image.new("RGBA", (N, N), (0, 0, 0, 0))
        self.d = ImageDraw.Draw(self.im)
        self.sgn = 1

    # unit coordinates: (0,0) centre, x/y in -1..1 (y down)
    def P(self, x, y):
        return (N / 2 + self.sgn * x * N / 2 * 0.92, N / 2 + y * N / 2 * 0.92)

    def line(self, pts, w=LW, col=CREAM):
        p = [self.P(*q) for q in pts]
        self.d.line(p, fill=col, width=w, joint="curve")
        for q in (p[0], p[-1]):
            self.d.ellipse([q[0] - w / 2, q[1] - w / 2, q[0] + w / 2, q[1] + w / 2], fill=col)

    def dots(self, a, b, n, r=7 * K, col=CREAM):
        for i in range(n + 1):
            t = i / max(n, 1)
            x, y = self.P(a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)
            self.d.ellipse([x - r, y - r, x + r, y + r], fill=col)

    def circle(self, c, r, w=LW, col=CREAM, fill=None):
        x, y = self.P(*c)
        rr = r * N / 2 * 0.92
        self.d.ellipse([x - rr, y - rr, x + rr, y + rr], outline=col, width=w, fill=fill)

    def ring_dots(self, c, r, n, dr=6 * K, col=CREAM):
        for i in range(n):
            a = 2 * math.pi * i / n
            self.dots((c[0] + r * math.cos(a), c[1] + r * math.sin(a)), (c[0] + r * math.cos(a), c[1] + r * math.sin(a)), 0, dr, col)

    def star(self, c, r, rays=8, w=int(LW * 0.7), col=CREAM):
        for i in range(rays):
            a = math.pi * i / rays * 2
            self.line([(c[0] - r * math.cos(a) / 2, c[1] - r * math.sin(a) / 2), (c[0] + r * math.cos(a) / 2, c[1] + r * math.sin(a) / 2)], w, col)

    def cross(self, c, s, w=LW, col=CREAM, serif=True):
        x, y = c
        self.line([(x, y - s), (x, y + s)], w, col)
        self.line([(x - s * 0.7, y - s * 0.35), (x + s * 0.7, y - s * 0.35)], w, col)
        if serif:
            for dx, dy, h in ((0, -s, 1), (0, s, 1)):
                self.line([(x - s * 0.18, y + dy), (x + s * 0.18, y + dy)], w, col)
            for dx in (-0.7, 0.7):
                self.line([(x + s * dx, y - s * 0.35 - s * 0.16), (x + s * dx, y - s * 0.35 + s * 0.16)], w, col)

    def heart(self, c, s, w=LW, col=CREAM, fill=None):
        pts = []
        for i in range(0, 361, 4):
            t = math.radians(i)
            hx = 16 * math.sin(t) ** 3
            hy = -(13 * math.cos(t) - 5 * math.cos(2 * t) - 2 * math.cos(3 * t) - math.cos(4 * t))
            pts.append((c[0] + hx / 17 * s, c[1] + (hy + 2) / 17 * s))
        p = [self.P(*q) for q in pts]
        if fill:
            self.d.polygon(p, fill=fill)
        self.d.line(p + [p[0]], fill=col, width=w, joint="curve")

    def flag(self, base, h, w=LW, col=CREAM):
        x, y = base
        self.line([(x, y), (x, y - h)], w, col)
        self.line([(x, y - h), (x + h * 0.55, y - h * 0.8), (x, y - h * 0.6)], w, col)

    def snake(self, x0, y0, y1, amp, waves, phase=0.0, w=int(LW * 1.4), col=CREAM, head=True):
        pts = []
        steps = 120
        for i in range(steps + 1):
            t = i / steps
            pts.append((x0 + amp * math.sin(phase + t * waves * 2 * math.pi) * (0.55 + 0.45 * t), y0 + (y1 - y0) * t))
        self.line(pts, w, col)
        if head:
            hx, hy = pts[0]
            self.line([(hx - 0.045, hy - 0.01), (hx, hy - 0.07), (hx + 0.045, hy - 0.01), (hx, hy + 0.025), (hx - 0.045, hy - 0.01)], int(LW * 0.8), col)
            self.line([(hx, hy - 0.07), (hx - 0.02, hy - 0.115)], int(LW * 0.5), col)
            self.line([(hx, hy - 0.07), (hx + 0.02, hy - 0.115)], int(LW * 0.5), col)

    def halo(self, c, r, col=GOLD):
        self.circle(c, r, int(LW * 0.8), col)
        self.circle(c, r * 0.86, int(LW * 0.5), col)
        for i in range(24):
            a = 2 * math.pi * i / 24
            r0, r1 = r * 1.04, r * (1.16 if i % 2 == 0 else 1.1)
            self.line([(c[0] + r0 * math.cos(a), c[1] + r0 * math.sin(a)), (c[0] + r1 * math.cos(a), c[1] + r1 * math.sin(a))], int(LW * 0.7), col)

    def wave(self, y, x0=-0.75, x1=0.75, amp=0.03, n=3, w=int(LW * 0.8), col=CREAM):
        pts = [(x0 + (x1 - x0) * i / 100, y + amp * math.sin(i / 100 * n * 2 * math.pi)) for i in range(101)]
        self.line(pts, w, col)

    def mirror(self, fn):
        for s in (1, -1):
            self.sgn = s
            fn()
        self.sgn = 1

    def save(self, name):
        glow = self.im.filter(ImageFilter.GaussianBlur(14 * K))
        tint = Image.new("RGBA", (N, N), GOLD)
        tint.putalpha(glow.getchannel("A").point(lambda v: min(255, int(v * 0.9))))
        base = Image.alpha_composite(tint, self.im)
        base = base.resize((S, S), Image.LANCZOS)
        os.makedirs(OUT, exist_ok=True)
        base.save(os.path.join(OUT, name + ".png"))


def frame(cv, r=0.93, ring=True):
    if ring:
        cv.circle((0, 0), r, int(LW * 0.6))
        cv.ring_dots((0, 0), r * 0.94, 48, 4 * K)


# --------------------------------------------------------------------------- the images

def legba():
    """Papa Legba, who opens the gate: the crossroads cross, the cane, the key."""
    cv = Cv(); frame(cv)
    cv.line([(0, -0.78), (0, 0.78)]); cv.line([(-0.78, 0), (0.78, 0)])
    cv.line([(-0.55, -0.55), (0.55, 0.55)], int(LW * 0.6)); cv.line([(-0.55, 0.55), (0.55, -0.55)], int(LW * 0.6))
    for p in ((0, -0.8), (0, 0.8), (-0.8, 0), (0.8, 0)):
        cv.cross(p, 0.1, int(LW * 0.7))
    cv.circle((0, 0), 0.16); cv.circle((0, 0), 0.07, fill=CREAM)
    cv.mirror(lambda: (cv.dots((0.1, -0.1), (0.5, -0.5), 8), cv.star((0.62, -0.62), 0.12, 8)))
    cv.mirror(lambda: cv.line([(0.3, 0.3), (0.3, 0.55), (0.36, 0.62), (0.44, 0.55)], int(LW * 0.8)))  # the crutch-cane
    cv.save("legba")


def danbala():
    """Danbala Wèdo and Ayida Wèdo: two snakes on the pole, flags, the egg."""
    cv = Cv(); frame(cv)
    cv.line([(0, -0.82), (0, 0.72)], int(LW * 0.7))
    cv.snake(0, -0.66, 0.68, 0.19, 2.5, 0.0)
    cv.snake(0, -0.66, 0.68, 0.19, 2.5, math.pi, head=False)
    cv.mirror(lambda: (cv.flag((0.5, 0.45), 0.5), cv.star((0.62, -0.1), 0.18, 6), cv.dots((0.1, 0.7), (0.5, 0.45), 6)))
    cv.circle((0, 0.8), 0.07)
    cv.heart((0, 0.57), 0.17, int(LW * 0.7))
    cv.save("danbala")


def ogou():
    """Ogou, the iron lwa of war and work: the sabre between two flags, Sen Jak's halo."""
    cv = Cv(); frame(cv, ring=False)
    cv.halo((0, -0.3), 0.36)
    cv.line([(0, -0.7), (0.05, 0.4)], int(LW * 1.5))
    cv.line([(-0.18, 0.18), (0.18, 0.18)], int(LW * 1.1)); cv.line([(0, 0.4), (0, 0.62)], int(LW * 1.3)); cv.circle((0, 0.66), 0.05, fill=CREAM)
    cv.mirror(lambda: (cv.flag((0.45, 0.62), 0.75), cv.star((0.56, 0.78), 0.16, 8), cv.dots((0.12, 0.3), (0.45, 0.62), 5)))
    cv.line([(-0.6, 0.86), (0.6, 0.86)], int(LW * 0.8))
    cv.save("ogou")


def erzulie():
    """Erzulie Freda: love, beauty, mirror and perfume. The heart in its halo."""
    cv = Cv(); frame(cv, ring=False)
    cv.halo((0, 0), 0.62)
    cv.heart((0, 0), 0.8, int(LW * 1.2))
    cv.heart((0, 0.02), 0.45, int(LW * 0.8))
    cv.mirror(lambda: (cv.heart((0.62, 0.72), 0.2, int(LW * 0.7)), cv.heart((0.7, -0.62), 0.14, int(LW * 0.6)), cv.star((0.78, 0.2), 0.14, 8)))
    cv.line([(-0.1, 0.12), (0.1, 0.12)], int(LW * 0.5))
    cv.save("erzulie")


def dantor():
    """Erzulie Dantò, the fierce mother: the heart pierced by the knife, two scars."""
    cv = Cv(); frame(cv)
    cv.heart((0, 0.02), 0.82, int(LW * 1.2))
    cv.line([(-0.5, -0.5), (0.4, 0.45)], int(LW * 1.3), GOLD)
    cv.line([(-0.5, -0.5), (-0.56, -0.62), (-0.4, -0.58), (-0.5, -0.5)], int(LW * 0.9), GOLD)
    cv.line([(0.4, 0.45), (0.5, 0.68)], int(LW * 0.9), GOLD)
    cv.mirror(lambda: (cv.line([(0.16, -0.3), (0.24, -0.2)], int(LW * 0.7)), cv.star((0.62, 0.72), 0.16, 8), cv.cross((0.7, -0.45), 0.1)))
    cv.save("dantor")


def baron():
    """Baron Samedi and the Gede: the cross on the grave, the coffin, the top hat."""
    cv = Cv(); frame(cv)
    cv.cross((0, -0.28), 0.5, int(LW * 1.3))
    # the tomb steps
    for i, w in enumerate((0.5, 0.38, 0.26)):
        cv.line([(-w, 0.5 + i * 0.1), (w, 0.5 + i * 0.1)], int(LW * 1.1))
    cv.line([(-0.5, 0.5), (-0.5, 0.6)]); cv.line([(0.5, 0.5), (0.5, 0.6)])
    # coffin outline
    cv.mirror(lambda: cv.line([(0.18, 0.78), (0.3, 0.88), (0.18, 0.98)], int(LW * 0.7)))
    # the hat
    cv.line([(-0.2, -0.76), (0.2, -0.76)], int(LW * 1.0)); cv.line([(-0.1, -0.76), (-0.1, -0.9), (0.1, -0.9), (0.1, -0.76)], int(LW * 1.0))
    cv.mirror(lambda: (cv.cross((0.66, 0.1), 0.14, int(LW * 0.8)), cv.cross((0.58, -0.45), 0.1, int(LW * 0.6))))
    cv.save("baron")


def agwe():
    """Agwe, king of the sea: the boat, its sail, the waves."""
    cv = Cv(); frame(cv)
    cv.line([(-0.55, 0.18), (-0.38, 0.45), (0.38, 0.45), (0.55, 0.18), (-0.55, 0.18)], int(LW * 1.2))
    cv.line([(0, 0.18), (0, -0.7)], int(LW * 1.1))
    cv.line([(0.02, -0.66), (0.42, 0.08), (0.02, 0.08)], int(LW * 1.0))
    cv.line([(-0.02, -0.5), (-0.34, 0.08), (-0.02, 0.08)], int(LW * 0.8))
    cv.flag((0, -0.7), 0.2, int(LW * 0.7))
    for i, y in enumerate((0.58, 0.7, 0.82)):
        cv.wave(y, -0.62 + 0.06 * i, 0.62 - 0.06 * i, 0.03, 3 + i)
    cv.mirror(lambda: cv.star((0.6, -0.5), 0.2, 8))
    cv.save("agwe")


def simbi():
    """Simbi, guardian of springs and rivers: the coiled snake and the water."""
    cv = Cv(); frame(cv)
    pts = []
    for i in range(0, 361):
        a = math.radians(i * 3.4)
        r = 0.08 + i / 360 * 0.55
        pts.append((r * math.cos(a), -0.05 + r * math.sin(a) * 0.95))
    cv.line(pts, int(LW * 1.6))
    hx, hy = pts[-1]
    cv.line([(hx, hy), (hx + 0.1, hy - 0.1), (hx + 0.22, hy - 0.08)], int(LW * 1.3))
    cv.circle((0, -0.05), 0.04, fill=CREAM)
    for i, y in enumerate((0.78, 0.88)):
        cv.wave(y, -0.5, 0.5, 0.025, 4)
    cv.mirror(lambda: cv.circle((0.66, -0.55), 0.07, int(LW * 0.7)))
    cv.save("simbi")


def asson():
    """The asson (rattle) and the drum: what calls the lwa."""
    cv = Cv(); frame(cv)
    # the asson: a calabash on a handle, beads around it
    cv.circle((-0.28, -0.3), 0.3, int(LW * 1.1))
    cv.line([(-0.28, -0.0), (-0.28, 0.45)], int(LW * 1.2))
    for i in range(14):
        a = i / 14 * 2 * math.pi
        cv.circle((-0.28 + 0.4 * math.cos(a), -0.3 + 0.4 * math.sin(a)), 0.018, int(LW * 0.5), GOLD, GOLD)
    cv.star((-0.28, -0.3), 0.3, 6, int(LW * 0.6))
    # the drum
    cv.line([(0.1, 0.0), (0.7, 0.0)], int(LW * 1.1)); cv.line([(0.14, 0.0), (0.2, 0.55), (0.6, 0.55), (0.66, 0.0)], int(LW * 1.1))
    cv.line([(0.1, 0.0), (0.7, 0.0)], int(LW * 1.1))
    for i in range(5):
        x = 0.16 + i * 0.1
        cv.line([(x, 0.0), (x + (0.02 if i < 2 else -0.02 if i > 2 else 0), 0.55)], int(LW * 0.5))
    cv.star((0.4, -0.16), 0.2, 8, int(LW * 0.6))
    cv.dots((-0.7, 0.72), (0.7, 0.72), 18)
    cv.save("asson")


def wanga():
    """A wanga (charm): the bound packet with its cross, the pins, the thread."""
    cv = Cv(); frame(cv)
    # the pwen: a knotted cloth bundle
    cv.line([(-0.32, -0.05), (-0.22, 0.45), (0.22, 0.45), (0.32, -0.05), (0.14, -0.18), (-0.14, -0.18), (-0.32, -0.05)], int(LW * 1.2))
    cv.line([(-0.12, -0.18), (-0.18, -0.42), (0, -0.34), (0.18, -0.42), (0.12, -0.18)], int(LW * 1.0))
    for i in range(5):
        y = -0.12 + i * 0.1
        cv.line([(-0.28 + 0.01 * i, y), (0.28 - 0.01 * i, y + 0.04)], int(LW * 0.5), GOLD)
    cv.cross((0, 0.2), 0.14, int(LW * 0.8))
    # pins and the feather
    cv.mirror(lambda: (cv.line([(0.45, 0.0), (0.7, 0.25)], int(LW * 0.7)), cv.circle((0.72, 0.28), 0.03, fill=GOLD), cv.line([(0.45, -0.3), (0.72, -0.55)], int(LW * 0.7)), cv.circle((0.74, -0.58), 0.03, fill=GOLD)))
    cv.dots((-0.4, 0.7), (0.4, 0.7), 12)
    cv.star((0, 0.82), 0.14, 8)
    cv.save("wanga")


def govi():
    """A govi, the sacred jar where a lwa lives, with its cross and the candle."""
    cv = Cv(); frame(cv)
    cv.line([(-0.1, -0.5), (-0.1, -0.35), (-0.3, -0.1), (-0.32, 0.35), (-0.22, 0.52), (0.22, 0.52), (0.32, 0.35), (0.3, -0.1), (0.1, -0.35), (0.1, -0.5), (-0.1, -0.5)], int(LW * 1.2))
    cv.line([(-0.12, -0.55), (0.12, -0.55)], int(LW * 1.2))
    cv.cross((0, 0.1), 0.2, int(LW * 0.9))
    # the candle
    cv.line([(-0.62, 0.3), (-0.62, 0.6)], int(LW * 2.0)); cv.line([(-0.62, 0.3), (-0.68, 0.12), (-0.62, 0.04), (-0.56, 0.12), (-0.62, 0.3)], int(LW * 0.8), GOLD)
    cv.line([(0.62, 0.3), (0.62, 0.6)], int(LW * 2.0)); cv.line([(0.62, 0.3), (0.56, 0.12), (0.62, 0.04), (0.68, 0.12), (0.62, 0.3)], int(LW * 0.8), GOLD)
    cv.mirror(lambda: (cv.star((0.42, -0.5), 0.2, 8), cv.dots((0.15, 0.7), (0.5, 0.7), 5)))
    cv.save("govi")


def saint():
    """The saint's halo: how the lwa walk beside the Catholic saints in Haiti. A face-less icon."""
    cv = Cv(); frame(cv, ring=False)
    cv.halo((0, -0.18), 0.45)
    cv.circle((0, -0.18), 0.16, int(LW * 1.0))
    cv.line([(-0.38, 0.5), (-0.28, 0.2), (-0.12, 0.12), (0.12, 0.12), (0.28, 0.2), (0.38, 0.5)], int(LW * 1.2))
    cv.line([(0, 0.12), (0, 0.5)], int(LW * 0.6))
    cv.cross((0, 0.3), 0.1, int(LW * 0.7))
    cv.mirror(lambda: (cv.star((0.6, 0.6), 0.16, 8), cv.heart((0.64, -0.5), 0.14, int(LW * 0.7))))
    cv.line([(-0.6, 0.8), (0.6, 0.8)], int(LW * 0.8)); cv.dots((-0.5, 0.9), (0.5, 0.9), 12)
    cv.save("saint")


def led():
    """The billboard grid: a bulb on every 4 px, thin dark gaps between."""
    t = 4
    im = Image.new("RGBA", (t, t), (0, 0, 0, 0))
    px = im.load()
    for i in range(t):
        px[i, t - 1] = (0, 0, 0, 190)
        px[t - 1, i] = (0, 0, 0, 190)
    px[0, 0] = (255, 255, 255, 60)
    im.save(os.path.join(OUT, "led.png"))


if __name__ == "__main__":
    for f in (legba, danbala, ogou, erzulie, dantor, baron, agwe, simbi, asson, wanga, govi, saint):
        f()
    led()
    print("wrote", sorted(os.listdir(OUT)))
