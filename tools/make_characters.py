"""Build the game's cast from the Mixamo character FBX files.

  Blender -b -P tools/make_characters.py -- SRC_DIR OUT_DIR [name ...]

SRC_DIR holds the downloaded FBX files (see CAST below for the file names). Each
character is cleaned (bone names normalised to "mixamorig:*" so every clip plays on
it, textures shrunk to 1024 px, props removed), re-dressed where the story asks
(dreadlock bun, swimsuit, cowboy hat...) and exported to OUT_DIR/<name>.glb.
A cast.json with each character's height and hip height is written beside them; the
game uses it to fit the shared animation clips to every body.
"""
import bpy, bmesh, sys, os, json, math, re
import mathutils
from mathutils import Vector
import numpy as np

args = sys.argv[sys.argv.index("--") + 1:]
SRC, DST = args[0], args[1]
ONLY = set(args[2:])
os.makedirs(DST, exist_ok=True)

# name -> (source file, builder function name)
CAST = {
    "hero":     ("Swat.fbx", "build_hero"),
    "nwit":     ("Ch10_nonPBR.fbx", "build_nwit"),
    "flanm":    ("Ch26_nonPBR.fbx", "build_flanm"),
    "woch":     ("Ch32_nonPBR.fbx", "build_woch"),
    "manchet":  ("Paladin WProp J Nordstrom.fbx", "build_manchet"),
    "gang_a":   ("Remy.fbx", "build_gang_a"),
    "gang_b":   ("Ch06_nonPBR.fbx", "build_gang_b"),
    "brute":    ("Brute.fbx", "build_brute"),
    "yaku":     ("Yaku J Ignite.fbx", "build_yaku"),
    "prisoner": ("Prisoner B Styperek.fbx", "build_prisoner"),
}


# ----------------------------------------------------------------- helpers

def meshes():
    return [o for o in bpy.data.objects if o.type == "MESH"]


def armature():
    return [o for o in bpy.data.objects if o.type == "ARMATURE"][0]


def normalise_bones():
    """mixamorig5:Hips -> mixamorig:Hips (bones and the vertex groups that follow them)."""
    arm = armature()
    pat = re.compile(r"^mixamorig\d*:")
    for b in arm.data.bones:
        n = pat.sub("mixamorig:", b.name)
        if n != b.name:
            b.name = n
    for o in meshes():
        for vg in o.vertex_groups:
            n = pat.sub("mixamorig:", vg.name)
            if n != vg.name:
                vg.name = n


def shrink_textures(limit=1024):
    for img in bpy.data.images:
        try:
            w, h = img.size
        except Exception:
            continue
        if w > limit or h > limit:
            k = limit / max(w, h)
            img.scale(max(1, int(w * k)), max(1, int(h * k)))


def drop(*names):
    for o in list(meshes()):
        if o.name in names:
            bpy.data.objects.remove(o, do_unlink=True)


def tint_image(img, fn):
    """fn maps an (n,3) float array of rgb to a new (n,3) array."""
    w, h = img.size
    px = np.empty(w * h * 4, dtype=np.float32)
    img.pixels.foreach_get(px)
    px = px.reshape(-1, 4)
    px[:, :3] = np.clip(fn(px[:, :3]), 0.0, 1.0)
    img.pixels.foreach_set(px.reshape(-1))
    img.update()


def images_of(obj):
    out = []
    for m in obj.data.materials:
        if m and m.use_nodes:
            for n in m.node_tree.nodes:
                if n.type == "TEX_IMAGE" and n.image and n.image not in out:
                    out.append(n.image)
    return out


def base_color_images(obj):
    """Only the images that feed Base Color (not normal/specular maps)."""
    out = []
    for m in obj.data.materials:
        if not (m and m.use_nodes):
            continue
        for n in m.node_tree.nodes:
            if n.type == "BSDF_PRINCIPLED":
                s = n.inputs["Base Color"]
                for l in s.links:
                    src = l.from_node
                    seen = 0
                    while src.type != "TEX_IMAGE" and seen < 6:
                        ins = [i for i in src.inputs if i.links]
                        if not ins:
                            break
                        src = ins[0].links[0].from_node
                        seen += 1
                    if src.type == "TEX_IMAGE" and src.image and src.image not in out:
                        out.append(src.image)
    return out


