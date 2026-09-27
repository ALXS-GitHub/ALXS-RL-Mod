"""Parse the MaterialInstanceConstant exports in the modded UPK and dump
their TextureParameterValues. Verifies that the MIC's `Diffuse` param
really binds to `Force_Body_D` (and what other params exist that we
might have missed)."""

import base64, struct, zlib
from pathlib import Path
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes

UPK = Path(r"C:/Program Files/Epic Games/rocketleague/TAGame/CookedPCConsole/mods/Skin_Octane_Stars_SF.upk")
KEYS = Path(__file__).parent.parent.parent / "src-tauri/resources/decryptor/keys.txt"

UPK_MAGIC = 0x9E2A83C1

def aes_decrypt(data, key):
    return Cipher(algorithms.AES(key), modes.ECB()).decryptor().update(data) + b""

buf = UPK.read_bytes()
total_header = struct.unpack_from("<I", buf, 8)[0]
folder_len = abs(struct.unpack_from("<i", buf, 12)[0])
p = 16 + folder_len + 4
name_count = struct.unpack_from("<I", buf, p)[0]; p += 4
name_offset = struct.unpack_from("<I", buf, p)[0]; p += 4
export_count = struct.unpack_from("<I", buf, p)[0]; p += 4
export_offset = struct.unpack_from("<I", buf, p)[0]; p += 4
import_count = struct.unpack_from("<I", buf, p)[0]; p += 4
import_offset = struct.unpack_from("<I", buf, p)[0]
region_len = (total_header - name_offset) & ~15

keys = [base64.b64decode(line.strip()) for line in KEYS.read_text().splitlines() if line.strip()]
region = buf[name_offset:name_offset+region_len]
plain = None
for key in keys:
    cand = aes_decrypt(region, key)
    n = struct.unpack_from("<i", cand, 0)[0]
    if 1 <= n <= 256 and all(b == 0 or 32 <= b < 127 for b in cand[4:4+n]):
        plain = cand; break

names = []
pos = 0
for _ in range(name_count):
    n = struct.unpack_from("<i", plain, pos)[0]; pos += 4
    if n > 0:
        s = plain[pos:pos+n-1].rstrip(b"\0").decode("latin-1", errors="replace")
        names.append(s); pos += n + 8
    elif n < 0:
        s = plain[pos:pos+(-n)*2].decode("utf-16-le").rstrip("\0")
        names.append(s); pos += (-n)*2 + 8
    else:
        names.append(""); pos += 8

# Build imports list (for resolving import refs in MIC's parent / parameter values)
imports = []
pos = import_offset - name_offset
for i in range(import_count):
    pkg_idx = struct.unpack_from("<i", plain, pos)[0]
    class_idx = struct.unpack_from("<i", plain, pos+8)[0]
    outer_idx = struct.unpack_from("<i", plain, pos+16)[0]
    obj_name_idx = struct.unpack_from("<i", plain, pos+20)[0]
    pkg = names[pkg_idx] if 0 <= pkg_idx < len(names) else "?"
    class_name = names[class_idx] if 0 <= class_idx < len(names) else "?"
    obj_name = names[obj_name_idx] if 0 <= obj_name_idx < len(names) else "?"
    imports.append({"class": class_name, "name": obj_name, "pkg": pkg, "outer": outer_idx})
    pos += 28

# Find MIC class import idx
mic_class_idx = None
for i, imp in enumerate(imports):
    if imp["name"] == "MaterialInstanceConstant":
        mic_class_idx = -(i+1); break

# Parse all exports
exports = []
pos = export_offset - name_offset
for _ in range(export_count):
    class_idx = struct.unpack_from("<i", plain, pos)[0]
    name_idx = struct.unpack_from("<i", plain, pos+12)[0]
    serial_size = struct.unpack_from("<I", plain, pos+32)[0]
    serial_offset = struct.unpack_from("<Q", plain, pos+36)[0]
    net_count = struct.unpack_from("<i", plain, pos+48)[0]
    entry_size = 52 + net_count*4 + 16 + 4
    exports.append({"class": class_idx, "name": names[name_idx], "so": serial_offset, "ss": serial_size})
    pos += entry_size

