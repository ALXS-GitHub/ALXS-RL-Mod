"""Find TextureFileCacheName value for each Texture2D — search directly for
name idx 195 inside each export's serial data."""

import base64, struct, zlib
from pathlib import Path
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes

UPK_PATH = Path(r"C:/Program Files/Epic Games/rocketleague/TAGame/CookedPCConsole/skin_octane_galefire_SF.upk")
KEYS_PATH = Path(__file__).parent / "src-tauri/resources/decryptor/keys.txt"

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

# Now for each Texture2D, search for TextureFileCacheName tag inside the serial range
tfc_tag = struct.pack("<i", tfc_name_idx) + b"\0\0\0\0"  # name_idx + name_num=0
print(f"Searching for TextureFileCacheName tag bytes: {tfc_tag.hex()}")
print(f"name idx for 'TextureFileCacheName': {tfc_name_idx}")

print(f"\n--- TextureFileCacheName per Texture2D ---")
for name, so, ss in exports:
    body_pos = so - total_header_size + preamble_len
    serial_data = body[body_pos : body_pos + ss]
    idx = serial_data.find(tfc_tag)
    if idx < 0:
        print(f"  {name}: NO TextureFileCacheName tag found")
        continue
    # Tag layout: name(8) + type(8) + val_size(4) + arr_idx(4) + value(val_size)
    # For NameProperty val_size=8 and value is (name_idx, name_num)
    type_idx = struct.unpack_from("<i", serial_data, idx + 8)[0]
    val_size = struct.unpack_from("<i", serial_data, idx + 16)[0]
    type_name = names[type_idx] if 0 <= type_idx < len(names) else f"?{type_idx}"
    val_off = idx + 24
    if type_name == "NameProperty" and val_size == 8:
        v_idx = struct.unpack_from("<i", serial_data, val_off)[0]
        v_name = names[v_idx] if 0 <= v_idx < len(names) else f"?{v_idx}"
        print(f"  {name}: TextureFileCacheName = {v_name!r}  (name idx {v_idx})")
    else:
        print(f"  {name}: weird (type={type_name}, val_size={val_size})")