def simple_material(name, rgb, rough=0.7, metal=0.0):
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    b = m.node_tree.nodes["Principled BSDF"]
    b.inputs["Base Color"].default_value = (rgb[0], rgb[1], rgb[2], 1.0)
    b.inputs["Roughness"].default_value = rough
    b.inputs["Metallic"].default_value = metal
    return m


def attach(obj, bone_name):
    """Bind a new piece to one bone (weight 1) like the originals."""
    arm = armature()
    vg = obj.vertex_groups.new(name=bone_name)
    vg.add([v.index for v in obj.data.vertices], 1.0, "REPLACE")
    mod = obj.modifiers.new("Armature", "ARMATURE")
    mod.object = arm
    obj.parent = arm
    obj.matrix_parent_inverse = arm.matrix_world.inverted()


def new_mesh_object(name, bm_or_fn, mat):
    me = bpy.data.meshes.new(name)
    bm = bmesh.new()
    bm_or_fn(bm)
    bm.to_mesh(me)
    bm.free()
    ob = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(ob)
    me.materials.append(mat)
    for p in me.polygons:
        p.use_smooth = True
    return ob


def head_info(skin_obj):
    """Top of the skull and its centre, from the vertices above the neck."""
    arm = armature()
    head = arm.data.bones["mixamorig:Head"]
    hz = (arm.matrix_world @ head.head_local).z
    pts = []
    for v in skin_obj.data.vertices:
        p = skin_obj.matrix_world @ v.co
        if p.z > hz + 0.03:
            pts.append(p)
    top = max(p.z for p in pts)
    near = [p for p in pts if p.z > top - 0.04]
    cx = sum(p.x for p in near) / len(near)
    cy = sum(p.y for p in near) / len(near)
    # head width: widest x among vertices in the middle of the head
    mid = [p for p in pts if abs(p.z - (hz + (top - hz) * 0.55)) < 0.03]
    half_w = max(abs(p.x) for p in mid) if mid else 0.08
    return Vector((cx, cy, top)), half_w, hz


def smooth(a, b, x):
    t = min(1.0, max(0.0, (x - a) / (b - a)))
    return t * t * (3 - 2 * t)


# ------------------------------------------------------------ accessories

