"""Inspect the MODDED UPK to see actual TextureFileCacheName values and
patched mip array offsets after swap. Also verify what bytes live at those
offsets in MyDecal_.tfc and MyDecal_3.tfc."""

import base64, struct, zlib, hashlib
from pathlib import Path
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes

UPK_PATH = Path(r"C:/Program Files/Epic Games/rocketleague/TAGame/CookedPCConsole/mods/Skin_Octane_Stars_SF.upk")
TFC_DIFFUSE = Path(r"C:/Program Files/Epic Games/rocketleague/TAGame/CookedPCConsole/MyDecal_.tfc")
TFC_MASK = Path(r"C:/Program Files/Epic Games/rocketleague/TAGame/CookedPCConsole/MyDecal_3.tfc")
KEYS_PATH = Path(__file__).parent.parent.parent / "src-tauri/resources/decryptor/keys.txt"

UPK_MAGIC = 0x9E2A83C1
AES_BLOCK = 16

def aes_decrypt(data, key):
    c = Cipher(algorithms.AES(key), modes.ECB())
    d = c.decryptor()
    return d.update(data) + d.finalize()

buf = UPK_PATH.read_bytes()
total_header_size = struct.unpack_from("<I", buf, 8)[0]
folder_len = abs(struct.unpack_from("<i", buf, 12)[0])
p = 16 + folder_len + 4
name_count = struct.unpack_from("<I", buf, p)[0]; p += 4
name_offset = struct.unpack_from("<I", buf, p)[0]; p += 4
export_count = struct.unpack_from("<I", buf, p)[0]; p += 4
export_offset = struct.unpack_from("<I", buf, p)[0]; p += 4
import_count = struct.unpack_from("<I", buf, p)[0]; p += 4
import_offset = struct.unpack_from("<I", buf, p)[0]; p += 4
region_len = (total_header_size - name_offset) & ~(AES_BLOCK - 1)

keys = [base64.b64decode(line.strip()) for line in KEYS_PATH.read_text().splitlines() if line.strip()]
region = buf[name_offset:name_offset+region_len]
plain = None
for key in keys:
    cand = aes_decrypt(region, key)
    n = struct.unpack_from("<i", cand, 0)[0]
    if 1 <= n <= 256 and all(b == 0 or 32 <= b < 127 for b in cand[4:4+n]):
        plain = cand; break

names = []
pos = 0
for i in range(name_count):
    n = struct.unpack_from("<i", plain, pos)[0]; pos += 4
    if n > 0:
        s = plain[pos:pos+n-1].rstrip(b"\0").decode("latin-1", errors="replace")
        names.append(s); pos += n + 8
    elif n < 0:
        s = plain[pos:pos+(-n)*2].decode("utf-16-le").rstrip("\0")
        names.append(s); pos += (-n)*2 + 8
    else:
        names.append(""); pos += 8

# Show MyDecal* and Textures* in the name table
print("--- Renamed name table entries ---")
for i, n in enumerate(names):
    if "MyDecal" in n or n.startswith("Texture") and len(n) <= 12:
        print(f"  [{i}] {n!r}")

tfc_name_idx = next(i for i, n in enumerate(names) if n == "TextureFileCacheName")
texture2d_idx = None
pos = import_offset - name_offset
for i in range(import_count):
    obj_name_idx = struct.unpack_from("<i", plain, pos+20)[0]
    if 0 <= obj_name_idx < len(names) and names[obj_name_idx] == "Texture2D":
        texture2d_idx = -(i+1); break
    pos += 28

exports = []
pos = export_offset - name_offset
for _ in range(export_count):
    class_idx = struct.unpack_from("<i", plain, pos)[0]
    name_idx = struct.unpack_from("<i", plain, pos+12)[0]
    serial_size = struct.unpack_from("<I", plain, pos+32)[0]
    serial_offset = struct.unpack_from("<Q", plain, pos+36)[0]
    net_count = struct.unpack_from("<i", plain, pos+48)[0]
    entry_size = 52 + net_count * 4 + 16 + 4
    if class_idx == texture2d_idx:
        exports.append((names[name_idx], serial_offset, serial_size))
    pos += entry_size

# Decompress body
body = bytearray()
p = total_header_size
while p + 16 <= len(buf):
    if struct.unpack_from("<I", buf, p)[0] != UPK_MAGIC: break
    blk_size = struct.unpack_from("<I", buf, p+4)[0]
    u_total = struct.unpack_from("<I", buf, p+12)[0]
    nblocks = (u_total + blk_size - 1) // blk_size
    metas_off = p + 16
    cur = metas_off + nblocks * 8
    for bi in range(nblocks):
        cs = struct.unpack_from("<I", buf, metas_off + bi*8)[0]
        body += zlib.decompress(buf[cur:cur+cs])
        cur += cs
    p = cur

