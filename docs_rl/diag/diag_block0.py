"""Verify the modded UPK's block 0 recompression integrity.
Decompress block 0 from the modded UPK and check that it has the patched
mip array values. Compare to original donor block 0."""

import struct, zlib
from pathlib import Path

MODDED = Path(r"C:/Program Files/Epic Games/rocketleague/TAGame/CookedPCConsole/mods/Skin_Octane_Stars_SF.upk")
DONOR = Path(r"C:/Program Files/Epic Games/rocketleague/TAGame/CookedPCConsole/skin_octane_galefire_SF.upk")

UPK_MAGIC = 0x9E2A83C1

def decompress_chunk0_block0(path):
    buf = path.read_bytes()
    total_header_size = struct.unpack_from("<I", buf, 8)[0]
    h = total_header_size
    blk_size = struct.unpack_from("<I", buf, h + 4)[0]
    u_total = struct.unpack_from("<I", buf, h + 12)[0]
    nblocks = (u_total + blk_size - 1) // blk_size
    c_size0 = struct.unpack_from("<I", buf, h + 16)[0]
    data_off = h + 16 + nblocks * 8
    print(f"  chunk0: total_header={total_header_size:#x} blk_size={blk_size:#x} u_total={u_total:#x} nblocks={nblocks} c_size0={c_size0}")
    print(f"  block0 data offset in file: {data_off:#x}")
    compressed = buf[data_off:data_off + c_size0]
    # Look at the bytes after compressed end up to budget (= c_size0 from meta)
    # to check for zero padding (our code pads)
    decomp = zlib.decompress(compressed)
    return decomp, c_size0, compressed

print("--- MODDED ---")
modded_block0, modded_cs, modded_comp = decompress_chunk0_block0(MODDED)
print(f"  block0 decompressed: {len(modded_block0)} bytes (expected 131072)")
# Try to find the patched mip 0 offset bytes (0x19453de9 for Force_Body_D, 0x19464d3d for BlankSkin)
patched_d_offset = struct.pack("<Q", 0x19453de9)
patched_bs_offset = struct.pack("<Q", 0x19464d3d)
print(f"  Force_Body_D mip 0 offset bytes found at: {[i for i in range(len(modded_block0) - 8) if modded_block0[i:i+8] == patched_d_offset]}")
print(f"  BlankSkin mip 0 offset bytes found at: {[i for i in range(len(modded_block0) - 8) if modded_block0[i:i+8] == patched_bs_offset]}")

# Also verify the raw zlib data validates without padding issues
# Look at compressed buffer: is it terminated correctly?
print(f"  zlib stream (compressed): {len(modded_comp)} bytes (budget = {modded_cs})")
# Find the zlib end marker (0x00 0x00 ff ff is deflate flush, but the stream itself ends differently)
# Just check: do we have non-zero bytes followed by zero padding?
trailing_zeros = 0
for b in reversed(modded_comp):
    if b == 0:
        trailing_zeros += 1
    else:
        break
print(f"  trailing zero padding: {trailing_zeros} bytes")
print(f"  effective compressed data: {len(modded_comp) - trailing_zeros} bytes")
# Try to decompress just the effective data
try:
    eff_decomp = zlib.decompress(modded_comp[:len(modded_comp) - trailing_zeros])
    print(f"  effective data decompresses to: {len(eff_decomp)} bytes")
except Exception as e:
    print(f"  effective data decompression: {e}")

print("\n--- DONOR (original) ---")
donor_block0, donor_cs, donor_comp = decompress_chunk0_block0(DONOR)
print(f"  block0 decompressed: {len(donor_block0)} bytes")
trailing_zeros_d = 0
for b in reversed(donor_comp):
    if b == 0:
        trailing_zeros_d += 1
    else:
        break
print(f"  trailing zero padding: {trailing_zeros_d} bytes")
print(f"  effective compressed: {len(donor_comp) - trailing_zeros_d} bytes")

# Compare block0 bytes between modded and donor — should differ only at the patched mip fields
diff_ranges = []
in_diff = False
start = 0
for i in range(min(len(modded_block0), len(donor_block0))):
    if modded_block0[i] != donor_block0[i]:
        if not in_diff:
            start = i
            in_diff = True
    else:
        if in_diff:
            diff_ranges.append((start, i))
            in_diff = False
if in_diff:
    diff_ranges.append((start, len(modded_block0)))
print(f"\n--- Block0 diffs between modded and donor ---")
print(f"  total diff ranges: {len(diff_ranges)}")
for s, e in diff_ranges[:20]:
    print(f"    [{s:#x}, {e:#x}) — {e-s} bytes")