def cowboy_hat(top, half_w, hat_rgb, band_rgb):
    """A felt cowboy hat sitting on the skull. `top` is the highest point of the head."""
    scale = half_w / 0.085
    crown_r, crown_h, brim_r = 0.098 * scale, 0.115 * scale, 0.235 * scale
    base_z = top.z - 0.045 * scale
    cx, cy = top.x, top.y

    def crown(bm):
        # a tapered tube with a pinched, dented top
        rings = 8
        segs = 28
        verts = []
        for r in range(rings + 1):
            t = r / rings
            rad = crown_r * (1.0 - 0.10 * t)
            z = base_z + crown_h * t
            ring = []
            for s in range(segs):
                a = 2 * math.pi * s / segs
                rr = rad
                if t > 0.86:  # the crease down the middle of the crown
                    rr *= 1.0 - 0.12 * math.cos(2 * a) * smooth(0.86, 1.0, t)
                zz = z - (0.015 * scale if (t > 0.9 and abs(math.sin(a)) < 0.45) else 0)
                ring.append(bm.verts.new((cx + rr * math.cos(a), cy + rr * 1.08 * math.sin(a), zz)))
            verts.append(ring)
        for r in range(rings):
            for s in range(segs):
                bm.faces.new([verts[r][s], verts[r][(s + 1) % segs], verts[r + 1][(s + 1) % segs], verts[r + 1][s]])
        cap = bm.verts.new((cx, cy, base_z + crown_h - 0.012 * scale))
        for s in range(segs):
            bm.faces.new([verts[rings][s], verts[rings][(s + 1) % segs], cap])

    def brim(bm):
        rings, segs = 6, 36
        verts = []
        for r in range(rings + 1):
            t = r / rings
            rad = crown_r * 0.95 + (brim_r - crown_r * 0.95) * t
            ring = []
            for s in range(segs):
                a = 2 * math.pi * s / segs
                x, y = math.cos(a), math.sin(a)
                # the sides curl up, front and back stay lower: the cowboy shape
                curl = 0.06 * scale * (t ** 2.2) * (0.35 + 0.65 * abs(x) ** 1.5) + 0.012 * scale * t
                dip = -0.022 * scale * (t ** 2) * (y * y) * (1 if y < 0 else 0.4)
                ring.append(bm.verts.new((cx + rad * x, cy + rad * 1.12 * y, base_z + curl + dip)))
            verts.append(ring)
        for r in range(rings):
            for s in range(segs):
                bm.faces.new([verts[r][s], verts[r][(s + 1) % segs], verts[r + 1][(s + 1) % segs], verts[r + 1][s]])
        # give it some thickness by a second, slightly lower skin
        bm.faces.ensure_lookup_table()

    def band(bm):
        segs = 28
        rings = []
        for z0 in (base_z + 0.004 * scale, base_z + 0.034 * scale):
            ring = []
            for s in range(segs):
                a = 2 * math.pi * s / segs
                rr = crown_r * 1.025 * (1.0 - 0.10 * ((z0 - base_z) / crown_h))
                ring.append(bm.verts.new((cx + rr * math.cos(a), cy + rr * 1.08 * math.sin(a), z0)))
            rings.append(ring)
        for s in range(segs):
            bm.faces.new([rings[0][s], rings[0][(s + 1) % segs], rings[1][(s + 1) % segs], rings[1][s]])

    hat_mat = simple_material("hat_felt", hat_rgb, rough=0.9)
    band_mat = simple_material("hat_band", band_rgb, rough=0.35, metal=0.6)
    for name, fn, mat in (("hat_crown", crown, hat_mat), ("hat_brim", brim, hat_mat), ("hat_band", band, band_mat)):
        ob = new_mesh_object(name, fn, mat)
        if name == "hat_brim":
            sol = ob.modifiers.new("thick", "SOLIDIFY")
            sol.thickness = 0.006 * scale
            sol.offset = -1
            bpy.context.view_layer.objects.active = ob
            bpy.ops.object.modifier_apply(modifier="thick")
        attach(ob, "mixamorig:Head")


