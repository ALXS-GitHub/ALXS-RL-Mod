"""Find every Material/MIC sub-expression or TextureParameterValue across
Startup.upk + Body_Octane_SF.upk + Body_Octane_T_SF.upk that references
Octane_Body_D, Pepe_Body_D, or any *_Body_D / *_Body_BlankSkin texture.

This tells us which materials/MICs actually SAMPLE the body diffuse layer.
"""

import sys, struct
sys.path.insert(0, str(__import__("pathlib").Path(__file__).parent))
from diag_body_diffuse_final import Upk
from pathlib import Path

RL = Path(r"C:/Program Files/Epic Games/rocketleague/TAGame/CookedPCConsole")


def collect_textures_with_body_in_name(upk):
    t2d_idx = None
    for imp in upk.imports:
        if imp["name"] == "Texture2D":
            t2d_idx = -(imp["idx"]+1); break
    out = []
    for e in upk.exports:
        if e["class"] == t2d_idx:
            nm = e["name"].lower()
            if "body" in nm or "chasis" in nm:
                out.append((e["idx"], e["name"]))
    # also imports
    for imp in upk.imports:
        if imp["class"] == "Texture2D":
            nm = imp["name"].lower()
            if "body" in nm or "chasis" in nm:
                out.append((-(imp["idx"]+1), imp["name"]))
    return out


def find_subexpression_users(upk, target_obj_indices, label):
    """Scan ALL MaterialExpression sub-exports for ObjectProperty references to target indices."""
    print(f"\n--- {label} ({upk.path.name}) ---")
    print(f"  target object indices: {target_obj_indices}")
    obj_prop = upk.names.index("ObjectProperty") if "ObjectProperty" in upk.names else -1
    if obj_prop < 0: return
    hits = []
    for e in upk.exports:
        cls = upk.class_name(e["class"])
        if not cls.startswith("MaterialExpression") and cls not in ("MaterialInstanceConstant","Material"):
            continue
        body_pos = e["so"] - upk.total_header
        serial = bytes(upk.body[body_pos:body_pos+e["ss"]])
        # Scan for ObjectProperty entries (16-byte type FName + val_size=4 + arr_idx + i32 val)
        for k in range(0, len(serial) - 28, 4):
            n_idx = struct.unpack_from("<i", serial, k)[0]
            n_num = struct.unpack_from("<i", serial, k+4)[0]
            t_idx = struct.unpack_from("<i", serial, k+8)[0]
            if t_idx != obj_prop: continue
            if not (0 <= n_idx < len(upk.names)): continue
            if n_num != 0: continue
            val_size = struct.unpack_from("<i", serial, k+16)[0]
            if val_size != 4: continue
            v = struct.unpack_from("<i", serial, k+24)[0]
            if v in target_obj_indices:
                tag_name = upk.names[n_idx]
                hits.append({"export": e["idx"], "export_name": e["name"], "class": cls, "tag_name": tag_name, "offset_in_export": k, "target": v})
                print(f"  hit: export[{e['idx']}] {e['name']} ({cls}) tag={tag_name!r} -> {upk.resolve(v)}")
                break  # one hit per sub-export is enough
    if not hits:
        print(f"  no hits")
    return hits


def main():
    print("Loading UPKs...")
    startup = Upk(RL/"Startup.upk", preamble_override=0)
    body_octane = Upk(RL/"Body_Octane_SF.upk")
    if (RL/"Body_Octane_T_SF.upk").exists():
        body_octane_t = Upk(RL/"Body_Octane_T_SF.upk")
    else:
        body_octane_t = None

    for upk, label in [(startup, "Startup"), (body_octane, "Body_Octane_SF"), (body_octane_t, "Body_Octane_T_SF")]:
        if upk is None: continue
        textures = collect_textures_with_body_in_name(upk)
        print(f"\n{label}: body-related Texture2D candidates ({len(textures)}):")
        for tidx, nm in textures:
            print(f"  {tidx:6}: {nm}")
        # convert export indices to ObjectIndex notation (positive = export+1)
        target_obj_indices = set()
        for tidx, nm in textures:
            if tidx >= 0:
                target_obj_indices.add(tidx + 1)
            else:
                target_obj_indices.add(tidx)  # negative for imports
        find_subexpression_users(upk, target_obj_indices, f"{label}: who references body textures")


if __name__ == "__main__":
    main()
