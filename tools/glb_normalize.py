"""Make a converted Mixamo clip play correctly on every character.

  python3 tools/glb_normalize.py assets/mixamo name1 name2 ...

Clips downloaded from other characters carry a translation (and scale) track for every bone,
which overrides the target character's limb lengths (stretched, thin limbs) and can come from a
rig twice the size. The original clips only animate rotations plus the Hips translation. This:
  * drops translation and scale tracks on every bone except the Hips translation,
  * scales the Hips translation by (X Bot leg length / the clip's leg length), so the body height matches.
Run it once per clip, after tools/glb_fix_prefix.py and before tools/mixamo_inplace.py is fine either way
(it only scales, so a drift that is already zero stays zero). Re-running a normalised clip does nothing.
"""
import array, json, struct, sys

XBOT_LEG = 44.371
d, names = sys.argv[1], sys.argv[2:]
for name in names:
    p = f"{d}/{name}.glb"
    b = bytearray(open(p, "rb").read())
    n = struct.unpack("<I", b[12:16])[0]
    j = json.loads(b[20:20 + n])
    bin_start = 20 + n + 8
    nodes = j["nodes"]
    idx = {x.get("name"): i for i, x in enumerate(nodes)}
    hips, leg = idx["mixamorig:Hips"], idx["mixamorig:LeftLeg"]
    an = j["animations"][0]

    def track(node, path):
        for c in an["channels"]:
            if c["target"].get("node") == node and c["target"]["path"] == path:
                return an["samplers"][c["sampler"]]["output"]
        return None

    leg_out = track(leg, "translation")
    scale = 1.0
    if leg_out is not None:
        acc = j["accessors"][leg_out]
        bv = j["bufferViews"][acc["bufferView"]]
        off = bin_start + bv.get("byteOffset", 0) + acc.get("byteOffset", 0)
        v = array.array("f"); v.frombytes(bytes(b[off:off + 12]))
        scale = XBOT_LEG / (v[0] ** 2 + v[1] ** 2 + v[2] ** 2) ** 0.5
        h_out = track(hips, "translation")
        if h_out is not None and abs(scale - 1.0) > 1e-4:
            acc = j["accessors"][h_out]
            bv = j["bufferViews"][acc["bufferView"]]
            off = bin_start + bv.get("byteOffset", 0) + acc.get("byteOffset", 0)
            a = array.array("f"); a.frombytes(bytes(b[off:off + acc["count"] * 12]))
            for i in range(len(a)):
                a[i] *= scale
            b[off:off + acc["count"] * 12] = a.tobytes()
            acc["min"] = [x * scale for x in acc["min"]] if "min" in acc else None
            acc["max"] = [x * scale for x in acc["max"]] if "max" in acc else None
            for k in ("min", "max"):
                if acc.get(k) is None:
                    acc.pop(k, None)
    before = len(an["channels"])
    an["channels"] = [c for c in an["channels"]
                      if c["target"]["path"] == "rotation" or (c["target"]["path"] == "translation" and c["target"].get("node") == hips)]
    js = json.dumps(j, separators=(",", ":")).encode()
    js += b" " * (-len(js) % 4)
    out = bytes(b[:12]) + struct.pack("<I", len(js)) + b"JSON" + js + bytes(b[20 + n:])
    out = out[:8] + struct.pack("<I", len(out)) + out[12:]
    open(p, "wb").write(out)
    print(f"{name}: scale {scale:.3f}, channels {before} -> {len(an['channels'])}")