def dreadlock_bun(top, half_w, hz, rgb, count=24):
    """Dreadlocks gathered, folded over and tied into a round bun on the crown: each rope
    runs over the top of a ball, so they cross like a knot of folded locks."""
    scale = half_w / 0.085
    mat = simple_material("dreads", rgb, rough=0.85)
    cx, cy = top.x, top.y
    R = 0.105 * scale
    cz = top.z - 0.18 * R
    z0 = top.z - 0.03 * scale
    objs = []
    import random
    rnd = random.Random(7)
    for i in range(count):
        phi = math.pi * (i + rnd.uniform(-0.2, 0.2)) / count
        rr = R * rnd.uniform(0.92, 1.12)
        tmax = rnd.uniform(1.15, 1.35)
        pts = []
        for k in range(9):
            t = -tmax + 2 * tmax * k / 8
            pts.append((cx + rr * math.sin(t) * math.cos(phi) * 1.05,
                        cy + rr * math.sin(t) * math.sin(phi) * 1.05,
                        cz + rr * math.cos(t) * 1.0 - 0.1 * R * (abs(t) / tmax) ** 2))
        cu = bpy.data.curves.new(f"dread{i}", "CURVE")
        cu.dimensions = "3D"
        cu.bevel_depth = 0.0125 * scale
        cu.bevel_resolution = 2
        cu.use_fill_caps = True
        sp = cu.splines.new("NURBS")
        sp.points.add(len(pts) - 1)
        for p, c in zip(sp.points, pts):
            p.co = (c[0], c[1], c[2], 1.0)
        sp.order_u = 4
        sp.use_endpoint_u = True
        sp.resolution_u = 5
        ob = bpy.data.objects.new(f"dread{i}", cu)
        bpy.context.scene.collection.objects.link(ob)
        objs.append(ob)
    # the tie: a cord wrapped round the base of the knot
    def cord(bm):
        segs = 24
        for k, dz in enumerate((0.0, 0.016)):
            pass
        rings = []
        for dz in (0.0, 0.022 * scale):
            ring = []
            for s in range(segs):
                a = 2 * math.pi * s / segs
                rr = 0.074 * scale
                ring.append(bm.verts.new((cx + rr * math.cos(a), cy + rr * math.sin(a), top.z - 0.020 * scale + dz)))
            rings.append(ring)
        for s in range(segs):
            bm.faces.new([rings[0][s], rings[0][(s + 1) % segs], rings[1][(s + 1) % segs], rings[1][s]])
    band = new_mesh_object("bun_cord", cord, simple_material("cord", (0.55, 0.12, 0.08), rough=0.6))
    # convert the curves to meshes and merge them
    bpy.ops.object.select_all(action="DESELECT")
    for ob in objs:
        ob.select_set(True)
    bpy.context.view_layer.objects.active = objs[0]
    bpy.ops.object.convert(target="MESH")
    bpy.ops.object.join()
    joined = bpy.context.view_layer.objects.active
    joined.data.materials.clear()
    joined.data.materials.append(mat)
    for p in joined.data.polygons:
        p.use_smooth = True
    joined.name = "dreadlocks"
    attach(joined, "mixamorig:Head")
    attach(band, "mixamorig:Head")
    return joined


# ----------------------------------------------------------- body shaping

def shape_body(obj, fn):
    """Move every vertex through fn(point) -> point (rest pose, world metres)."""
    mw = obj.matrix_world
    inv = mw.inverted()
    for v in obj.data.vertices:
        p = mw @ v.co
        v.co = inv @ fn(p)
    obj.data.update()


def bulk(chest=1.0, belly=1.0, hips=1.0, thigh=1.0, upper_arm=1.0, neck=1.0, shoulder_x=1.0, calf=1.0, forearm=1.0,
         depth=1.0, centre_y=0.02):
    """Return a shaping function that thickens regions of a T-posed body."""
    def f(p):
        x, y, z = p.x, p.y - centre_y, p.z
        ax = abs(x)
        # --- torso (inside the arm line): scale around the spine
        torso_w = 1 - smooth(0.17, 0.30, ax)  # 1 in the torso, 0 out on the arms
        sx = 1.0
        sy = 1.0
        # height bands
        wchest = smooth(1.18, 1.30, z) * (1 - smooth(1.42, 1.52, z))
        wbelly = smooth(0.92, 1.02, z) * (1 - smooth(1.14, 1.24, z))
        whip = smooth(0.74, 0.86, z) * (1 - smooth(0.96, 1.06, z))
        k = 1.0 + (chest - 1) * wchest + (belly - 1) * wbelly + (hips - 1) * whip
        # shoulders widen the line that carries the arms
        sh = 1.0 + (shoulder_x - 1) * smooth(1.28, 1.40, z) * (1 - smooth(1.52, 1.62, z))
        x_new = x * (1 + (k - 1) * torso_w) * (1 if ax <= 0.17 else 1)
        y_new = y * (1 + (k * depth - 1) * torso_w)
        # --- neck
        if 1.46 < z < 1.62 and ax < 0.09:
            wn = smooth(1.46, 1.50, z) * (1 - smooth(1.56, 1.64, z))
            x_new *= 1 + (neck - 1) * wn
            y_new *= 1 + (neck - 1) * wn
        # --- arms: thicken around the arm axis (about shoulder height), keep their length
        wa = smooth(0.17, 0.27, ax) * (1 - smooth(0.14, 0.24, abs(z - 1.43)))
        if wa > 0.0:
            ua = (1 - smooth(0.42, 0.52, ax))  # upper arm weight fades over the elbow
            fa = (upper_arm - 1) * ua + (forearm - 1) * (1 - ua)
            y_new = y * (1 + fa * wa)
            z = 1.43 + (z - 1.43) * (1 + fa * wa)
        # --- legs: thicken around the leg axis, strongest at the thigh
        if z < 0.90 and ax > 0.01 and ax < 0.30:
            cx = math.copysign(0.09, x)
            wl_hi = smooth(0.40, 0.70, z) * (1 - smooth(0.82, 0.92, z))
            wl_lo = (1 - smooth(0.15, 0.45, z))
            kf = 1 + (thigh - 1) * wl_hi + (calf - 1) * wl_lo
            x_new = cx + (x - cx) * kf
            y_new = y * kf
        return Vector((x_new, y_new + centre_y, z))
    return f


