"""Decode Pepe_Body_D's mip 0 BC1 data from Textures2.tfc and save as PNG.
Verifies our hijack actually wrote the user's diffuse content."""

import struct, zlib
from pathlib import Path

TFC = Path(r"C:/Program Files/Epic Games/rocketleague/TAGame/CookedPCConsole/Textures2.tfc")
OUT_DIR = Path(__file__).parent

# Pepe_Body_D mip 0
OFFSET = 0x6255c7f2
SIZE = 871730

with TFC.open("rb") as f:
    f.seek(OFFSET)
    data = f.read(SIZE)

magic, blk_size, c_total, u_total = struct.unpack_from("<IIII", data, 0)
print(f"magic={magic:#x} blk_size={blk_size} c_total={c_total} u_total={u_total}")
nblocks = (u_total + blk_size - 1) // blk_size
metas_off = 16
cur = metas_off + nblocks * 8

# Decompress all blocks
raw = bytearray()
for i in range(nblocks):
    cs = struct.unpack_from("<I", data, metas_off + i*8)[0]
    block = data[cur:cur+cs]
    raw += zlib.decompress(block)
    cur += cs

print(f"Decompressed BC1 raw: {len(raw)} bytes (expected {u_total})")

# Decode BC1 (8 bytes per 4x4 block)
w = h = 2048
out = bytearray(w * h * 4)
for by in range(h // 4):
    for bx in range(w // 4):
        block_off = (by * (w // 4) + bx) * 8
        c0 = struct.unpack_from("<H", raw, block_off)[0]
        c1 = struct.unpack_from("<H", raw, block_off + 2)[0]
        indices = int.from_bytes(raw[block_off+4:block_off+8], "little")

        def rgb565(c):
            r = (c >> 11) & 0x1F
            g = (c >> 5) & 0x3F
            b = c & 0x1F
            return ((r << 3) | (r >> 2), (g << 2) | (g >> 4), (b << 3) | (b >> 2))
        r0, g0, b0 = rgb565(c0)
        r1, g1, b1 = rgb565(c1)
        colors = [(r0, g0, b0, 255), (r1, g1, b1, 255)]
        if c0 > c1:
            colors.append(((2*r0+r1)//3, (2*g0+g1)//3, (2*b0+b1)//3, 255))
            colors.append(((r0+2*r1)//3, (g0+2*g1)//3, (b0+2*b1)//3, 255))
        else:
            colors.append(((r0+r1)//2, (g0+g1)//2, (b0+b1)//2, 255))
            colors.append((0, 0, 0, 0))  # transparent in BC1
        for ty in range(4):
            for tx in range(4):
                idx = (indices >> ((ty*4 + tx) * 2)) & 0x3
                px = by*4 + ty
                px_x = bx*4 + tx
                pix_off = (px * w + px_x) * 4
                out[pix_off:pix_off+4] = bytes(colors[idx])

# Save as PNG
def chunk(t, d):
    crc = zlib.crc32(t + d) & 0xffffffff
    return struct.pack(">I", len(d)) + t + d + struct.pack(">I", crc)

ihdr = struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)
raw_lines = b"".join(b"\x00" + bytes(out[y*w*4:(y+1)*w*4]) for y in range(h))
idat = zlib.compress(raw_lines)

out_path = OUT_DIR / "decoded_pepe_body_d.png"
with out_path.open("wb") as f:
    f.write(b"\x89PNG\r\n\x1a\n")
    f.write(chunk(b"IHDR", ihdr))
    f.write(chunk(b"IDAT", idat))
    f.write(chunk(b"IEND", b""))
print(f"Wrote {out_path}")
