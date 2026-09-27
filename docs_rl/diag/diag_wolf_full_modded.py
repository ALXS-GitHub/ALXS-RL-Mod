"""Robust Wolf UPK MIC dumper — find preamble by NetIndex=-1 hit + None FName
inside likely property blocks; then parse both Wolf MICs."""

import base64, struct, zlib
from pathlib import Path
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes

UPK = Path(r"C:/Program Files/Epic Games/rocketleague/TAGame/CookedPCConsole/mods/skin_octane_wolf_SF.upk")
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

# Parse all imports for ObjectIndex resolution
imports = []
pos = import_offset - name_offset
for i in range(import_count):
    pkg_idx = struct.unpack_from("<i", plain, pos)[0]
    class_idx = struct.unpack_from("<i", plain, pos+8)[0]
    outer_idx = struct.unpack_from("<i", plain, pos+16)[0]
    obj_name_idx = struct.unpack_from("<i", plain, pos+20)[0]
    imports.append({
        "name": names[obj_name_idx] if 0 <= obj_name_idx < len(names) else "?",
        "class": names[class_idx] if 0 <= class_idx < len(names) else "?",
        "outer": outer_idx,
    })
    pos += 28

# Parse exports
exports = []
pos = export_offset - name_offset
for _ in range(export_count):
    class_idx = struct.unpack_from("<i", plain, pos)[0]
    name_idx = struct.unpack_from("<i", plain, pos+12)[0]
    serial_size = struct.unpack_from("<I", plain, pos+32)[0]
    serial_offset = struct.unpack_from("<Q", plain, pos+36)[0]
    net_count = struct.unpack_from("<i", plain, pos+48)[0]
    entry_size = 52 + net_count*4 + 16 + 4
    exports.append({
        "name": names[name_idx] if 0 <= name_idx < len(names) else "?",
        "class_idx": class_idx,
        "so": serial_offset,
        "ss": serial_size,
    })
    pos += entry_size

# Decompress body
body = bytearray()
pp = total_header
while pp + 16 <= len(buf):
    if struct.unpack_from("<I", buf, pp)[0] != UPK_MAGIC: break
    blk_size = struct.unpack_from("<I", buf, pp+4)[0]
    u_total = struct.unpack_from("<I", buf, pp+12)[0]
    nb = (u_total + blk_size - 1) // blk_size
    metas_off = pp + 16
    cur = metas_off + nb*8
    for bi in range(nb):
        cs = struct.unpack_from("<I", buf, metas_off + bi*8)[0]
        body += zlib.decompress(buf[cur:cur+cs])
        cur += cs
    pp = cur

# Find preamble by scanning for NetIndex=-1 at expected positions for the MIC exports
none_idx = next((i for i,n in enumerate(names) if n == "None"), None)
parent_idx = next((i for i,n in enumerate(names) if n == "Parent"), None)
texparval_idx = next((i for i,n in enumerate(names) if n == "TextureParameterValues"), None)
arrayprop_idx = next((i for i,n in enumerate(names) if n == "ArrayProperty"), None)
print(f"FName indices: None={none_idx} Parent={parent_idx} TextureParameterValues={texparval_idx} ArrayProperty={arrayprop_idx}")

# Find MICs (class_idx negative referring to MaterialInstanceConstant import)
mic_import_idx = next((i for i,imp in enumerate(imports) if imp["name"] == "MaterialInstanceConstant"), None)
mic_class_neg = -(mic_import_idx + 1) if mic_import_idx is not None else None
print(f"MIC class_idx (negative): {mic_class_neg}")

mic_exports = [e for e in exports if e["class_idx"] == mic_class_neg]
print(f"MIC exports found: {[e['name'] for e in mic_exports]}")

def resolve_object(idx):
    if idx == 0: return "None"
    if idx > 0:
        i = idx - 1
        if 0 <= i < len(exports):
            return f"export[{i}]={exports[i]['name']}"
        return f"export?{idx}"
    i = -idx - 1
    if 0 <= i < len(imports):
        return f"import[{i}]={imports[i]['name']}/{imports[i]['class']}"
    return f"import?{idx}"

for mic in mic_exports:
    base = mic["so"] - total_header
    print(f"\n=== {mic['name']} (so={mic['so']:#x} ss={mic['ss']}) ===")

    # Scan ALL NetIndex=-1 positions in the wider window, then for each
    # candidate verify that property walking (starting at +4) produces
    # a sensible 'Parent' or 'TextureParameterValues' tag.
    found_pre = None
    for k in range(0, 200000, 4):
        upos = base + k
        if upos + 200 > len(body): break
        if struct.unpack_from("<i", body, upos)[0] != -1: continue
        # Validate: read i32 at upos+4 (first property tag's name_idx). Should
        # be a valid FName index in [0, name_count) AND correspond to a name
        # that's a plausible property tag (Parent, TextureParameterValues,
        # VectorParameterValues, ScalarParameterValues, bHasStaticPermutationResource,
        # ParentLightingGuid, or starts with a capital letter and len > 1).
        n_idx = struct.unpack_from("<i", body, upos + 4)[0]
        if not (0 <= n_idx < len(names)): continue
        pname = names[n_idx]
        if pname in ("Parent", "TextureParameterValues", "VectorParameterValues",
                     "ScalarParameterValues", "bHasStaticPermutationResource",
                     "ParentLightingGuid"):
            found_pre = k
            break
    if found_pre is None:
        print(f"  Could not find valid preamble!")
        continue
    body_pos = base + found_pre
    print(f"  preamble = {found_pre:#x}, body_pos = {body_pos:#x}")

    # Walk tagged properties from body_pos + 4
    pp = body_pos + 4
    serial_end = body_pos + mic["ss"]
    iters = 0
    while pp + 24 <= serial_end and iters < 30:
        n_idx = struct.unpack_from("<i", body, pp)[0]
        if n_idx == none_idx:
            print(f"  @{pp - body_pos:#x}: None")
            break
        if n_idx < 0 or n_idx >= len(names):
            print(f"  @{pp - body_pos:#x}: bad name_idx {n_idx}")
            break
        pname = names[n_idx]
        t_idx = struct.unpack_from("<i", body, pp + 8)[0]
        tname = names[t_idx] if 0 <= t_idx < len(names) else f"?{t_idx}"
        val_size = struct.unpack_from("<i", body, pp + 16)[0]
        print(f"  @{pp - body_pos:#x}: {pname!r} ({tname}) val_size={val_size}")
        if pname == "Parent" and tname == "ObjectProperty":
            v = struct.unpack_from("<i", body, pp + 24)[0]
            print(f"      Parent -> {resolve_object(v)}")
        if pname == "TextureParameterValues" and tname == "ArrayProperty":
            count = struct.unpack_from("<i", body, pp + 24)[0]
            print(f"      Entries: {count}")
            for ei in range(count):
                entry_start = pp + 28 + ei * 116
                if entry_start + 60 > serial_end: break
                pn_idx = struct.unpack_from("<i", body, entry_start + 24)[0]
                pn = names[pn_idx] if 0 <= pn_idx < len(names) else "?"
                pv = struct.unpack_from("<i", body, entry_start + 56)[0]
                print(f"        [{ei}] {pn!r} -> {pv} ({resolve_object(pv)})")
        if tname == "ByteProperty":
            pp = pp + 24 + 8 + val_size
        elif tname == "BoolProperty":
            pp = pp + 24 + 1
        else:
            pp = pp + 24 + val_size
        iters += 1
