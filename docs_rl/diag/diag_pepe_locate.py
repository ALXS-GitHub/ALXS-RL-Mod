"""TASK 2: Locate Pepe_Body_D's body data in Startup.upk's decompressed body.
Decompress all zlib chunks, derive preamble via TextureFileCacheName-tag
search (NameProperty pattern), then dump:
  - Pepe_Body_D body byte offset (start)
  - body offset of `TextureFileCacheName` property tag + ParameterValue
  - resolved TFC name
  - mip array NumMips and per-mip body offset breakdown (flags/elem/size_disk/offset_in_file)
  - which zlib block holds the mip array (file_off, c_size, body_start, u_size)
"""

import base64, struct, zlib
from pathlib import Path
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes

UPK = Path(r"C:/Program Files/Epic Games/rocketleague/TAGame/CookedPCConsole/Startup.upk")
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
if pepe is None:
    print("Pepe_Body_D NOT FOUND in exports")
    raise SystemExit(1)
print(f"Pepe_Body_D = export[{pepe[0]}] serial_offset={pepe[1]:#x} serial_size={pepe[2]}")

# Decompress body, recording chunk boundaries
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
print(f"body decompressed: {len(body)} bytes ({len(chunks)} chunks)")

# Approach: search for Pepe_Body_D's serial body via TextureFileCacheName tag pattern,
# AND via the well-known structure: NetIndex(-1) i32 + tagged-props + NumMips ...
# Strategy: the "expected" body offset (serial_offset - total_header) is correct only
# if preamble == 0. For Startup.upk it may differ. Brute-force: scan the +/- few MB
# window around expected_base for NameProperty<TextureFileCacheName> followed by a TFC name like Textures1..Textures9 or Textures10..

tfc_name_idx = names.index("TextureFileCacheName")
tag = struct.pack("<i", tfc_name_idx) + b"\0\0\0\0"
print(f"TextureFileCacheName FName idx = {tfc_name_idx} (tag bytes = {tag.hex()})")

# Validate by mapping all candidate TFC names
texture_candidates = [(i, n) for i, n in enumerate(names) if n.startswith("Textures") and len(n) <= 10]
print(f"Candidate TFC name table entries: {texture_candidates[:20]}")

expected_base = pepe[1] - total_header
print(f"Expected body base (without preamble): {expected_base:#x}")

# For Startup.upk, serial_offset is 1:1 with body[so - total_header]. No preamble.
preamble = 0
print(f"preamble = {preamble} (Startup.upk convention)")

# Now decode Pepe_Body_D
body_pos = pepe[1] - total_header + preamble
print(f"\n=== Pepe_Body_D body_pos = {body_pos:#x} ===")
serial = body[body_pos : body_pos + pepe[2]]
print(f"First 64 bytes (after NetIndex): {serial[:64].hex()}")

# Walk tagged-props
def walk_props(start_abs, end_abs):
    """Walk tagged props from start_abs to end_abs (absolute body offsets). Yields props until 'None'."""
    pp = start_abs
    props = []
    iters = 0
    while pp + 24 <= end_abs and iters < 200:
        n_idx = struct.unpack_from("<i", body, pp)[0]
        if not (0 <= n_idx < len(names)):
            break
        pname = names[n_idx]
        if pname == "None":
            return props, pp + 8
        t_idx = struct.unpack_from("<i", body, pp+8)[0]
        val_size = struct.unpack_from("<i", body, pp+16)[0]
        tname = names[t_idx] if 0 <= t_idx < len(names) else f"?{t_idx}"
        prop = {"name": pname, "type": tname, "off": pp, "val_size": val_size, "val_off": pp+24}
        props.append(prop)
        if tname == "ByteProperty":
            pp = pp + 24 + 8 + val_size
        elif tname == "BoolProperty":
            pp = pp + 24 + 1
        elif tname == "StructProperty":
            pp = pp + 24 + 8 + val_size
        else:
            pp = pp + 24 + val_size
        iters += 1
    return props, pp

props, prop_end = walk_props(body_pos + 4, body_pos + pepe[2])
print(f"\nTagged-properties of Pepe_Body_D ({len(props)} props, end @body[{prop_end:#x}]):")
tfc_prop_offset = None
tfc_value = None
for pr in props:
    extra = ""
    if pr["type"] == "NameProperty" and pr["val_size"] == 8:
        v_idx = struct.unpack_from("<i", body, pr["val_off"])[0]
        v_name = names[v_idx] if 0 <= v_idx < len(names) else f"?{v_idx}"
        extra = f" -> {v_name!r}"
        if pr["name"] == "TextureFileCacheName":
            tfc_prop_offset = pr["off"]
            tfc_value = v_name
    elif pr["type"] == "IntProperty" and pr["val_size"] == 4:
        extra = f" = {struct.unpack_from('<i', body, pr['val_off'])[0]}"
    elif pr["type"] == "BoolProperty":
        extra = f" = {bool(body[pr['val_off']])}"
    elif pr["type"] == "ByteProperty":
        enum_idx = struct.unpack_from("<i", body, pr["val_off"])[0]
        ev_idx = struct.unpack_from("<i", body, pr["val_off"]+8)[0]
        ev = names[ev_idx] if 0 <= ev_idx < len(names) else "?"
        extra = f" enum={names[enum_idx] if 0<=enum_idx<len(names) else '?'} val={ev}"
    print(f"  body[{pr['off']:#x}] {pr['name']!r} ({pr['type']}) vs={pr['val_size']}{extra}")

