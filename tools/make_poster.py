#!/usr/bin/env python3
"""Compose the Wout San Retou poster from the game's own characters.

Step 1 (renders the cast offscreen on a flat magenta background; ~25 s):
    WSR_AUTOPLAY=1 WSR_POSTER=1 WSR_SIZE=1200x1800 ./target/release/wout-san-retou     # -> playtest/poster_raw.png
    (use WSR_SIZE=2400x3600 for the print size)
Step 2 (this script; needs Pillow):
    python3 tools/make_poster.py [playtest/poster_raw.png] [playtest/poster.png]

It keys the magenta out, builds a dark vevè kaleidoscope sky with a red moon and mist, puts the cast over it and sets the
title in Fira Sans.  All characters are fictional.
"""
import math, os, random, sys
from PIL import Image, ImageChops, ImageDraw, ImageFilter, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..")
SRC = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "playtest", "poster_raw.png")
OUT = sys.argv[2] if len(sys.argv) > 2 else os.path.join(ROOT, "playtest", "poster.png")
FONT_B = os.path.join(ROOT, "assets", "fonts", "FiraSans-Bold.ttf")
FONT_R = os.path.join(ROOT, "assets", "fonts", "FiraSans-Regular.ttf")
random.seed(666)


def key_out(img):
    """Chroma-key against the pixel at (0, 0); un-mix the edge pixels so no magenta fringe is left."""
    rgb = img.convert("RGB")
    kr, kg, kb = rgb.getpixel((0, 0))
    r, g, b = rgb.split()
    d = ImageChops.add(ImageChops.add(ImageChops.difference(r, Image.new("L", rgb.size, kr)),
                                      ImageChops.difference(g, Image.new("L", rgb.size, kg))),
                       ImageChops.difference(b, Image.new("L", rgb.size, kb)))
    a = d.point(lambda v: 0 if v < 14 else min(255, int((v - 14) * 255 / 70)))
    a = a.filter(ImageFilter.MinFilter(3)).filter(ImageFilter.GaussianBlur(0.8))   # eat the one-pixel halo
    # un-mix: c = (px - (1-a) key) / a
    px, ap = rgb.load(), a.load()
    out = Image.new("RGBA", rgb.size)
    po = out.load()
    for y in range(rgb.size[1]):
        for x in range(rgb.size[0]):
            al = ap[x, y]
            if al == 0:
                continue
            p = px[x, y]
            if al < 255:
                f = al / 255.0
                p = tuple(max(0, min(255, int((c - (1 - f) * k) / f))) for c, k in zip(p, (kr, kg, kb)))
            po[x, y] = (*p, al)
    return out


def grade(cast):
    """Calm the hot reds and oranges (the jumpsuit, the dress) so the violet and gold of the poster lead."""
    alpha = cast.split()[-1]
    h, sat, v = cast.convert("RGB").convert("HSV").split()
    hot = h.point(lambda x: 255 if (x < 24 or x > 232) else 0)
    hot = ImageChops.multiply(hot, sat.point(lambda x: 255 if x > 100 else 0)).filter(ImageFilter.GaussianBlur(1.5))
    sat2 = sat.point(lambda x: int(x * 0.76))
    v2 = v.point(lambda x: int(x * 0.86))
    h2 = h.point(lambda x: x if x > 128 else (x + 255 if False else x))
    soft = Image.merge("HSV", (h, sat2, v2)).convert("RGB")
    base = Image.merge("HSV", (h, sat, v)).convert("RGB")
    out = Image.composite(soft, base, hot).convert("RGBA")
    out.putalpha(alpha)
    return out


