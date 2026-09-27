"""Deep inspection of skin_octane_wolf_SF.upk to plan the Wolf-donor pipeline.

Reports:
- All exports (Texture2D, MIC, Material) with class + serial range
- All imports (resolved through outer chains)
- octane_team_wolf's full Texture2D layout: TFC name, dimensions, mip array
- Both MICs' full parameter values (resolved through ObjectIndex)
- Package renames needed to map Wolf → Stars
- Comparison with our GaleFire donor for refactoring plan"""

import base64, struct, zlib
from pathlib import Path
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes

UPK = Path(r"C:/Program Files/Epic Games/rocketleague/TAGame/CookedPCConsole/skin_octane_wolf_SF.upk")
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

# Parse imports
imports = []
pos = import_offset - name_offset
for i in range(import_count):
    pkg_idx = struct.unpack_from("<i", plain, pos)[0]
    class_idx = struct.unpack_from("<i", plain, pos+8)[0]
    outer_idx = struct.unpack_from("<i", plain, pos+16)[0]
    obj_name_idx = struct.unpack_from("<i", plain, pos+20)[0]
    imports.append({
        "idx": i,
        "name": names[obj_name_idx] if 0 <= obj_name_idx < len(names) else "?",
        "class": names[class_idx] if 0 <= class_idx < len(names) else "?",
        "pkg": names[pkg_idx] if 0 <= pkg_idx < len(names) else "?",
        "outer": outer_idx,
    })
    pos += 28

# Parse exports
exports = []
pos = export_offset - name_offset
for i in range(export_count):
    class_idx = struct.unpack_from("<i", plain, pos)[0]
    name_idx = struct.unpack_from("<i", plain, pos+12)[0]
    serial_size = struct.unpack_from("<I", plain, pos+32)[0]
    serial_offset = struct.unpack_from("<Q", plain, pos+36)[0]
    net_count = struct.unpack_from("<i", plain, pos+48)[0]
    entry_size = 52 + net_count*4 + 16 + 4
    exports.append({
        "idx": i,
        "class_idx": class_idx,
        "name": names[name_idx] if 0 <= name_idx < len(names) else "?",
        "so": serial_offset,
        "ss": serial_size,
    })
    pos += entry_size

# Resolve class names for exports
for e in exports:
    if e["class_idx"] < 0:
        ii = -e["class_idx"] - 1
        e["class"] = imports[ii]["name"] if 0 <= ii < len(imports) else "?"
    else:
        e["class"] = "(local class)"

def resolve_obj(idx):
    if idx == 0: return "None"
    if idx > 0:
        i = idx - 1
        if 0 <= i < len(exports):
            return f"export[{i}]={exports[i]['name']}/{exports[i]['class']}"
        return f"export?{idx}"
    i = -idx - 1
    if 0 <= i < len(imports):
        imp = imports[i]
        return f"import[{i}]={imp['name']}/{imp['class']}@{imp['pkg']}"
    return f"import?{idx}"

print(f"=== Wolf UPK: {UPK.name}")
print(f"    file size = {len(buf)}, total_header = {total_header:#x}")
print(f"    names={name_count} imports={import_count} exports={export_count}")

print(f"\n=== Imports (Material/MIC/Texture2D/Package only) ===")
for imp in imports:
    if imp["class"] in ("MaterialInstanceConstant", "Material", "Texture2D", "Package"):
        print(f"  [{imp['idx']:3}] {imp['name']:35} ({imp['class']:25}) pkg={imp['pkg']:25} outer={imp['outer']}")

print(f"\n=== Exports (Material/MIC/Texture2D/Skin only) ===")
for e in exports:
    cls = e["class"]
    if cls in ("MaterialInstanceConstant", "Material", "Texture2D") or "Skin" in cls or "Asset" in cls:
        print(f"  [{e['idx']:3}] {e['name']:40} ({cls:30}) so={e['so']:#x} ss={e['ss']}")

# Decompress body to parse Texture2D + MIC details
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
print(f"\nBody: {len(body)} bytes across {len(chunks)} zlib blocks")
for i, (foff, csz, bs, us) in enumerate(chunks):
    print(f"  block #{i}: file_off={foff:#x} c_size={csz} body_start={bs:#x} u_size={us}")

