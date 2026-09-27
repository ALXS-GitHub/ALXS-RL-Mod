"""Robust Pepe_Body_D inspector in Startup.upk: skip the fragile
0x108 offset assumption and locate mip array via 0x10003 flags scan."""

import base64, struct, zlib
from pathlib import Path
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes

UPK = Path(r"C:/Program Files/Epic Games/rocketleague/TAGame/CookedPCConsole/Startup.upk")
KEYS = Path(__file__).parent.parent.parent / "src-tauri/resources/decryptor/keys.txt"

UPK_MAGIC = 0x9E2A83C1

def aes_decrypt(data, key):
    return Cipher(algorithms.AES(key), modes.ECB()).decryptor().update(data) + b""

buf = UPK.read_bytes()
print(f"file size: {len(buf)} = {len(buf)/(1024*1024):.1f} MB")
total_header = struct.unpack_from("<I", buf, 8)[0]
folder_len = abs(struct.unpack_from("<i", buf, 12)[0])
p = 16 + folder_len + 4
name_count = struct.unpack_from("<I", buf, p)[0]; p += 4
name_offset = struct.unpack_from("<I", buf, p)[0]; p += 4
export_count = struct.unpack_from("<I", buf, p)[0]; p += 4
export_offset = struct.unpack_from("<I", buf, p)[0]; p += 4
import_count = struct.unpack_from("<I", buf, p)[0]; p += 4
_io = struct.unpack_from("<I", buf, p)[0]
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

# Find Pepe_Body_D export
pepe = None
pos = export_offset - name_offset
for i in range(export_count):
    name_idx = struct.unpack_from("<i", plain, pos+12)[0]
    serial_size = struct.unpack_from("<I", plain, pos+32)[0]
    serial_offset = struct.unpack_from("<Q", plain, pos+36)[0]
    net_count = struct.unpack_from("<i", plain, pos+48)[0]
    entry_size = 52 + net_count*4 + 16 + 4
    if 0 <= name_idx < len(names) and names[name_idx] == "Pepe_Body_D":
        pepe = (i, serial_offset, serial_size)
        break
    pos += entry_size
print(f"Pepe_Body_D = export[{pepe[0]}] serial=({pepe[1]:#x}, {pepe[2]})")

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

# Find Pepe_Body_D's serial in body — search for likely body_pos by scanning
# for the FName ref of "Pepe_Body_D" near the expected location.
# Actually simpler: scan a window around serial_offset - total_header for the
# 0x10003 mip pattern.
expected_base = pepe[1] - total_header  # rough body offset before preamble correction
print(f"\nSearching for mip array (flags=0x10003) near body[{expected_base:#x}] +/- 64KB")

mips_found = []
search_start = max(0, expected_base - 0x10000)
search_end = min(len(body) - 28, expected_base + 0x10000)
i = search_start
while i < search_end:
    flags = struct.unpack_from("<I", body, i)[0]
    if flags == 0x10003:
        elem = struct.unpack_from("<I", body, i+4)[0]
        size_disk = struct.unpack_from("<I", body, i+8)[0]
        offset_in_file = struct.unpack_from("<Q", body, i+12)[0]
        # BC3 validity: dim*dim == elem, dim is power of 2
        bc3_dim = int(round(elem ** 0.5))
        if (4 <= bc3_dim <= 4096 and bc3_dim * bc3_dim == elem
            and 0 < size_disk < elem * 2
            and 0 < offset_in_file < 0xFFFFFFFF):
            mips_found.append((i, flags, elem, size_disk, offset_in_file, bc3_dim))
        i += 4
    else:
        i += 4

# Group consecutive mips (28 bytes apart)
print(f"  Found {len(mips_found)} potential TFC mip entries in search window")
groups = []
current = []
for m in mips_found:
    if not current:
        current = [m]
    elif m[0] - current[-1][0] == 28:
        current.append(m)
    else:
        if len(current) >= 3:
            groups.append(current)
        current = [m]
if len(current) >= 3:
    groups.append(current)

# Pick the group whose first mip is closest to expected_base
def distance(g):
    return abs(g[0][0] - expected_base)
groups.sort(key=distance)
if not groups:
    print("No mip array group found!")
else:
    g = groups[0]
    print(f"\nBest group: {len(g)} consecutive TFC mips, starts at body[{g[0][0]:#x}] (distance from expected: {distance(g)})")
    for body_off, flags, elem, sd, oif, dim in g:
        print(f"  body[{body_off:#x}] flags={flags:#x} elem={elem} (~{dim}px) size_disk={sd} offset_in_file={oif:#x}")

    # Locate the zlib block containing the FIRST mip entry
    target = g[0][0]
    for ci, (foff, csz, bs, us) in enumerate(chunks):
        if bs <= target < bs + us:
            print(f"\nFirst mip is in zlib block #{ci}: file_off={foff:#x} c_size={csz} body_start={bs:#x} u_size={us}")
            break

# Also extract TextureFileCacheName by searching backward from the mip array
# (it's a property in the tagged-property block before the mip array start).
tfc_name_idx_in_table = names.index("TextureFileCacheName")
print(f"\n'TextureFileCacheName' FName idx in name table = {tfc_name_idx_in_table}")
tag = struct.pack("<i", tfc_name_idx_in_table) + b"\0\0\0\0"
# Search a 64KB window before the first mip
if groups:
    first_mip = groups[0][0][0]
    win_start = max(0, first_mip - 0x10000)
    found_at = body.find(tag, win_start, first_mip)
    if found_at >= 0:
        v_idx = struct.unpack_from("<i", body, found_at + 24)[0]
        tfc_name = names[v_idx] if 0 <= v_idx < len(names) else f"?{v_idx}"
        print(f"  TextureFileCacheName property found at body[{found_at:#x}] -> {tfc_name!r} (name idx {v_idx})")
