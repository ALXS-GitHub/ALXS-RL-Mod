"""TASK 1: Deep-dump Body_Paintable_Mat (export[261]) in Startup.upk.
Enumerate every MaterialExpression* sub-export whose Outer points to
Body_Paintable_Mat. For each TextureSampleParameter2D, report:
  - ParameterName FName (string)
  - Group FName (string)
  - ExpressionGUID (16 bytes)
  - DefaultTexture / Texture ObjectIndex
Then match GUID 71B78637-5FAD-894A-889C-A31EEC008A34 against found nodes.
"""

import base64, struct, zlib
from pathlib import Path
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes

UPK = Path(r"C:/Program Files/Epic Games/rocketleague/TAGame/CookedPCConsole/Startup.upk")
KEYS = Path(__file__).parent.parent.parent / "src-tauri/resources/decryptor/keys.txt"
UPK_MAGIC = 0x9E2A83C1

TARGET_DIFFUSE_GUID = bytes.fromhex("3786B7715FAD4A89889CA31EEC008A34")  # endian: per-field little
# The GUID 71B78637-5FAD-894A-889C-A31EEC008A34 stored as FGuid (4 uint32 LE)
# Field A=0x71B78637 (LE -> 37 86 B7 71), B=0x5FAD894A or layout depends.
# Standard MS GUID byte order: first 3 fields LE, last 8 bytes BE.
# 71B78637  LE-> 37 86 B7 71
# 5FAD-894A LE -> stored as uint32 0x894A5FAD ?  No — GUID = "71B78637-5FAD-894A-889C-A31EEC008A34"
# Better: try both encodings, search for both.
GUID_VARIANTS = []
def make_guid_variants(s):
    h = s.replace("-", "")
    raw = bytes.fromhex(h)
    GUID_VARIANTS.append(("MS GUID layout", bytes(reversed(raw[:4])) + bytes(reversed(raw[4:6])) + bytes(reversed(raw[6:8])) + raw[8:]))
    GUID_VARIANTS.append(("UE3 4xUInt32-LE", b"".join(bytes(reversed(raw[i:i+4])) for i in range(0,16,4))))
    GUID_VARIANTS.append(("raw bytes", raw))
make_guid_variants("71B78637-5FAD-894A-889C-A31EEC008A34")
make_guid_variants("F6B52F25-42AB-BD49-AA45-5F91F981F5C9")  # Skin (known good)

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
print(f"names: {len(names)}")

# Imports
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
print(f"imports: {len(imports)}")

# Exports
exports = []
pos = export_offset - name_offset
for _ in range(export_count):
    class_idx = struct.unpack_from("<i", plain, pos)[0]
    super_idx = struct.unpack_from("<i", plain, pos+4)[0]
    outer_idx = struct.unpack_from("<i", plain, pos+8)[0]
    name_idx = struct.unpack_from("<i", plain, pos+12)[0]
    serial_size = struct.unpack_from("<I", plain, pos+32)[0]
    serial_offset = struct.unpack_from("<Q", plain, pos+36)[0]
    net_count = struct.unpack_from("<i", plain, pos+48)[0]
    entry_size = 52 + net_count*4 + 16 + 4
    exports.append({"class": class_idx, "outer": outer_idx, "name": names[name_idx] if 0<=name_idx<len(names) else "?", "so": serial_offset, "ss": serial_size})
    pos += entry_size
print(f"exports: {len(exports)}")

# Map class idx -> class name
def class_name_for(idx):
    if idx == 0: return "None"
    if idx > 0:
        i = idx-1
        return exports[i]["name"] if 0 <= i < len(exports) else f"?ex{idx}"
    i = -idx - 1
    return imports[i]["name"] if 0 <= i < len(imports) else f"?im{idx}"

def resolve_object(idx):
    if idx == 0: return "None"
    if idx > 0:
        i = idx-1
        if 0 <= i < len(exports):
            return f"export[{i}]={exports[i]['name']}({class_name_for(exports[i]['class'])})"
        return f"export[?{idx}]"
    i = -idx - 1
    if 0 <= i < len(imports):
        return f"import[{i}]={imports[i]['name']}/{imports[i]['class']}"
    return f"import[?{idx}]"

# Locate Body_Paintable_Mat (expected export[261])
bpm = None
for ei, e in enumerate(exports):
    if e["name"] == "Body_Paintable_Mat" and class_name_for(e["class"]) == "Material":
        bpm = (ei, e)
        break
print(f"Body_Paintable_Mat = export[{bpm[0]}], so={bpm[1]['so']:#x}, ss={bpm[1]['ss']}")
bpm_ei = bpm[0]

# Find all sub-exports whose Outer == bpm export+1 (UE3 outer ref is export index +1)
bpm_outer_ref = bpm_ei + 1
sub_exports = []
for ei, e in enumerate(exports):
    if e["outer"] == bpm_outer_ref:
        sub_exports.append((ei, e))
print(f"\nBody_Paintable_Mat has {len(sub_exports)} sub-objects (MaterialExpression*):")

