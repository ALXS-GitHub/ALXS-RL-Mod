"""Diag: for each Texture2D export in the donor UPK, list every mip's
flags and whether it's INLINE or TFC-stored.

Goal: confirm hypothesis that `Force_Body_D` in GaleFire donor is INLINE
(not TFC-stored), which would explain why our pipeline never swaps it
and the user sees blank/white in literal-diffuse zones."""

import base64, struct, zlib, sys
from pathlib import Path
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes

UPK_PATH = Path(r"C:/Program Files/Epic Games/rocketleague/TAGame/CookedPCConsole/skin_octane_galefire_SF.upk")
KEYS_PATH = Path(__file__).parent / "src-tauri/resources/decryptor/keys.txt"

UPK_MAGIC = 0x9E2A83C1
AES_BLOCK = 16
BULKDATA_STORE_IN_SEPARATE_FILE = 0x01

def aes_decrypt(data, key):
    c = Cipher(algorithms.AES(key), modes.ECB())
    d = c.decryptor()
    return d.update(data) + d.finalize()

buf = UPK_PATH.read_bytes()
assert struct.unpack_from("<I", buf, 0)[0] == UPK_MAGIC, "bad magic"
total_header_size = struct.unpack_from("<I", buf, 8)[0]
folder_len_signed = struct.unpack_from("<i", buf, 12)[0]
folder_len = abs(folder_len_signed)
p = 16 + folder_len + 4
name_count = struct.unpack_from("<I", buf, p)[0]; p += 4
name_offset = struct.unpack_from("<I", buf, p)[0]; p += 4
export_count = struct.unpack_from("<I", buf, p)[0]; p += 4
export_offset = struct.unpack_from("<I", buf, p)[0]; p += 4
import_count = struct.unpack_from("<I", buf, p)[0]; p += 4
import_offset = struct.unpack_from("<I", buf, p)[0]; p += 4
region_len = (total_header_size - name_offset) & ~(AES_BLOCK - 1)
print(f"total_header={total_header_size:#x} name_offset={name_offset:#x} region_len={region_len:#x}")
print(f"name_count={name_count} import_count={import_count} export_count={export_count}")

# Try each key
keys = [base64.b64decode(line.strip()) for line in KEYS_PATH.read_text().splitlines() if line.strip()]
region = buf[name_offset:name_offset+region_len]
plain = None
for key in keys:
    cand = aes_decrypt(region, key)
    n = struct.unpack_from("<i", cand, 0)[0]
    if 1 <= n <= 256 and all(b == 0 or 32 <= b < 127 for b in cand[4:4+n]):
        plain = cand
        print(f"matched key: {base64.b64encode(key).decode()[:16]}…")
        break
assert plain, "no key matched"

# Parse names
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

# Find Texture2D class idx
texture2d_idx = None
pos = import_offset - name_offset
for i in range(import_count):
    obj_name_idx = struct.unpack_from("<i", plain, pos+20)[0]
    if 0 <= obj_name_idx < len(names) and names[obj_name_idx] == "Texture2D":
        texture2d_idx = -(i + 1)
        break
    pos += 28
print(f"Texture2D class idx: {texture2d_idx}")

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
        exports.append((names[name_idx] if 0 <= name_idx < len(names) else "?", serial_offset, serial_size))
    pos += entry_size

print(f"\nTexture2D exports: {len(exports)}")
for name, so, ss in exports:
    print(f"  {name}: serial_offset={so:#x} size={ss}")

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
print(f"\ndecompressed body: {len(body)} bytes")

# Find preamble length via voting
votes = {}
for name, so, ss in exports:
    base = so - total_header_size
    for k in range(0, 65536, 4):
        upos = base + k
        if upos < 0 or upos + 0x10c > len(body):
            continue
        netindex = struct.unpack_from("<i", body, upos)[0]
        if netindex != -1:
            continue
        num_mips = struct.unpack_from("<i", body, upos + 0x108)[0]
        if 1 <= num_mips <= 24:
            votes[k] = votes.get(k, 0) + 1
preamble_len = min(k for k, v in votes.items() if v == len(exports)) if votes else 0
print(f"preamble_len: {preamble_len:#x}")

# For each export, walk mips
print(f"\n--- Texture2D mip analysis ---")
for name, so, ss in exports:
    body_pos = so - total_header_size + preamble_len
    netindex = struct.unpack_from("<i", body, body_pos)[0]
    num_mips = struct.unpack_from("<i", body, body_pos + 0x108)[0]
    serial_end = body_pos + ss
    print(f"\n{name}: netindex={netindex} num_mips={num_mips}")
    p = body_pos + 0x10c
    for mip_i in range(num_mips):
        if p + 20 > serial_end:
            print(f"  mip {mip_i}: TRUNCATED")
            break
        flags = struct.unpack_from("<I", body, p)[0]
        elem = struct.unpack_from("<I", body, p+4)[0]
        size_disk = struct.unpack_from("<I", body, p+8)[0]
        offset_in_file = struct.unpack_from("<Q", body, p+12)[0]
        in_tfc = bool(flags & BULKDATA_STORE_IN_SEPARATE_FILE)
        bc3_dim = int(round(elem ** 0.5))
        storage = "TFC" if in_tfc else "INLINE"
        print(f"  mip {mip_i}: flags={flags:#x} {storage} elem={elem} (~{bc3_dim}px) size_disk={size_disk} offset_in_file={offset_in_file:#x}")
        if in_tfc:
            p += 28
        else:
            # inline mip: 20-byte header + size_disk bytes inline
            p += 20 + size_disk