# Find octane_team_wolf and parse its Texture2D structure
wolf_tex = next((e for e in exports if e["name"] == "octane_team_wolf"), None)
if wolf_tex:
    print(f"\n=== octane_team_wolf (Texture2D) ===")
    print(f"  export[{wolf_tex['idx']}] so={wolf_tex['so']:#x} ss={wolf_tex['ss']}")
    # Find preamble for this UPK by scanning for the texture's mip array (flags=0x10003)
    base = wolf_tex["so"] - total_header
    print(f"  base body_pos (no preamble) = {base:#x}")
    # Locate which zlib block it's in
    for ci, (foff, csz, bs, us) in enumerate(chunks):
        if bs <= base < bs + us:
            print(f"  start of serial is in zlib block #{ci}")
            break
    # Search for flags=0x10003 within +/- 64KB window
    search_start = max(0, base - 0x10000)
    search_end = min(len(body) - 28, base + 0x20000)
    found_mips = []
    i = search_start
    while i < search_end:
        flags = struct.unpack_from("<I", body, i)[0]
        if flags == 0x10003:
            elem = struct.unpack_from("<I", body, i+4)[0]
            sd = struct.unpack_from("<I", body, i+8)[0]
            oif = struct.unpack_from("<Q", body, i+12)[0]
            dim = int(round(elem ** 0.5))
            if 4 <= dim <= 4096 and dim*dim == elem and 0 < sd < elem*2 and 0 < oif < 0xFFFFFFFF:
                found_mips.append((i, elem, sd, oif, dim))
        i += 4
    # Group consecutive (stride 28)
    groups = []
    current = []
    for m in found_mips:
        if not current or m[0] - current[-1][0] == 28:
            current.append(m)
        else:
            if len(current) >= 3: groups.append(current)
            current = [m]
    if len(current) >= 3: groups.append(current)
    # Pick closest to base
    groups.sort(key=lambda g: abs(g[0][0] - base))
    if groups:
        g = groups[0]
        print(f"  Mip array @ body[{g[0][0]:#x}] ({len(g)} TFC mips):")
        for body_off, elem, sd, oif, dim in g:
            print(f"    body[{body_off:#x}] elem={elem} (~{dim}px BC?) size_disk={sd} offset_in_file={oif:#x}")
        # Find TextureFileCacheName
        tfc_idx_in_names = next((i for i,n in enumerate(names) if n == "TextureFileCacheName"), None)
        if tfc_idx_in_names is not None:
            tag = struct.pack("<i", tfc_idx_in_names) + b"\0\0\0\0"
            tag_pos = body.find(tag, max(0, g[0][0] - 0x10000), g[0][0])
            if tag_pos >= 0:
                v_idx = struct.unpack_from("<i", body, tag_pos + 24)[0]
                tfc_name = names[v_idx] if 0 <= v_idx < len(names) else f"?{v_idx}"
                print(f"  TextureFileCacheName property at body[{tag_pos:#x}] -> {tfc_name!r}")
        # Find Format property
        fmt_idx = next((i for i,n in enumerate(names) if n == "Format"), None)
        if fmt_idx is not None:
            tag = struct.pack("<i", fmt_idx) + b"\0\0\0\0"
            tag_pos = body.find(tag, max(0, g[0][0] - 0x10000), g[0][0])
            if tag_pos >= 0:
                # ByteProperty: 24 tag + 8 enum-name + 8 value (FName of enum const)
                enum_val_off = tag_pos + 32
                v_idx = struct.unpack_from("<i", body, enum_val_off)[0]
                fmt_name = names[v_idx] if 0 <= v_idx < len(names) else f"?{v_idx}"
                print(f"  Format = {fmt_name}")

# Find both MICs and try to dump their TextureParameterValues
print(f"\n=== Wolf MIC details ===")
mic_imp_idx = next((i for i,imp in enumerate(imports) if imp["name"] == "MaterialInstanceConstant"), None)
mic_cls_neg = -(mic_imp_idx + 1) if mic_imp_idx is not None else None
none_idx = next((i for i,n in enumerate(names) if n == "None"), None)
tpv_idx = next((i for i,n in enumerate(names) if n == "TextureParameterValues"), None)
arr_idx = next((i for i,n in enumerate(names) if n == "ArrayProperty"), None)
par_idx = next((i for i,n in enumerate(names) if n == "Parent"), None)

for mic in [e for e in exports if e["class_idx"] == mic_cls_neg]:
    print(f"\n  MIC: {mic['name']} (export[{mic['idx']}], so={mic['so']:#x}, ss={mic['ss']})")
    # Find body_pos by scanning for valid first property tag (Parent/TextureParameterValues/etc)
    base = mic["so"] - total_header
    body_pos = None
    for k in range(0, 200000, 4):
        upos = base + k
        if upos + 24 > len(body): break
        if struct.unpack_from("<i", body, upos)[0] != -1: continue
        n_idx = struct.unpack_from("<i", body, upos + 4)[0]
        if not (0 <= n_idx < len(names)): continue
        pname = names[n_idx]
        if pname in ("TextureParameterValues", "Parent", "VectorParameterValues",
                     "ScalarParameterValues", "bHasStaticPermutationResource",
                     "ParentLightingGuid"):
            body_pos = upos
            break
    if body_pos is None:
        print(f"    Could not locate MIC body position")
        continue
    print(f"    body_pos = {body_pos:#x} (preamble = {body_pos - base:#x})")
    # Walk tagged properties
    pp = body_pos + 4
    iters = 0
    while pp + 24 <= body_pos + mic["ss"] and iters < 30:
        n_idx = struct.unpack_from("<i", body, pp)[0]
        if n_idx == none_idx:
            break
        if not (0 <= n_idx < len(names)): break
        pname = names[n_idx]
        t_idx = struct.unpack_from("<i", body, pp + 8)[0]
        tname = names[t_idx] if 0 <= t_idx < len(names) else "?"
        val_size = struct.unpack_from("<i", body, pp + 16)[0]
        if pname == "TextureParameterValues" and tname == "ArrayProperty":
            count = struct.unpack_from("<i", body, pp + 24)[0]
            print(f"    TextureParameterValues count={count}:")
            for ei in range(count):
                entry_start = pp + 28 + ei * 116
                pn_idx = struct.unpack_from("<i", body, entry_start + 24)[0]
                pn = names[pn_idx] if 0 <= pn_idx < len(names) else "?"
                pv = struct.unpack_from("<i", body, entry_start + 56)[0]
                print(f"      [{ei}] {pn!r:25} -> {resolve_obj(pv)}")
        elif pname == "Parent" and tname == "ObjectProperty":
            v = struct.unpack_from("<i", body, pp + 24)[0]
            print(f"    Parent -> {resolve_obj(v)}")
        else:
            print(f"    {pname!r} ({tname}) val_size={val_size}")
        if tname == "ByteProperty": pp += 24 + 8 + val_size
        elif tname == "BoolProperty": pp += 24 + 1
        else: pp += 24 + val_size
        iters += 1
