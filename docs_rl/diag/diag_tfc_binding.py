"""Parse TextureFileCacheName property for each Texture2D in the donor UPK.

The FName 'TextureFileCacheName' is a UProperty inside each Texture2D's
serial data, before the mip array. We scan property entries until we hit
'None' (end of property block)."""

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
        plain = cand
        break

# Parse name table
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

# Show all names containing "Texture" or "Decal" or "Skin"
print("--- Name table entries containing 'Texture' or 'Skin' or starting with Force ---")
for i, n in enumerate(names):
    if "Texture" in n or "Skin" in n or n.startswith("Force") or "Decal" in n or n == "TextureFileCacheName" or n == "None":
        print(f"  [{i}] {n!r}")

# Find Texture2D class idx
texture2d_idx = None
pos = import_offset - name_offset
for i in range(import_count):
    obj_name_idx = struct.unpack_from("<i", plain, pos+20)[0]
    if 0 <= obj_name_idx < len(names) and names[obj_name_idx] == "Texture2D":
        texture2d_idx = -(i + 1); break
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
    entry_size = 52 + net_count * 4 + 16 + 4
    if class_idx == texture2d_idx:
        exports.append((names[name_idx], serial_offset, serial_size))
    pos += entry_size

# Decompress body
body = bytearray()
p = total_header_size
while p + 16 <= len(buf):
    if struct.unpack_from("<I", buf, p)[0] != UPK_MAGIC:
        break
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

# Find preamble length
votes = {}
for name, so, ss in exports:
    base = so - total_header_size
    for k in range(0, 65536, 4):
        upos = base + k
        if upos < 0 or upos + 0x10c > len(body):
            continue
        netindex = struct.unpack_from("<i", body, upos)[0]
        if netindex != -1: continue
        num_mips = struct.unpack_from("<i", body, upos + 0x108)[0]
        if 1 <= num_mips <= 24:
            votes[k] = votes.get(k, 0) + 1
preamble_len = min(k for k, v in votes.items() if v == len(exports)) if votes else 0

# For each Texture2D, parse properties until 'None'.
# UE3 property entry layout:
#   name_idx (4)    -- FName index
#   name_number (4) -- FName instance number
#   type_idx (4)    -- FName index of type name (e.g. "NameProperty")
#   type_number (4)
#   value_size (4)
#   array_idx (4)
#   value (value_size bytes)
none_idx = next((i for i, n in enumerate(names) if n == "None"), None)
tfc_name_idx = next((i for i, n in enumerate(names) if n == "TextureFileCacheName"), None)
print(f"\nname idx 'None'={none_idx} 'TextureFileCacheName'={tfc_name_idx}")

print(f"\n--- Property scan per Texture2D ---")
for name, so, ss in exports:
    body_pos = so - total_header_size + preamble_len
    print(f"\n{name}: serial_size={ss}")
    p = body_pos
    # Properties start at NetIndex(4 bytes) + StackUnknown(some bytes)?
    # Actually for Texture2D the layout is:
    #   NetIndex (4) = -1
    #   then properties stream starting at body_pos + 4
    # But really the conventional UE3 export structure is:
    #   - PropertyStream (UProperty tags until 'None')
    #   - Then UTexture2D-specific fields including bIsSourceArt, num_mips, etc.
    # The 0x108 offset of num_mips suggests there's a big preamble before mips.

    # Walk properties from body_pos + 4 (skip netindex)
    pp = body_pos + 4
    serial_end = body_pos + ss
    iters = 0
    while pp + 24 <= serial_end and iters < 30:
        n_idx = struct.unpack_from("<i", body, pp)[0]
        n_num = struct.unpack_from("<i", body, pp+4)[0]
        if not (0 <= n_idx < len(names)):
            print(f"  @{pp - body_pos:#x}: bad name_idx {n_idx} — stop")
            break
        prop_name = names[n_idx]
        if prop_name == "None":
            print(f"  @{pp - body_pos:#x}: None — property block end")
            pp += 8
            break
        t_idx = struct.unpack_from("<i", body, pp+8)[0]
        t_num = struct.unpack_from("<i", body, pp+12)[0]
        val_size = struct.unpack_from("<i", body, pp+16)[0]
        arr_idx = struct.unpack_from("<i", body, pp+20)[0]
        type_name = names[t_idx] if 0 <= t_idx < len(names) else f"?{t_idx}"
        val_off = pp + 24
        # For NameProperty: value = i32 name_idx + i32 number (8 bytes)
        if type_name == "NameProperty" and val_size == 8:
            v_idx = struct.unpack_from("<i", body, val_off)[0]
            v_num = struct.unpack_from("<i", body, val_off+4)[0]
            v_name = names[v_idx] if 0 <= v_idx < len(names) else f"?{v_idx}"
            print(f"  @{pp - body_pos:#x}: {prop_name}({type_name}) = {v_name!r}")
        elif type_name == "IntProperty" and val_size == 4:
            v = struct.unpack_from("<i", body, val_off)[0]
            print(f"  @{pp - body_pos:#x}: {prop_name}({type_name}) = {v}")
        elif type_name == "BoolProperty":
            # val_size is 0 for bool, but a 1-byte value follows
            v = body[val_off] if val_off < serial_end else "?"
            print(f"  @{pp - body_pos:#x}: {prop_name}({type_name}) = {bool(v) if v != '?' else v}")
            # bool has 0 val_size but 1 byte follows
            pp = val_off + 1
            iters += 1
            continue
        elif type_name == "ByteProperty":
            # ByteProperty value: i32 enum_name_idx + i32 enum_num + 1 byte (or 8 bytes name)
            print(f"  @{pp - body_pos:#x}: {prop_name}({type_name}) val_size={val_size}")
        else:
            print(f"  @{pp - body_pos:#x}: {prop_name}({type_name}) val_size={val_size}")
        pp = val_off + val_size
        iters += 1