def swimwear(shirt, pants, skin_rgb, suit_rgb):
    """Her body mesh only holds head, hands and feet; the torso and legs live in the clothes
    meshes. Re-skin those: swimsuit over the trunk, bare skin everywhere else."""
    arm = armature()
    hips = (arm.matrix_world @ arm.data.bones["mixamorig:Hips"].head_local).z
    sp2 = (arm.matrix_world @ arm.data.bones["mixamorig:Spine2"].head_local).z
    z_top, z_bot = sp2 + 0.05, hips - 0.10
    skin = simple_material("skin", skin_rgb, rough=0.6)
    suit = simple_material("swimsuit", suit_rgb, rough=0.4)
    n_suit = 0
    for o in (shirt, pants):
        o.data.materials.clear()
        o.data.materials.append(skin)   # index 0
        o.data.materials.append(suit)   # index 1
        mw = o.matrix_world
        for p in o.data.polygons:
            c = mw @ p.center
            trunk = abs(c.x) < (0.21 if c.z > hips else 0.30)
            covered = z_bot + 0.13 * min(1.0, abs(c.x) / 0.16) < c.z < z_top
            p.material_index = 1 if (trunk and covered) else 0
            n_suit += p.material_index
            p.use_smooth = True
    print("SWIM faces", n_suit)


# ----------------------------------------------------------------- builders

def build_hero():
    pass


def build_nwit():
    body = [o for o in meshes() if o.name == "Ch10"][0]
    top, half_w, hz = head_info(body)
    dreadlock_bun(top, half_w, hz, (0.06, 0.04, 0.03))
    # a fraud wearing a ceremonial cloth: darken the rags to dyed indigo
    for img in base_color_images(body):
        tint_image(img, lambda c: c * np.array([0.9, 0.95, 1.1]))


def build_flanm():
    drop("Ch26_Heels")
    body = [o for o in meshes() if o.name == "Ch26_Body"][0]
    shirt = [o for o in meshes() if o.name == "Ch26_Shirt"][0]
    pants = [o for o in meshes() if o.name == "Ch26__Pants"][0]
    skin = (0.24, 0.125, 0.08)  # linear rgb, deep brown to match her face
    fat = bulk(chest=1.20, belly=1.55, hips=1.55, thigh=1.40, upper_arm=1.40, forearm=1.20, neck=1.15, calf=1.15, depth=1.25)
    for o in (body, shirt, pants):
        shape_body(o, fat)
    swimwear(shirt, pants, skin, (0.50, 0.03, 0.10))


def build_woch():
    body = [o for o in meshes() if o.name == "Ch32"][0]
    shape_body(body, bulk(chest=1.45, belly=1.20, hips=1.10, thigh=1.30, upper_arm=1.75, forearm=1.45, neck=1.50,
                          shoulder_x=1.20, calf=1.20, depth=1.25))