# Decompress body
body = bytearray()
pp = total_header
chunks = []
while pp + 16 <= len(buf):
    if struct.unpack_from("<I", buf, pp)[0] != UPK_MAGIC: break
    blk_size = struct.unpack_from("<I", buf, pp+4)[0]
    u_total = struct.unpack_from("<I", buf, pp+12)[0]
    nb = (u_total + blk_size - 1) // blk_size
    metas_off = pp + 16
    cur = metas_off + nb*8
    for bi in range(nb):
        cs = struct.unpack_from("<I", buf, metas_off + bi*8)[0]
        us = struct.unpack_from("<I", buf, metas_off + bi*8 + 4)[0]
        chunks.append((cur, cs, len(body), us))
        body += zlib.decompress(buf[cur:cur+cs])
        cur += cs
    pp = cur
print(f"body decompressed: {len(body)} bytes ({len(chunks)} chunks)")

# Find preamble length by voting on Texture2D exports starting with NetIndex=-1 (same heuristic as donor)
# Startup.upk: use any reliable export's body. Try the small MaterialExpression bodies.
# For Materials, the body starts with NetIndex (i32, usually -1) too. Just use the simplest: try preamble_len = 4.

# Startup.upk: serial_offset is 1:1 with body[so - total_header]. NO preamble.
# (The donor UPK heuristic of NetIndex=-1 + 0x108 doesn't apply here.)
preamble_len = 0
print(f"preamble_len forced to 0 (Startup.upk convention)")

# Tagged-property walker (re-used from diag_mic_params.py)
none_idx = next((i for i,n in enumerate(names) if n == "None"), None)

def walk_props(start, end_max):
    pp = start
    props = []
    iters = 0
    while pp + 24 <= end_max and iters < 200:
        n_idx = struct.unpack_from("<i", body, pp)[0]
        if not (0 <= n_idx < len(names)):
            return props, pp, f"bad n_idx={n_idx} at {pp:#x}"
        pname = names[n_idx]
        if pname == "None":
            return props, pp + 8, "ok"
        t_idx = struct.unpack_from("<i", body, pp+8)[0]
        val_size = struct.unpack_from("<i", body, pp+16)[0]
        arr_idx = struct.unpack_from("<i", body, pp+20)[0]
        tname = names[t_idx] if 0 <= t_idx < len(names) else f"?{t_idx}"
        # Sanity check: val_size must be non-negative and within remaining bytes
        if val_size < 0 or val_size > (end_max - pp - 24):
            return props, pp, f"bad val_size={val_size} (pname={pname!r} tname={tname!r})"
        prop = {"name": pname, "type": tname, "off": pp, "val_size": val_size, "val_off": pp+24, "arr_idx": arr_idx}
        props.append(prop)
        if tname == "ByteProperty":
            new_pp = pp + 24 + 8 + val_size
        elif tname == "BoolProperty":
            new_pp = pp + 24 + 1
        elif tname == "StructProperty":
            # 24-byte tag + 8-byte struct FName + val_size payload
            new_pp = pp + 24 + 8 + val_size
            prop["struct_payload_off"] = prop["val_off"] + 8
            prop["struct_name_idx"] = struct.unpack_from("<i", body, prop["val_off"])[0]
        else:
            new_pp = pp + 24 + val_size
        if new_pp <= pp or new_pp > end_max:
            return props, pp, f"bad advance new_pp={new_pp} from pp={pp}"
        pp = new_pp
        iters += 1
    return props, pp, "limit"

# Robust extraction via tag-pattern scanning.
# For each sub-export body, scan for FName tag of "ParameterName" (i32 name idx + i32 number=0),
# followed by NameProperty / NameProperty val_size=8 / arr_idx + ParameterName value FName.
# Also scan for "ExpressionGUID" tag followed by StructProperty + struct name FName "Guid" + 16 bytes.

PNAME_IDX_PN = names.index("ParameterName") if "ParameterName" in names else -1
PNAME_IDX_GUID = names.index("ExpressionGUID") if "ExpressionGUID" in names else -1
PNAME_IDX_GROUP = names.index("Group") if "Group" in names else -1
PNAME_IDX_TEXTURE = names.index("Texture") if "Texture" in names else -1
PNAME_IDX_DEFTEX = names.index("DefaultTexture") if "DefaultTexture" in names else -1
TNAME_IDX_NAME = names.index("NameProperty") if "NameProperty" in names else -1
TNAME_IDX_STRUCT = names.index("StructProperty") if "StructProperty" in names else -1
TNAME_IDX_OBJECT = names.index("ObjectProperty") if "ObjectProperty" in names else -1
print(f"\nKey FName indices: ParameterName={PNAME_IDX_PN}, ExpressionGUID={PNAME_IDX_GUID}, Group={PNAME_IDX_GROUP}, Texture={PNAME_IDX_TEXTURE}, DefaultTexture={PNAME_IDX_DEFTEX}")
print(f"  NameProperty={TNAME_IDX_NAME}, StructProperty={TNAME_IDX_STRUCT}, ObjectProperty={TNAME_IDX_OBJECT}")