# Decompress body
body = bytearray()
p = total_header
while p + 16 <= len(buf):
    if struct.unpack_from("<I", buf, p)[0] != UPK_MAGIC: break
    blk = struct.unpack_from("<I", buf, p+4)[0]
    u_total = struct.unpack_from("<I", buf, p+12)[0]
    nb = (u_total + blk - 1) // blk
    metas_off = p + 16
    cur = metas_off + nb*8
    for bi in range(nb):
        cs = struct.unpack_from("<I", buf, metas_off + bi*8)[0]
        body += zlib.decompress(buf[cur:cur+cs])
        cur += cs
    p = cur

# Find preamble length by voting on Texture2D exports starting with NetIndex=-1
t2d_imp = None
for i, imp in enumerate(imports):
    if imp["name"] == "Texture2D":
        t2d_imp = -(i+1); break
votes = {}
for e in exports:
    if e["class"] != t2d_imp: continue
    base = e["so"] - total_header
    for k in range(0, 65536, 4):
        upos = base + k
        if upos < 0 or upos + 0x10c > len(body): continue
        if struct.unpack_from("<i", body, upos)[0] != -1: continue
        nm = struct.unpack_from("<i", body, upos + 0x108)[0]
        if 1 <= nm <= 24:
            votes[k] = votes.get(k, 0) + 1
preamble_len = min(k for k,v in votes.items() if v == max(votes.values()))

def resolve_object(idx):
    """Return human name for an ObjectIndex (UE3 packs export+import refs into a single i32:
       positive = export+1, negative = -(import+1), 0 = None)."""
    if idx == 0: return "None"
    if idx > 0:
        i = idx - 1
        if 0 <= i < len(exports):
            return f"export[{i}]={exports[i]['name']}"
        return f"export[?{idx}]"
    else:
        i = -idx - 1
        if 0 <= i < len(imports):
            return f"import[{i}]={imports[i]['name']}/{imports[i]['class']} (outer={imports[i]['outer']})"
        return f"import[?{idx}]"

# Find MICs (specifically Octane_Stars_MIC)
print("--- MaterialInstanceConstant exports ---")
mics = [e for e in exports if e["class"] == mic_class_idx]
for m in mics:
    print(f"  {m['name']}  so={m['so']:#x} ss={m['ss']}")

# Parse the Octane_Stars_MIC body
mic = next(m for m in mics if m["name"] == "Octane_Stars_MIC")
body_pos = mic["so"] - total_header + preamble_len
mic_data = body[body_pos : body_pos + mic["ss"]]
print(f"\n--- Octane_Stars_MIC body @ body[{body_pos:#x}], {mic['ss']} bytes ---")
print(f"  first 32 bytes: {mic_data[:32].hex()}")

# UE3 MaterialInstanceConstant: starts with NetIndex (4) + property block (until None)
# Properties of interest: Parent (ObjectProperty) → parent material/MIC
#                         TextureParameterValues (ArrayProperty of FTextureParameterValue structs)
# Each FTextureParameterValue (UE3) typically has:
#     ParameterName (FName) — the param key
#     ParameterValue (ObjectIndex i32) — the bound texture
#     ExpressionGUID (16 bytes)

# Property walker
none_idx = next((i for i,n in enumerate(names) if n == "None"), None)
print(f"  None FName idx: {none_idx}")

