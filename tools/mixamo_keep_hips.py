"""Strip every translation (and scale) track from the clip GLBs except the Hips.

Mixamo clips bake translation keys for every bone, which carry the proportions of
the character they were made on. Left in, they squash or stretch any other body to
that skeleton (a 2 m Remy shrinks to a child). Rotations plus the Hips translation
(root height and sway) are all a clip needs.

  python3 tools/mixamo_keep_hips.py assets/mixamo [clip ...]
"""
import json, struct, sys, os

folder = sys.argv[1]
only = set(sys.argv[2:])
CHARS = {"hero", "nwit", "flanm", "woch", "manchet", "gang_a", "gang_b", "brute", "yaku", "prisoner", "xbot"}
for fn in sorted(os.listdir(folder)):
    if not fn.endswith(".glb") or fn[:-4] in CHARS or (only and fn[:-4] not in only):
        continue
    path = os.path.join(folder, fn)
    b = open(path, "rb").read()
    jl = struct.unpack("<I", b[12:16])[0]
    j = json.loads(b[20:20 + jl])
    rest = b[20 + jl:]
    hips = {i for i, n in enumerate(j["nodes"]) if n.get("name", "").endswith("Hips")}
    n_before = sum(len(a["channels"]) for a in j.get("animations", []))
    for a in j.get("animations", []):
        a["channels"] = [c for c in a["channels"]
                         if c["target"]["path"] == "rotation" or (c["target"]["path"] == "translation" and c["target"].get("node") in hips)]
    n_after = sum(len(a["channels"]) for a in j.get("animations", []))
    js = json.dumps(j, separators=(",", ":")).encode()
    js += b" " * ((4 - len(js) % 4) % 4)
    out = b"glTF" + struct.pack("<II", 2, 12 + 8 + len(js) + len(rest)) + struct.pack("<I", len(js)) + b"JSON" + js + rest
    open(path, "wb").write(out)
    print(fn, n_before, "->", n_after)