def find_tag(serial_bytes, name_idx, type_idx, val_size=None):
    """Find a property tag with given name and type FName indices. Returns offset within serial_bytes or -1."""
    pat = struct.pack("<ii", name_idx, 0) + struct.pack("<ii", type_idx, 0)  # 16 bytes: name FName + type FName
    pos = 0
    while pos < len(serial_bytes):
        i = serial_bytes.find(pat, pos)
        if i < 0: return -1
        # check val_size if specified
        if val_size is not None and i + 24 <= len(serial_bytes):
            vs = struct.unpack_from("<i", serial_bytes, i+16)[0]
            if vs == val_size:
                return i
            pos = i + 1
            continue
        return i
    return -1

print(f"\n{'='*80}\nSub-export TextureSampleParameter2D / VectorParameter inventory:\n{'='*80}")
guid_hits = []
target_classes = {"MaterialExpressionTextureSampleParameter2D",
                  "MaterialExpressionVectorParameter",
                  "MaterialExpressionScalarParameter",
                  "MaterialExpressionStaticSwitchParameter",
                  "MaterialExpressionTextureObjectParameter"}

for sub_ei, sub in sub_exports:
    cls = class_name_for(sub["class"])
    if cls not in target_classes:
        continue
    body_pos = sub["so"] - total_header
    end = body_pos + sub["ss"]
    if end > len(body):
        print(f"[{sub_ei}] {sub['name']} ({cls}) OUT OF RANGE")
        continue
    serial = bytes(body[body_pos:end])

    # find ParameterName tag (NameProperty, val_size=8)
    pname_val = None
    group_val = None
    guid_val = None
    texture_idx = None

    if PNAME_IDX_PN >= 0 and TNAME_IDX_NAME >= 0:
        i = find_tag(serial, PNAME_IDX_PN, TNAME_IDX_NAME, val_size=8)
        if i >= 0:
            # tag = 24 bytes. value (FName) follows.
            v_idx = struct.unpack_from("<i", serial, i + 24)[0]
            if 0 <= v_idx < len(names):
                pname_val = names[v_idx]

    if PNAME_IDX_GROUP >= 0 and TNAME_IDX_NAME >= 0:
        i = find_tag(serial, PNAME_IDX_GROUP, TNAME_IDX_NAME, val_size=8)
        if i >= 0:
            v_idx = struct.unpack_from("<i", serial, i + 24)[0]
            if 0 <= v_idx < len(names):
                group_val = names[v_idx]

    if PNAME_IDX_GUID >= 0 and TNAME_IDX_STRUCT >= 0:
        # ExpressionGUID is StructProperty with val_size=16, struct name FName "Guid" (8 bytes) then 16 bytes payload.
        i = find_tag(serial, PNAME_IDX_GUID, TNAME_IDX_STRUCT, val_size=16)
        if i >= 0:
            # struct payload = serial[i+24 + 8 : i+24+8+16]
            guid_val = serial[i + 32 : i + 48]

    # Texture / DefaultTexture: ObjectProperty val_size=4 -> i32 object index
    for tex_name_idx in (PNAME_IDX_TEXTURE, PNAME_IDX_DEFTEX):
        if tex_name_idx < 0 or TNAME_IDX_OBJECT < 0: continue
        i = find_tag(serial, tex_name_idx, TNAME_IDX_OBJECT, val_size=4)
        if i >= 0:
            texture_idx = struct.unpack_from("<i", serial, i + 24)[0]
            break

    print(f"\n[{sub_ei}] {sub['name']} ({cls}) so={sub['so']:#x} ss={sub['ss']}")
    if pname_val: print(f"     ParameterName = {pname_val!r}")
    if group_val: print(f"     Group         = {group_val!r}")
    if guid_val:
        print(f"     ExpressionGUID = {guid_val.hex().upper()}")
        for label, gvar in GUID_VARIANTS:
            if guid_val == gvar:
                print(f"     ** MATCH ({label}) **")
                guid_hits.append((sub_ei, sub["name"], cls, pname_val, group_val, label, guid_val.hex().upper()))
    if texture_idx is not None and texture_idx != 0:
        print(f"     Texture/Default -> {resolve_object(texture_idx)}")

# Final summary
print(f"\n{'='*80}\nGUID MATCH SUMMARY\n{'='*80}")
print(f"Query GUIDs (raw / variants):")
for label, g in GUID_VARIANTS:
    print(f"  {label}: {g.hex().upper()}")
print(f"\nMatches found in Body_Paintable_Mat sub-expressions:")
if guid_hits:
    for hit in guid_hits:
        print(f"  export[{hit[0]}] {hit[1]} ({hit[2]}) ParameterName={hit[3]!r} Group={hit[4]!r} via {hit[5]}")
else:
    print("  NONE — the Diffuse GUID 71B78637-5FAD-894A-889C-A31EEC008A34 does NOT match any sub-expression")
    print("  in Body_Paintable_Mat. Our per-decal MIC.Diffuse override is routed to a GUID")
    print("  that does not exist in the parent material -> override is silently ignored.")