print(f"\nTextureFileCacheName property tag at body[{tfc_prop_offset:#x}]  value bytes at body[{tfc_prop_offset+24:#x}]  resolved -> {tfc_value!r}")

# After 'None', UE3 Texture2D has 4 bytes of unknown then NumMips (i32), then mip array
mip_meta_start = prop_end
# Standard UE3 Texture2D serial after tagged-props:
#   <unknown 4 bytes> + NumMips (i32) + mips[NumMips] (28 bytes each).
# In RL it's typically: 4 unknown bytes (sometimes 'flags'), then NumMips, then mips.
print(f"\nBytes after None (next 32): {bytes(body[mip_meta_start:mip_meta_start+32]).hex()}")

# Try NumMips at +0
candidates = []
for off in (0, 4, 8, 12, 16):
    nm = struct.unpack_from("<i", body, mip_meta_start + off)[0]
    if 1 <= nm <= 14:
        candidates.append((off, nm))
print(f"NumMips candidates (offset_after_None, value): {candidates}")

# Use the canonical "scan for 0x10003 mip flags pattern" approach
# Mip entry layout: flags(u32) + elem(u32) + size_disk(u32) + offset_in_file(u64) + size_uncompressed(u32) + uncompressed_size(u32)
# Total = 4+4+4+8+4+4 = 28 bytes
# Pattern: flags=0x10003 means stored in TFC.
mip_offsets = []
search_start = mip_meta_start
search_end = min(body_pos + pepe[2] + 4096, len(body) - 28)
i = search_start
while i < search_end:
    flags = struct.unpack_from("<I", body, i)[0]
    if flags == 0x10003:
        elem = struct.unpack_from("<I", body, i+4)[0]
        size_disk = struct.unpack_from("<I", body, i+8)[0]
        offset_in_file = struct.unpack_from("<Q", body, i+12)[0]
        size_unc = struct.unpack_from("<I", body, i+20)[0]
        bc3_dim = int(round(elem ** 0.5))
        if (1 <= bc3_dim <= 4096 and bc3_dim*bc3_dim == elem and 0 < size_disk < elem*2 and 0 < offset_in_file < 0xFFFFFFFF):
            mip_offsets.append((i, flags, elem, size_disk, offset_in_file, bc3_dim, size_unc))
    i += 4

# group consecutive 28-byte
groups = []
cur = []
for m in mip_offsets:
    if not cur:
        cur = [m]
    elif m[0] - cur[-1][0] == 28:
        cur.append(m)
    else:
        if len(cur) >= 3: groups.append(cur)
        cur = [m]
if len(cur) >= 3: groups.append(cur)

groups.sort(key=lambda g: abs(g[0][0] - expected_base))
print(f"\nMip array groups (count={len(groups)}):")
for gi, g in enumerate(groups[:3]):
    print(f"  group[{gi}]: {len(g)} mips, starts body[{g[0][0]:#x}], distance from expected_base={abs(g[0][0]-expected_base)}")

if groups:
    g = groups[0]
    # NumMips field is 4 bytes before first mip entry (UE3 standard FByteBulkData array preceded by NumMips i32)
    num_mips_off = g[0][0] - 4
    num_mips = struct.unpack_from("<i", body, num_mips_off)[0]
    print(f"\nNumMips at body[{num_mips_off:#x}] = {num_mips}")
    print(f"Mip array entries ({len(g)} of declared {num_mips}):")
    for body_off, flags, elem, sd, oif, dim, su in g:
        rel = body_off - body_pos
        print(f"  body[{body_off:#x}] (+{rel:#x} from pepe) flags={flags:#x} elem={elem} (~{dim}px) size_disk={sd} offset_in_file={oif:#x} size_unc={su}")

    # Locate the zlib block(s) containing the mip array
    first_mip = g[0][0]
    last_mip_end = g[-1][0] + 28
    chunks_holding = []
    for ci, (foff, csz, bs, us) in enumerate(chunks):
        if bs <= first_mip < bs + us or bs <= last_mip_end < bs + us or (first_mip < bs and last_mip_end > bs+us):
            chunks_holding.append((ci, foff, csz, bs, us))
    print(f"\nZlib blocks containing the mip array (body[{first_mip:#x}..{last_mip_end:#x}]):")
    for ci, foff, csz, bs, us in chunks_holding:
        print(f"  block[{ci}] file_off={foff:#x} c_size={csz} body_start={bs:#x} u_size={us}")

    # Also locate the block holding Pepe_Body_D's body_pos (start)
    for ci, (foff, csz, bs, us) in enumerate(chunks):
        if bs <= body_pos < bs + us:
            print(f"\nPepe_Body_D BODY START in zlib block #{ci}: file_off={foff:#x} c_size={csz} body_start={bs:#x} u_size={us}")
            break
    # And the block holding TextureFileCacheName tag
    if tfc_prop_offset:
        for ci, (foff, csz, bs, us) in enumerate(chunks):
            if bs <= tfc_prop_offset < bs + us:
                print(f"TextureFileCacheName TAG in zlib block #{ci}: file_off={foff:#x} c_size={csz} body_start={bs:#x} u_size={us}")
                break
