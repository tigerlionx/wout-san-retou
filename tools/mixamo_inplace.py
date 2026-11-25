"""Remove root-motion drift from Mixamo clips so they play in place.

  python3 tools/mixamo_inplace.py assets/mixamo walk run strafe_left ...

Mixamo bakes forward travel into the Hips translation (local x/y = horizontal, z = height).
For each clip we subtract the straight line from the first to the last frame from x and y,
which keeps the sway and bounce but leaves the body where it started. One-shot clips that
should actually move the body (deaths, get_up) are left alone.
"""
import json, struct, sys, array

d, names = sys.argv[1], sys.argv[2:]
for name in names:
    p = f"{d}/{name}.glb"
    b = bytearray(open(p, "rb").read())
    n = struct.unpack("<I", b[12:16])[0]
    j = json.loads(b[20:20 + n])
    m = 20 + n
    bin_start = m + 8
    hips = next(i for i, nd in enumerate(j["nodes"]) if nd["name"] == "mixamorig:Hips")
    an = j["animations"][0]
    for ch in an["channels"]:
        if ch["target"]["node"] == hips and ch["target"]["path"] == "translation":
            acc = j["accessors"][an["samplers"][ch["sampler"]]["output"]]
            bv = j["bufferViews"][acc["bufferView"]]
            off = bin_start + bv.get("byteOffset", 0) + acc.get("byteOffset", 0)
            cnt = acc["count"]
            a = array.array("f")
            a.frombytes(bytes(b[off:off + cnt * 12]))
            for axis in (0, 1):
                first, last = a[axis], a[(cnt - 1) * 3 + axis]
                for i in range(cnt):
                    a[i * 3 + axis] -= (last - first) * i / (cnt - 1)
            b[off:off + cnt * 12] = a.tobytes()
            print("in-place", name, "frames", cnt)
    open(p, "wb").write(b)