def build_manchet():
    # the file's names are swapped: Nordstrom = sword, _Helmet = shield, _Shield = helmet, _Sword = the body
    drop("Paladin_J_Nordstrom_Helmet", "Paladin_J_Nordstrom_Shield")
    body = [o for o in meshes() if o.name == "Paladin_J_Nordstrom_Sword"][0]
    # black and gold ceremonial dress instead of battle steel
    def dress(c):
        lum = (c[:, 0] * 0.3 + c[:, 1] * 0.59 + c[:, 2] * 0.11)[:, None]
        mx, mn = c.max(axis=1), c.min(axis=1)
        d = np.maximum(mx - mn, 1e-5)
        sat = ((mx - mn) / np.maximum(mx, 1e-4))[:, None]
        r, g, b = c[:, 0], c[:, 1], c[:, 2]
        h = np.where(mx == r, ((g - b) / d) % 6, np.where(mx == g, (b - r) / d + 2, (r - g) / d + 4)) * 60.0
        h = h[:, None]
        suit = np.array([0.05, 0.045, 0.075]) + lum * np.array([0.40, 0.34, 0.52])
        gold = np.array([0.95, 0.72, 0.18]) * (0.45 + lum)
        skin = np.array([0.50, 0.60, 0.48]) * (0.35 + lum * 1.2)
        is_gold = ((h > 38) & (h < 66) & (sat > 0.72)).astype(np.float32)
        is_skin = ((h >= 5) & (h <= 60) & (sat > 0.14) & (sat <= 0.72) & (lum > 0.2)).astype(np.float32)
        out = suit * (1 - is_gold) + gold * is_gold
        return out * (1 - is_skin) + skin * is_skin
    for img in base_color_images(body):
        tint_image(img, dress)
    top = Vector((0.0, 0.02, 1.70))
    cowboy_hat(top, 0.085, (0.05, 0.04, 0.04), (0.85, 0.65, 0.15))


def _remy_common():
    body = [o for o in meshes() if o.name == "Body"][0]
    return body


def build_gang_a():
    pass


def build_gang_b():
    # James (Ch06): used as downloaded, the second gang shooter
    pass


def build_brute():
    drop("BattleAxe_GEO")
    # a hulking man the sickness has already half taken: greyer, colder skin
    for o in meshes():
        if o.name in ("MaleBruteA_Body",):
            for img in base_color_images(o):
                tint_image(img, lambda c: c * np.array([0.78, 0.86, 0.80]))


def build_yaku():
    pass


def build_prisoner():
    pass


# ------------------------------------------------------------------- driver

def height_of_body():
    lo, hi = 1e9, -1e9
    for o in meshes():
        if o.name in ("Paladin_J_Nordstrom", ) or o.name.startswith("hat") or o.name == "dreadlocks":
            continue
        for v in o.data.vertices:
            z = (o.matrix_world @ v.co).z
            lo, hi = min(lo, z), max(hi, z)
    return lo, hi


def export(name):
    out = os.path.join(DST, name + ".glb")
    for a in bpy.data.actions:
        a.name = name
    bpy.ops.export_scene.gltf(filepath=out, export_format="GLB", export_animations=True, export_yup=True,
                              export_force_sampling=True, export_image_format="AUTO")
    return out


cast_info = {}
cast_path = os.path.join(DST, "cast.json")
if os.path.exists(cast_path):
    cast_info = json.load(open(cast_path))

for name, (fn, builder) in CAST.items():
    if ONLY and name not in ONLY:
        continue
    path = os.path.join(SRC, fn)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.fbx(filepath=path)
    normalise_bones()
    shrink_textures()
    globals()[builder]()
    arm = armature()
    hips = (arm.matrix_world @ arm.data.bones["mixamorig:Hips"].head_local).z
    # height of the body only (hats and props excluded)
    body_names = [o for o in meshes() if not (o.name.startswith("hat") or o.name in ("dreadlocks", "bun_cord", "swimsuit"))]
    lo, hi = 1e9, -1e9
    for o in body_names:
        if o.name == "Paladin_J_Nordstrom":
            continue
        for v in o.data.vertices:
            z = (o.matrix_world @ v.co).z
            lo, hi = min(lo, z), max(hi, z)
    cast_info[name] = {"height": round(hi - lo, 3), "hips_cm": round(hips * 100, 2)}
    export(name)
    print("BUILT", name, cast_info[name])

json.dump(cast_info, open(cast_path, "w"), indent=1)
