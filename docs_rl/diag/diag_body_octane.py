"""Inspect Body_Octane_SF.upk to locate Pepe_Body_D and its TFC binding.

We need:
1. Pepe_Body_D's export index (for patches addressing).
2. Pepe_Body_D's TextureFileCacheName FName value (so we know what to
   rename in the name table to redirect into our custom TFC).
3. Pepe_Body_D's mip array body offsets (to patch the per-mip
   BulkDataOffsetInFile + BulkDataSizeOnDisk + ElementCount fields).

Output guides the Body_Octane_SF.upk hijack implementation."""

import base64, struct, zlib
from pathlib import Path
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes

import sys
UPK = Path(sys.argv[1] if len(sys.argv) > 1 else r"C:/Program Files/Epic Games/rocketleague/TAGame/CookedPCConsole/Body_Octane_SF.upk")
KEYS = Path(__file__).parent.parent.parent / "src-tauri/resources/decryptor/keys.txt"

UPK_MAGIC = 0x9E2A83C1

def aes_decrypt(data, key):
    return Cipher(algorithms.AES(key), modes.ECB()).decryptor().update(data) + b""

buf = UPK.read_bytes()
print(f"file size: {len(buf)} bytes")
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
print(f"total_header={total_header:#x} names={name_count} imports={import_count} exports={export_count}")

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

# Find Texture2D class
t2d_idx = None
pos = import_offset - name_offset
for i in range(import_count):
    obj_name_idx = struct.unpack_from("<i", plain, pos+20)[0]
    if 0 <= obj_name_idx < len(names) and names[obj_name_idx] == "Texture2D":
        t2d_idx = -(i+1); break
    pos += 28

# Parse exports, find Pepe_Body_D
exports = []
pos = export_offset - name_offset
for i in range(export_count):
    class_idx = struct.unpack_from("<i", plain, pos)[0]
    name_idx = struct.unpack_from("<i", plain, pos+12)[0]
    serial_size = struct.unpack_from("<I", plain, pos+32)[0]
    serial_offset = struct.unpack_from("<Q", plain, pos+36)[0]
    net_count = struct.unpack_from("<i", plain, pos+48)[0]
    entry_size = 52 + net_count*4 + 16 + 4
    name = names[name_idx] if 0 <= name_idx < len(names) else ""
    exports.append({"idx": i, "class": class_idx, "name": name, "so": serial_offset, "ss": serial_size})
    pos += entry_size

print(f"\nTexture2D class idx: {t2d_idx}")
t2d_exports = [e for e in exports if e["class"] == t2d_idx]
print(f"Texture2D exports: {len(t2d_exports)}")
for e in t2d_exports[:50]:
    print(f"  export[{e['idx']:3}] {e['name']:35} serial=({e['so']:#x}, {e['ss']})")

pepe = next(e for e in exports if e["name"] == "Pepe_Body_D")
print(f"\nPepe_Body_D = export[{pepe['idx']}] serial=({pepe['so']:#x}, {pepe['ss']})")

# Decompress body
body = bytearray()
pp = total_header
chunks = []  # (file_off, c_size, body_start, u_size)
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
print(f"\nbody decompressed: {len(body)} bytes across {len(chunks)} zlib blocks (first block c_size={chunks[0][1]})")

# Find preamble length by voting across all Texture2D exports
votes = {}
for e in t2d_exports:
    base = e["so"] - total_header
    for k in range(0, 65536, 4):
        upos = base + k
        if upos < 0 or upos + 0x10c > len(body): continue
        if struct.unpack_from("<i", body, upos)[0] != -1: continue
        nm = struct.unpack_from("<i", body, upos + 0x108)[0]
        if 1 <= nm <= 24:
            votes[k] = votes.get(k, 0) + 1
preamble_len = min(k for k,v in votes.items() if v == max(votes.values()))
print(f"preamble_len={preamble_len:#x}")

# Parse Pepe_Body_D's serial data
body_pos = pepe["so"] - total_header + preamble_len
serial_end = body_pos + pepe["ss"]
print(f"\nPepe_Body_D body_pos={body_pos:#x} serial_end={serial_end:#x}")

# Locate which zlib block contains the start of Pepe_Body_D + its mip array
for ci, (foff, csz, bs, us) in enumerate(chunks):
    if bs <= body_pos < bs + us:
        print(f"  body_pos is in zlib block #{ci}: file_off={foff:#x} c_size={csz} body_start={bs:#x} u_size={us}")
        break

# Find TextureFileCacheName property
tfc_name_idx = names.index("TextureFileCacheName")
tag = struct.pack("<i", tfc_name_idx) + b"\0\0\0\0"
serial = body[body_pos:serial_end]
idx = serial.find(tag)
if idx >= 0:
    v_idx = struct.unpack_from("<i", serial, idx+24)[0]
    tfc_name = names[v_idx] if 0 <= v_idx < len(names) else f"?{v_idx}"
    print(f"  TextureFileCacheName = {tfc_name!r}  (name idx {v_idx})")

# Find num_mips (at body_pos + 0x108) and mip array
num_mips = struct.unpack_from("<i", body, body_pos + 0x108)[0]
print(f"  num_mips = {num_mips}")
mip_arr_off = body_pos + 0x10c
print(f"\n  Mip array starts at body[{mip_arr_off:#x}]:")
p = mip_arr_off
for i in range(min(num_mips, 12)):
    if p + 20 > serial_end:
        print(f"    mip {i}: TRUNCATED at body[{p:#x}]")
        break
    flags = struct.unpack_from("<I", body, p)[0]
    elem = struct.unpack_from("<I", body, p+4)[0]
    size_disk = struct.unpack_from("<I", body, p+8)[0]
    offset_in_file = struct.unpack_from("<Q", body, p+12)[0]
    in_tfc = bool(flags & 0x01)
    bc3_dim = int(round(elem ** 0.5))
    print(f"    mip {i}: body[{p:#x}] flags={flags:#x} {'TFC' if in_tfc else 'INLINE'} elem={elem} (~{bc3_dim}px) size_disk={size_disk} offset_in_file={offset_in_file:#x}")
    if in_tfc:
        p += 28
    else:
        p += 20 + size_disk
