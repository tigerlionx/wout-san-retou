"""Rename Mixamo bones "mixamorig5:Hips" -> "mixamorig:Hips" in converted GLBs.

  python3 tools/glb_fix_prefix.py assets/mixamo name1 name2 ...

Mixamo numbers the bone prefix when a character was downloaded after others in one session;
the game's skeleton uses plain "mixamorig:", and Bevy matches animation targets by bone name.
"""
import json, re, struct, sys

d, names = sys.argv[1], sys.argv[2:]
for name in names:
    p = f"{d}/{name}.glb"
    b = open(p, "rb").read()
    n = struct.unpack("<I", b[12:16])[0]
    j = json.loads(b[20:20 + n])
    changed = 0
    for nd in j["nodes"]:
        if "name" in nd:
            new = re.sub(r"^mixamorig\d+:", "mixamorig:", nd["name"])
            changed += new != nd["name"]
            nd["name"] = new
    js = json.dumps(j, separators=(",", ":")).encode()
    js += b" " * (-len(js) % 4)
    rest = b[20 + n:]
    out = b[:12] + struct.pack("<I", len(js)) + b"JSON" + js + rest
    out = out[:8] + struct.pack("<I", len(out)) + out[12:]
    open(p, "wb").write(out)
    print(name, "renamed", changed)
