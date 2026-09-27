"""Check that MIC_Octane_Wolf.Diffuse was rebound from import Pepe_Body_D
to local export octane_team_wolf in the modded wolf UPK."""

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

imports = []
pos = import_offset - name_offset
for i in range(import_count):
    class_idx = struct.unpack_from("<i", plain, pos+8)[0]
    obj_name_idx = struct.unpack_from("<i", plain, pos+20)[0]
    imports.append({"class": names[class_idx] if 0 <= class_idx < len(names) else "?",
                    "name": names[obj_name_idx] if 0 <= obj_name_idx < len(names) else "?"})
    pos += 28

mic_cls = next((-(i+1) for i, imp in enumerate(imports) if imp["name"] == "MaterialInstanceConstant"), None)

exports = []
pos = export_offset - name_offset
for _ in range(export_count):
    class_idx = struct.unpack_from("<i", plain, pos)[0]
    name_idx = struct.unpack_from("<i", plain, pos+12)[0]
    serial_size = struct.unpack_from("<I", plain, pos+32)[0]
    serial_offset = struct.unpack_from("<Q", plain, pos+36)[0]
    net_count = struct.unpack_from("<i", plain, pos+48)[0]
    entry_size = 52 + net_count*4 + 16 + 4
    exports.append({"class_idx": class_idx,
                    "name": names[name_idx] if 0 <= name_idx < len(names) else "?",
                    "so": serial_offset, "ss": serial_size})
    pos += entry_size

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

# Find preamble
t2d_imp = next((-(i+1) for i, imp in enumerate(imports) if imp["name"] == "Texture2D"), None)
votes = {}
for e in exports:
    if e["class_idx"] != t2d_imp: continue
    base = e["so"] - total_header
    for k in range(0, 65536, 4):
        upos = base + k
        if upos < 0 or upos + 0x10c > len(body): continue
        if struct.unpack_from("<i", body, upos)[0] != -1: continue
        nm = struct.unpack_from("<i", body, upos + 0x108)[0]
        if 1 <= nm <= 24:
            votes[k] = votes.get(k, 0) + 1
preamble_len = min(k for k,v in votes.items() if v == max(votes.values()))

def resolve(idx):
    if idx == 0: return "None"
    if idx > 0:
        i = idx - 1
        return f"export[{i}]={exports[i]['name']}" if 0 <= i < len(exports) else f"export[?{idx}]"
    i = -idx - 1
    return f"import[{i}]={imports[i]['name']}/{imports[i]['class']}" if 0 <= i < len(imports) else f"import[?{idx}]"

for mic in [e for e in exports if e["class_idx"] == mic_cls]:
    base = mic["so"] - total_header + preamble_len
    print(f"=== {mic['name']} so={mic['so']:#x} ss={mic['ss']} base={base:#x} ===")
    pp = base + 4
    iters = 0
    while pp + 24 <= base + mic["ss"] and iters < 20:
        n_idx = struct.unpack_from("<i", body, pp)[0]
        if not (0 <= n_idx < len(names)):
            print(f"  @{pp - base:#x}: bad n_idx={n_idx}")
            break
        pname = names[n_idx]
        if pname == "None":
            print(f"  @{pp - base:#x}: None — end of props")
            break
        t_idx = struct.unpack_from("<i", body, pp+8)[0]
        val_size = struct.unpack_from("<i", body, pp+16)[0]
        tname = names[t_idx] if 0 <= t_idx < len(names) else f"?{t_idx}"
        print(f"  @{pp - base:#x}: {pname!r} ({tname}) val_size={val_size}")

        if pname == "Parent" and tname == "ObjectProperty":
            v = struct.unpack_from("<i", body, pp+24)[0]
            print(f"  Parent -> {resolve(v)}")
        if pname == "TextureParameterValues" and tname == "ArrayProperty":
            ecount = struct.unpack_from("<i", body, pp+24)[0]
            print(f"  TextureParameterValues ({ecount}):")
            for i in range(ecount):
                eb = pp + 28 + i * 116
                pn_idx = struct.unpack_from("<i", body, eb + 24)[0]
                pn = names[pn_idx] if 0 <= pn_idx < len(names) else "?"
                pv = struct.unpack_from("<i", body, eb + 56)[0]
                print(f"    [{i}] {pn!r} -> {pv} ({resolve(pv)})")

        if tname == "ByteProperty":
            pp = pp + 24 + 8 + val_size
        elif tname == "BoolProperty":
            pp = pp + 24 + 1
        else:
            pp = pp + 24 + val_size
        iters += 1
    print()
