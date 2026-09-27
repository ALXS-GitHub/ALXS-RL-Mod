"""Inspect the MODDED wolf UPK at mods/skin_octane_wolf_SF.upk to verify
that the rename Textures3→MyDecal_3 took effect and the patched mip
offsets actually point into MyDecal_3.tfc at a location containing
user content."""

import base64, struct, zlib, hashlib
from pathlib import Path
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes

UPK = Path(r"C:/Program Files/Epic Games/rocketleague/TAGame/CookedPCConsole/mods/skin_octane_wolf_SF.upk")
TFC = Path(r"C:/Program Files/Epic Games/rocketleague/TAGame/CookedPCConsole/MyDecal_3.tfc")
KEYS = Path(__file__).parent.parent.parent / "src-tauri/resources/decryptor/keys.txt"
UPK_MAGIC = 0x9E2A83C1

def aes_decrypt(data, key):
    return Cipher(algorithms.AES(key), modes.ECB()).decryptor().update(data) + b""

buf = UPK.read_bytes()
print(f"UPK size: {len(buf)}")
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

print("\n--- Name table 'Texture*' / 'MyDecal*' entries ---")
for i, n in enumerate(names):
    if n.startswith("Texture") and len(n) <= 12 or "MyDecal" in n:
        print(f"  [{i}] {n!r}")

# Find octane_team_wolf export
imports = []
pos = import_offset - name_offset
for i in range(import_count):
    class_idx = struct.unpack_from("<i", plain, pos+8)[0]
    obj_name_idx = struct.unpack_from("<i", plain, pos+20)[0]
    imports.append({"class": names[class_idx] if 0 <= class_idx < len(names) else "?",
                    "name": names[obj_name_idx] if 0 <= obj_name_idx < len(names) else "?"})
    pos += 28

t2d_imp = None
for i, imp in enumerate(imports):
    if imp["name"] == "Texture2D":
        t2d_imp = -(i+1); break

exports = []
pos = export_offset - name_offset
for _ in range(export_count):
    class_idx = struct.unpack_from("<i", plain, pos)[0]
    name_idx = struct.unpack_from("<i", plain, pos+12)[0]
    serial_size = struct.unpack_from("<I", plain, pos+32)[0]
    serial_offset = struct.unpack_from("<Q", plain, pos+36)[0]
    net_count = struct.unpack_from("<i", plain, pos+48)[0]
    entry_size = 52 + net_count*4 + 16 + 4
    exports.append({"class_idx": class_idx, "name": names[name_idx] if 0 <= name_idx < len(names) else "?",
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

# Find Texture2D preamble
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
print(f"\npreamble_len = {preamble_len:#x}")

# Find octane_team_wolf, decode its tagged props for TextureFileCacheName + mip 0 offset
tfc_tag_idx = next(i for i, n in enumerate(names) if n == "TextureFileCacheName")
tfc_tag = struct.pack("<i", tfc_tag_idx) + b"\0\0\0\0"

for e in exports:
    if e["class_idx"] == t2d_imp:
        body_pos = e["so"] - total_header + preamble_len
        serial_data = body[body_pos : body_pos + e["ss"]]
        idx = serial_data.find(tfc_tag)
        tfc_name = "(none)"
        if idx >= 0:
            type_idx = struct.unpack_from("<i", serial_data, idx + 8)[0]
            val_size = struct.unpack_from("<i", serial_data, idx + 16)[0]
            type_name = names[type_idx] if 0 <= type_idx < len(names) else "?"
            if type_name == "NameProperty" and val_size == 8:
                v_idx = struct.unpack_from("<i", serial_data, idx + 24)[0]
                tfc_name = names[v_idx] if 0 <= v_idx < len(names) else "?"
        # Read mip table at body_pos + 0x10c (assumption — same as Stars)
        mip0_off_body = body_pos + 0x10c
        mip0_elem = struct.unpack_from("<I", body, mip0_off_body + 4)[0]
        mip0_size = struct.unpack_from("<I", body, mip0_off_body + 8)[0]
        mip0_offset_in_file = struct.unpack_from("<Q", body, mip0_off_body + 12)[0]
        print(f"\n--- {e['name']!r} ---")
        print(f"  TFC = {tfc_name!r}")
        print(f"  mip0: elem={mip0_elem}, size_disk={mip0_size}, offset_in_file={mip0_offset_in_file:#x}")

        # Read bytes at that offset from MyDecal_3.tfc
        if TFC.exists() and tfc_name == "MyDecal_3":
            with TFC.open("rb") as f:
                f.seek(mip0_offset_in_file)
                first = f.read(min(64, mip0_size))
                if len(first) >= 16:
                    magic = struct.unpack_from("<I", first, 0)[0]
                    blk = struct.unpack_from("<I", first, 4)[0]
                    ctot = struct.unpack_from("<I", first, 8)[0]
                    utot = struct.unpack_from("<I", first, 12)[0]
                    is_chunk = magic == UPK_MAGIC
                    print(f"  TFC@{mip0_offset_in_file:#x} (MyDecal_3.tfc):")
                    print(f"    magic={magic:#x}{' (UPK_MAGIC OK)' if is_chunk else ' (INVALID)'} blk={blk} c_total={ctot} u_total={utot}")
                    print(f"    first 32 bytes: {first[:32].hex()}")

# Also check what's at the STOCK octane_team_wolf offset (0x64054 from diag_decode_wolf.py)
print("\n--- MyDecal_3.tfc @ 0x64054 (stock wolf mip 0 offset, after sparse construction) ---")
with TFC.open("rb") as f:
    f.seek(0x64054)
    data = f.read(64)
    print(f"  first 32 bytes: {data[:32].hex()}")
    magic = struct.unpack_from("<I", data, 0)[0]
    print(f"  magic={magic:#x}{' (UPK_MAGIC)' if magic == UPK_MAGIC else ' (INVALID/zeros)'}")

# UPK file size and check if it matches stock
print(f"\nModded UPK size: {len(buf)} bytes")
stock_wolf = Path(r"C:/Program Files/Epic Games/rocketleague/TAGame/CookedPCConsole/skin_octane_wolf_SF.upk")
print(f"Stock UPK size: {stock_wolf.stat().st_size} bytes")
print(f"Sizes match: {len(buf) == stock_wolf.stat().st_size}")