# Walk properties from body_pos + 4 (skip NetIndex)
def walk_props(start, end_max):
    pp = start
    props = []
    iters = 0
    while pp + 24 <= end_max and iters < 50:
        n_idx = struct.unpack_from("<i", body, pp)[0]
        n_num = struct.unpack_from("<i", body, pp+4)[0]
        if not (0 <= n_idx < len(names)):
            print(f"  @{pp-start:#x}: bad n_idx={n_idx} — stop")
            break
        pname = names[n_idx]
        if pname == "None":
            print(f"  @{pp-start:#x}: None — end of properties")
            return props, pp + 8
        t_idx = struct.unpack_from("<i", body, pp+8)[0]
        val_size = struct.unpack_from("<i", body, pp+16)[0]
        arr_idx = struct.unpack_from("<i", body, pp+20)[0]
        tname = names[t_idx] if 0 <= t_idx < len(names) else f"?{t_idx}"
        props.append({"name": pname, "type": tname, "off": pp, "val_size": val_size, "val_off": pp+24})
        # Most properties: 24-byte tag + val_size bytes
        # Exceptions: ByteProperty (24-byte tag + 8-byte enum name ref + val_size bytes), BoolProperty (24 + 1)
        if tname == "ByteProperty":
            pp = pp + 24 + 8 + val_size
        elif tname == "BoolProperty":
            pp = pp + 24 + 1
        else:
            pp = pp + 24 + val_size
        iters += 1
    return props, pp

props, end = walk_props(body_pos + 4, body_pos + mic["ss"])
print(f"\n  Properties found ({len(props)}):")
for pr in props:
    print(f"    {pr['name']!r} ({pr['type']}) val_size={pr['val_size']} @{pr['off']-body_pos:#x}")

# For ObjectProperty (Parent), val_size=4, value is i32 ObjectIndex
# For ArrayProperty (TextureParameterValues, VectorParameterValues, ScalarParameterValues),
#   val_size = total bytes; first 4 bytes = array element count; rest = inline-serialized array of struct
for pr in props:
    if pr["name"] == "Parent" and pr["type"] == "ObjectProperty":
        v = struct.unpack_from("<i", body, pr["val_off"])[0]
        print(f"\n  Parent -> {resolve_object(v)}")
    if pr["name"] in ("TextureParameterValues", "VectorParameterValues", "ScalarParameterValues") and pr["type"] == "ArrayProperty":
        ecount = struct.unpack_from("<i", body, pr["val_off"])[0]
        print(f"\n  {pr['name']}: count={ecount}")
        # FTextureParameterValue struct layout per entry (UE3, 116 bytes):
        #   ParameterName tag (24 bytes) + name FName value (8 bytes)
        #   ParameterValue tag (24 bytes) + object index value (4 bytes)
        #   ExpressionGUID tag (24 bytes) + struct name FName (8 bytes) + GUID (16 bytes)
        #   None FName (8 bytes)
        raw = body[pr["val_off"]+4 : pr["val_off"] + pr["val_size"]]
        if pr["name"] == "TextureParameterValues":
            entry_size = 116
            for i in range(ecount):
                base = i * entry_size
                pname_idx = struct.unpack_from("<i", raw, base + 24)[0]
                pname = names[pname_idx] if 0 <= pname_idx < len(names) else "?"
                obj_idx = struct.unpack_from("<i", raw, base + 56)[0]
                print(f"    [{i}] ParameterName = {pname!r}  ParameterValue = {obj_idx} ({resolve_object(obj_idx)})")
        elif pr["name"] == "VectorParameterValues":
            # Layout: ParameterName (32) + ParameterValue LinearColor (4 floats = 16 bytes inside struct prop, ~40) + ExpressionGUID (48) + None (8)
            # Quick parse: just find each ParameterName in the array
            stride = (pr["val_size"] - 4) // max(ecount, 1)
            for i in range(ecount):
                base = i * stride
                if base + 32 > len(raw): break
                pname_idx = struct.unpack_from("<i", raw, base + 24)[0]
                pname = names[pname_idx] if 0 <= pname_idx < len(names) else "?"
                print(f"    [{i}] (Vector) ParameterName = {pname!r}  stride={stride}")