# Find preamble
votes = {}
for name, so, ss in exports:
    base = so - total_header_size
    for k in range(0, 65536, 4):
        upos = base + k
        if upos < 0 or upos + 0x10c > len(body): continue
        if struct.unpack_from("<i", body, upos)[0] != -1: continue
        nm = struct.unpack_from("<i", body, upos + 0x108)[0]
        if 1 <= nm <= 24:
            votes[k] = votes.get(k, 0) + 1
preamble_len = min(k for k,v in votes.items() if v == len(exports))
print(f"\npreamble_len={preamble_len:#x}")

# For each Texture2D: TextureFileCacheName + mip 0 offset
tfc_tag = struct.pack("<i", tfc_name_idx) + b"\0\0\0\0"
print(f"\n--- Modded UPK Texture2D bindings + mip 0 offset ---")
texture_info = {}
for name, so, ss in exports:
    body_pos = so - total_header_size + preamble_len
    serial_data = body[body_pos : body_pos + ss]
    idx = serial_data.find(tfc_tag)
    tfc_name = "(none)"
    if idx >= 0:
        type_idx = struct.unpack_from("<i", serial_data, idx + 8)[0]
        val_size = struct.unpack_from("<i", serial_data, idx + 16)[0]
        type_name = names[type_idx] if 0 <= type_idx < len(names) else "?"
        if type_name == "NameProperty" and val_size == 8:
            v_idx = struct.unpack_from("<i", serial_data, idx + 24)[0]
            tfc_name = names[v_idx] if 0 <= v_idx < len(names) else "?"
    # mip 0 offset
    mip0_off = body_pos + 0x10c
    elem = struct.unpack_from("<I", body, mip0_off + 4)[0]
    size_disk = struct.unpack_from("<I", body, mip0_off + 8)[0]
    offset_in_file = struct.unpack_from("<Q", body, mip0_off + 12)[0]
    print(f"  {name}: TFC={tfc_name!r}  mip0(elem={elem}, size_disk={size_disk}, offset_in_file={offset_in_file:#x})")
    texture_info[name] = (tfc_name, offset_in_file, size_disk)

# Now read bytes at the patched offsets from each TFC
print(f"\n--- Reading from TFCs at patched offsets ---")
def sample_tfc(path, offset, size, label):
    if not path.exists():
        print(f"  {label}: TFC {path.name} missing!")
        return
    with path.open("rb") as f:
        f.seek(offset)
        data = f.read(min(size, 64))  # just first 64 bytes
        # Check if this looks like our chunked-zlib wrapper (magic + block size + ...)
        if len(data) >= 16:
            magic = struct.unpack_from("<I", data, 0)[0]
            blk = struct.unpack_from("<I", data, 4)[0] if len(data) >= 8 else 0
            ctot = struct.unpack_from("<I", data, 8)[0] if len(data) >= 12 else 0
            utot = struct.unpack_from("<I", data, 12)[0] if len(data) >= 16 else 0
            is_chunk = magic == UPK_MAGIC
            print(f"  {label}: TFC={path.name} @ {offset:#x}: magic={magic:#x}{' (UPK_MAGIC OK)' if is_chunk else ''} blk={blk} c_total={ctot} u_total={utot}")
            print(f"    first 32 bytes: {data[:32].hex()}")

for name, (tfc_name, offset, size) in texture_info.items():
    if tfc_name == "MyDecal_":
        sample_tfc(TFC_DIFFUSE, offset, size, name)
    elif tfc_name == "MyDecal_3":
        sample_tfc(TFC_MASK, offset, size, name)
    else:
        print(f"  {name}: reads from {tfc_name}.tfc (not modded) at {offset:#x}")

# Also peek at MyDecal_3.tfc at offset max_orig (~0x19453d69) to see if mask user PNG actually got written there
print(f"\n--- MyDecal_3.tfc content at original max_orig area (0x19453de9, never read because no texture points here) ---")
sample_tfc(TFC_MASK, 0x19453de9, 64, "MyDecal_3 at 0x19453de9 (mask write target — orphan)")
# Also peek near beginning
print(f"--- MyDecal_3.tfc start (sparse zero region expected) ---")
sample_tfc(TFC_MASK, 0, 64, "MyDecal_3 at 0")
