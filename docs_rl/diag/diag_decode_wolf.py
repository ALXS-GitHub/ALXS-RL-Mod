"""Decode octane_team_wolf mip 0 from Textures3.tfc and save as PNG.
Verifies what the wolf decal art actually contains."""

import struct, zlib
from pathlib import Path

TFC = Path(r"C:/Program Files/Epic Games/rocketleague/TAGame/CookedPCConsole/Textures3.tfc")
OUT = Path(__file__).parent / "decoded_wolf.png"

# octane_team_wolf mip 0
OFFSET = 0x64054
SIZE = 87003

with TFC.open("rb") as f:
    f.seek(OFFSET)
    data = f.read(SIZE)

magic, blk_size, c_total, u_total = struct.unpack_from("<IIII", data, 0)
print(f"magic={magic:#x} blk_size={blk_size} c_total={c_total} u_total={u_total}")
nblocks = (u_total + blk_size - 1) // blk_size
metas_off = 16
cur = metas_off + nblocks * 8
raw = bytearray()
for i in range(nblocks):
    cs = struct.unpack_from("<I", data, metas_off + i*8)[0]
    raw += zlib.decompress(data[cur:cur+cs])
    cur += cs
print(f"Decompressed BC3 raw: {len(raw)} bytes")

# BC3 decode (16 bytes per 4x4 block: 8 alpha + 8 color)
w = h = 2048
out = bytearray(w * h * 4)
for by in range(h // 4):
    for bx in range(w // 4):
        block_off = (by * (w // 4) + bx) * 16
        a0 = raw[block_off]
        a1 = raw[block_off + 1]
        a_indices = int.from_bytes(raw[block_off+2:block_off+8], "little")
        if a0 > a1:
            alphas = [a0, a1] + [((7-i)*a0 + i*a1)//7 for i in range(1, 7)]
        else:
            alphas = [a0, a1] + [((5-i)*a0 + i*a1)//5 for i in range(1, 5)] + [0, 255]
        c0 = struct.unpack_from("<H", raw, block_off+8)[0]
        c1 = struct.unpack_from("<H", raw, block_off+10)[0]
        c_indices = int.from_bytes(raw[block_off+12:block_off+16], "little")
        def rgb565(c):
            r=(c>>11)&0x1F; g=(c>>5)&0x3F; b=c&0x1F
            return ((r<<3)|(r>>2), (g<<2)|(g>>4), (b<<3)|(b>>2))
        r0,g0,b0 = rgb565(c0); r1,g1,b1 = rgb565(c1)
        colors = [(r0,g0,b0),(r1,g1,b1),((2*r0+r1)//3,(2*g0+g1)//3,(2*b0+b1)//3),((r0+2*r1)//3,(g0+2*g1)//3,(b0+2*b1)//3)]
        for ty in range(4):
            for tx in range(4):
                idx = ty*4+tx
                a_idx = (a_indices >> (idx*3)) & 0x7
                c_idx = (c_indices >> (idx*2)) & 0x3
                r,g,b = colors[c_idx]
                a = alphas[a_idx]
                pix = (by*4+ty)*w + (bx*4+tx)
                out[pix*4:pix*4+4] = bytes([r,g,b,a])

# Save as PNG
def chunk(t,d):
    crc = zlib.crc32(t+d) & 0xffffffff
    return struct.pack(">I", len(d)) + t + d + struct.pack(">I", crc)
ihdr = struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)
raw_lines = b"".join(b"\x00" + bytes(out[y*w*4:(y+1)*w*4]) for y in range(h))
idat = zlib.compress(raw_lines)
with OUT.open("wb") as f:
    f.write(b"\x89PNG\r\n\x1a\n"); f.write(chunk(b"IHDR", ihdr)); f.write(chunk(b"IDAT", idat)); f.write(chunk(b"IEND", b""))
print(f"Wrote {OUT}")

# Sample some pixels to understand the texture content
import collections
print("\nSample pixels (random spread):")
samples = [(100,100),(500,500),(1024,1024),(1500,500),(1024,200),(200,1500),(1700,1500),(1024,1700)]
for x,y in samples:
    pix = (y*w + x)*4
    r,g,b,a = out[pix], out[pix+1], out[pix+2], out[pix+3]
    print(f"  ({x},{y}): R={r:3} G={g:3} B={b:3} A={a:3}")

# Channel histograms
print("\nChannel histograms (bucketed by /32):")
r_h = collections.Counter()
g_h = collections.Counter()
b_h = collections.Counter()
a_h = collections.Counter()
for i in range(0, len(out), 4):
    r_h[out[i]//32*32] += 1
    g_h[out[i+1]//32*32] += 1
    b_h[out[i+2]//32*32] += 1
    a_h[out[i+3]//32*32] += 1
n = w*h
for label,h in [("R",r_h),("G",g_h),("B",b_h),("A",a_h)]:
    print(f"  {label}:")
    for bucket in sorted(h.keys()):
        pct = 100*h[bucket]/n
        if pct >= 0.5:
            print(f"    {bucket:3}-{bucket+31:3}: {pct:.2f}%")