def glow(size, center, radius, color, alpha):
    """A soft round glow as an RGBA layer."""
    w, h = size
    layer = Image.new("RGBA", size, (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)
    d.ellipse([center[0] - radius, center[1] - radius, center[0] + radius, center[1] + radius], fill=(*color, alpha))
    return layer.filter(ImageFilter.GaussianBlur(radius * 0.45))


def tint(sigil, color, opacity):
    """A sigil (cream lines on transparent) recoloured and faded."""
    a = sigil.split()[-1].point(lambda v: int(v * opacity))
    out = Image.new("RGBA", sigil.size, (*color, 0))
    out.putalpha(a)
    return out


def kaleidoscope(size, center, base_r):
    w, h = size
    layer = Image.new("RGBA", size, (0, 0, 0, 0))
    names = ["legba", "danbala", "ogou", "erzulie", "dantor", "baron", "agwe", "simbi", "asson", "wanga", "govi", "saint"]
    sig = {n: Image.open(os.path.join(ROOT, "assets", "veve", n + ".png")).convert("RGBA") for n in names}
    rings = [  # (pieces, radius factor, size factor, colour, opacity, phase)
        (12, 1.00, 0.62, (212, 160, 60), 0.34, 0.0),
        (8, 0.62, 0.50, (190, 60, 70), 0.34, 0.2),
        (5, 0.30, 0.40, (150, 110, 230), 0.30, 0.1),
    ]
    k = 0
    for n, rf, sf, col, op, ph in rings:
        for i in range(n):
            ang = 2 * math.pi * (i / n + ph / n)
            s = sig[names[k % len(names)]]
            k += 1
            pw = int(base_r * sf)
            piece = tint(s.resize((pw, pw), Image.LANCZOS), col, op)
            if i % 2:
                piece = piece.transpose(Image.FLIP_LEFT_RIGHT)
            piece = piece.rotate(-math.degrees(ang) - 90, resample=Image.BICUBIC, expand=True)
            cx = center[0] + math.cos(ang) * base_r * rf - piece.size[0] / 2
            cy = center[1] + math.sin(ang) * base_r * rf - piece.size[1] / 2
            layer.alpha_composite(piece, (int(cx), int(cy)))
    # the central sigil, bigger
    c = tint(sig["legba"].resize((int(base_r * 0.5),) * 2, Image.LANCZOS), (255, 215, 140), 0.5)
    layer.alpha_composite(c, (int(center[0] - c.size[0] / 2), int(center[1] - c.size[1] / 2)))
    return layer


def mist(size, y0, y1, color, alpha, seed):
    w, h = size
    rnd = random.Random(seed)
    layer = Image.new("RGBA", size, (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)
    for _ in range(26):
        cx, cy = rnd.randint(-w // 8, w + w // 8), rnd.randint(int(y0), int(y1))
        rx, ry = rnd.randint(w // 6, w // 3), rnd.randint(h // 50, h // 22)
        d.ellipse([cx - rx, cy - ry, cx + rx, cy + ry], fill=(*color, rnd.randint(alpha // 2, alpha)))
    return layer.filter(ImageFilter.GaussianBlur(h / 90))


def text_center(img, text, y, font, fill, glow_col=None, stroke=0, spacing=0.0):
    """Draw centred text with optional glow; returns the bottom y."""
    w, h = img.size
    d = ImageDraw.Draw(img)
    # tracking: draw letter by letter
    widths = [d.textlength(ch, font=font) for ch in text]
    total = sum(widths) + spacing * font.size * (len(text) - 1)
    x = (w - total) / 2
    if glow_col:
        g = Image.new("RGBA", img.size, (0, 0, 0, 0))
        gd = ImageDraw.Draw(g)
        xx = x
        for ch, cw in zip(text, widths):
            gd.text((xx, y), ch, font=font, fill=(*glow_col, 255), stroke_width=stroke + 2, stroke_fill=(*glow_col, 255))
            xx += cw + spacing * font.size
        g = g.filter(ImageFilter.GaussianBlur(font.size * 0.12))
        img.alpha_composite(g)
        img.alpha_composite(g)
    xx = x
    d = ImageDraw.Draw(img)
    for ch, cw in zip(text, widths):
        d.text((xx, y), ch, font=font, fill=fill, stroke_width=stroke, stroke_fill=(20, 4, 6, 255))
        xx += cw + spacing * font.size
    return y + font.size


def main():
    raw = Image.open(SRC)
    W, H = raw.size
    cast = grade(key_out(raw))

    # --- the sky ---------------------------------------------------------------------------------------------
    sky = Image.new("RGBA", (W, H))
    top, mid, bot = (6, 3, 14), (30, 8, 38), (10, 4, 16)
    px = []
    for y in range(H):
        t = y / H
        c = tuple(int(a + (b - a) * (t / 0.55)) for a, b in zip(top, mid)) if t < 0.55 else \
            tuple(int(a + (b - a) * ((t - 0.55) / 0.45)) for a, b in zip(mid, bot))
        px.append(c)
    sky_d = ImageDraw.Draw(sky)
    for y, c in enumerate(px):
        sky_d.line([(0, y), (W, y)], fill=(*c, 255))

    moon_c = (W * 0.5, H * 0.365)
    sky.alpha_composite(glow((W, H), moon_c, W * 0.55, (130, 25, 95), 110))
    sky.alpha_composite(glow((W, H), moon_c, W * 0.30, (255, 90, 50), 130))
    sky.alpha_composite(glow((W, H), (W * 0.5, H * 0.62), W * 0.45, (110, 40, 190), 70))
    sky.alpha_composite(kaleidoscope((W, H), moon_c, W * 0.40))
    # the moon itself
    d = ImageDraw.Draw(sky)
    mr = W * 0.10
    moon = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    md = ImageDraw.Draw(moon)
    md.ellipse([moon_c[0] - mr, moon_c[1] - mr, moon_c[0] + mr, moon_c[1] + mr], fill=(255, 196, 140, 255))
    spots = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    sp = ImageDraw.Draw(spots)
    for _ in range(14):   # soft darker patches, like a blood moon's seas
        a, r = random.uniform(0, 6.28), random.uniform(0, mr * 0.8)
        cx, cy, rr = moon_c[0] + math.cos(a) * r, moon_c[1] + math.sin(a) * r, random.uniform(mr * 0.08, mr * 0.22)
        sp.ellipse([cx - rr, cy - rr, cx + rr, cy + rr], fill=(190, 80, 60, 90))
    spots = spots.filter(ImageFilter.GaussianBlur(mr * 0.06))
    clip = Image.new("L", (W, H), 0)
    ImageDraw.Draw(clip).ellipse([moon_c[0] - mr, moon_c[1] - mr, moon_c[0] + mr, moon_c[1] + mr], fill=255)
    spots.putalpha(ImageChops.multiply(spots.split()[-1], clip))
    moon.alpha_composite(spots)
    shade = Image.new("RGBA", (W, H), (60, 5, 20, 0))   # darker towards the lower right limb
    shade.putalpha(ImageChops.multiply(glow((W, H), (moon_c[0] + mr * 0.9, moon_c[1] + mr * 0.9), mr * 1.1, (0, 0, 0), 150).split()[-1], clip).point(lambda v: int(v * 0.6)))
    moon.alpha_composite(shade)
    sky.alpha_composite(glow((W, H), moon_c, mr * 1.6, (255, 150, 90), 150))
    sky.alpha_composite(moon)
    # stars and embers
    stars = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    sd = ImageDraw.Draw(stars)
    for _ in range(260):
        x, y = random.randint(0, W), random.randint(0, int(H * 0.6))
        r = random.choice([1, 1, 1, 2]) * W / 1200
        sd.ellipse([x - r, y - r, x + r, y + r], fill=(255, 235, 200, random.randint(60, 200)))
    sky.alpha_composite(stars.filter(ImageFilter.GaussianBlur(0.6)))
    sky.alpha_composite(mist((W, H), H * 0.55, H * 0.75, (120, 70, 160), 70, 1))

    # --- the ground shadow and the cast ----------------------------------------------------------------------
    bbox = cast.split()[-1].point(lambda v: 255 if v > 40 else 0).getbbox() or (0, 0, W, H)
    sh = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    sd = ImageDraw.Draw(sh)
    sd.ellipse([bbox[0] - W * 0.05, bbox[3] - H * 0.09, bbox[2] + W * 0.05, bbox[3] + H * 0.035], fill=(0, 0, 0, 200))
    sky.alpha_composite(sh.filter(ImageFilter.GaussianBlur(H / 70)))
    # a thin rim of light around the cast, so it separates from the sky
    a = cast.split()[-1]
    rim = ImageChops.subtract(a.filter(ImageFilter.MaxFilter(5)).filter(ImageFilter.GaussianBlur(3)), a)
    rim_layer = Image.new("RGBA", (W, H), (255, 205, 120, 0))
    rim_layer.putalpha(rim.point(lambda v: int(v * 0.55)))
    sky.alpha_composite(rim_layer)
    sky.alpha_composite(cast)
    sky.alpha_composite(mist((W, H), H * 0.80, H * 0.97, (150, 90, 190), 80, 2))

    # --- vignette ---------------------------------------------------------------------------------------------
    vg = Image.new("L", (W, H), 0)
    ImageDraw.Draw(vg).ellipse([-W * 0.25, -H * 0.12, W * 1.25, H * 1.12], fill=255)
    vg = vg.filter(ImageFilter.GaussianBlur(W * 0.14))
    dark = Image.new("RGBA", (W, H), (0, 0, 0, 255))
    dark.putalpha(ImageChops.invert(vg).point(lambda v: int(v * 0.75)))
    sky.alpha_composite(dark)

    # --- type -------------------------------------------------------------------------------------------------
    title = ImageFont.truetype(FONT_B, int(W * 0.148))
    sub = ImageFont.truetype(FONT_B, int(W * 0.040))
    tag = ImageFont.truetype(FONT_R, int(W * 0.030))
    small = ImageFont.truetype(FONT_R, int(W * 0.021))
    names = ImageFont.truetype(FONT_B, int(W * 0.0235))

    y = H * 0.035
    y = text_center(sky, "A  STEALTH  HORROR  GAME", y, small, (230, 200, 150, 235), spacing=0.12)
    y = text_center(sky, "WOUT", H * 0.062, title, (255, 232, 190, 255), glow_col=(210, 30, 40), stroke=3, spacing=0.02)
    y = text_center(sky, "SAN RETOU", y - title.size * 0.10, title, (255, 232, 190, 255), glow_col=(210, 30, 40), stroke=3, spacing=0.02)
    y = text_center(sky, "THE ROAD OF NO RETURN", y + H * 0.004, sub, (235, 190, 95, 255), glow_col=(120, 20, 30), stroke=1, spacing=0.10)

    base = H * 0.900
    text_center(sky, "SNEAK  •  WAKE THE LOST  •  SURVIVE THE NIGHT", base, tag, (240, 225, 200, 245), glow_col=(0, 0, 0), spacing=0.06)
    text_center(sky, "RO   ·   GRANMET NWIT   ·   MDANM FLANM   ·   TET WOCH   ·   ZOMBI MANCHET",
                base + H * 0.042, names, (235, 190, 95, 255), glow_col=(0, 0, 0), spacing=0.03)
    text_center(sky, "All characters, places and events are fictional.  Violence has a cost.",
                H * 0.972, small, (190, 170, 160, 220))

    sky.convert("RGB").save(OUT)
    print("wrote", OUT, sky.size)


if __name__ == "__main__":
    main()
