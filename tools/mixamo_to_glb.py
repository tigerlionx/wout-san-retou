"""Convert Mixamo FBX downloads to GLB for Bevy.

  Blender -b -P tools/mixamo_to_glb.py -- SRC_DIR OUT_DIR tools/mixamo_map.json

SRC_DIR holds the FBX files (zip contents flattened as "<Pack>__<file>.fbx").
mixamo_map.json maps "source file" -> "output name": "character" entries keep their
mesh, "clips" entries keep only the armature and its animation. All Mixamo rigs share
the bone names "mixamorig:Hips" ..., so any clip plays on any character.
"""
import bpy, sys, os, json

src, dst, mapf = sys.argv[sys.argv.index("--") + 1:][:3]
mp = json.load(open(mapf))
os.makedirs(dst, exist_ok=True)

def convert(fn, name, keep_mesh):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.fbx(filepath=os.path.join(src, fn))
    if not keep_mesh:
        for o in [o for o in bpy.data.objects if o.type == "MESH"]:
            bpy.data.objects.remove(o, do_unlink=True)
    for a in bpy.data.actions:
        a.name = name
    out = os.path.join(dst, name + ".glb")
    bpy.ops.export_scene.gltf(filepath=out, export_format="GLB", export_animations=True,
                              export_yup=True, export_force_sampling=True)
    print("converted", fn, "->", out)

for fn, name in mp["character"].items():
    convert(fn, name, True)
for fn, name in mp["clips"].items():
    convert(fn, name, False)
