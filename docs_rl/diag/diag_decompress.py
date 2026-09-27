"""Decompress what's at the patched offsets in MyDecal_.tfc to verify
the BC3 data RL would see. Output as PNG so we can visually inspect."""

import struct, zlib
from pathlib import Path

TFC_DIFFUSE = Path(r"C:/Program Files/Epic Games/rocketleague/TAGame/CookedPCConsole/MyDecal_.tfc")
TFC_MASK = Path(r"C:/Program Files/Epic Games/rocketleague/TAGame/CookedPCConsole/MyDecal02.tfc")
OUT_DIR = Path(__file__).parent

UPK_MAGIC = 0x9E2A83C1

def read_chunked_zlib(f, start, length):
    f.seek(start)
    data = f.read(length)
    magic, blk_size, c_total, u_total = struct.unpack_from("<IIII", data, 0)
    assert magic == UPK_MAGIC, f"bad magic {magic:#x}"
    nblocks = (u_total + blk_size - 1) // blk_size
    metas_off = 16
    cur = metas_off + nblocks * 8
    out = bytearray()
    for i in range(nblocks):
        c_size, u_size = struct.unpack_from("<II", data, metas_off + i * 8)
        block = data[cur:cur + c_size]
        decompressed = zlib.decompress(block)
        out += decompressed
        cur += c_size
    return bytes(out), {"blk_size": blk_size, "c_total": c_total, "u_total": u_total, "nblocks": nblocks}


def bc3_decode_to_rgba(bc3, w, h):
    """Decode BC3/DXT5 to RGBA bytes. Each 4x4 block = 16 bytes."""
    out = bytearray(w * h * 4)
    blocks_x = w // 4
    blocks_y = h // 4
    for by in range(blocks_y):
        for bx in range(blocks_x):
            block_off = (by * blocks_x + bx) * 16
            # Alpha block (BC4)
            a0 = bc3[block_off]
            a1 = bc3[block_off + 1]
            a_indices_bytes = bc3[block_off + 2:block_off + 8]
            a_indices = int.from_bytes(a_indices_bytes, "little")
            alphas = []
            if a0 > a1:
                alphas = [a0, a1] + [((7 - i) * a0 + i * a1) // 7 for i in range(1, 7)]
            else:
                alphas = [a0, a1] + [((5 - i) * a0 + i * a1) // 5 for i in range(1, 5)] + [0, 255]
            # Color block (BC1)
            c0 = struct.unpack_from("<H", bc3, block_off + 8)[0]
            c1 = struct.unpack_from("<H", bc3, block_off + 10)[0]
            def rgb565(c):
                r = (c >> 11) & 0x1f
                g = (c >> 5) & 0x3f
                b = c & 0x1f
                return ((r << 3) | (r >> 2), (g << 2) | (g >> 4), (b << 3) | (b >> 2))
            r0, g0, b0 = rgb565(c0)
            r1, g1, b1 = rgb565(c1)
            colors = [(r0, g0, b0), (r1, g1, b1)]
            if c0 > c1:
                colors.append(((2 * r0 + r1) // 3, (2 * g0 + g1) // 3, (2 * b0 + b1) // 3))
                colors.append(((r0 + 2 * r1) // 3, (g0 + 2 * g1) // 3, (b0 + 2 * b1) // 3))
            else:
                colors.append(((r0 + r1) // 2, (g0 + g1) // 2, (b0 + b1) // 2))
                colors.append((0, 0, 0))
            c_indices_bytes = bc3[block_off + 12:block_off + 16]
            c_indices = int.from_bytes(c_indices_bytes, "little")
            for ty in range(4):
                for tx in range(4):
                    pix_x = bx * 4 + tx
                    pix_y = by * 4 + ty
                    pix_idx = ty * 4 + tx
                    a_idx = (a_indices >> (pix_idx * 3)) & 0x7
                    c_idx = (c_indices >> (pix_idx * 2)) & 0x3
                    r, g, b = colors[c_idx]
                    a = alphas[a_idx]
                    pix_off = (pix_y * w + pix_x) * 4
                    out[pix_off:pix_off + 4] = bytes([r, g, b, a])
    return bytes(out)


def save_png(rgba, w, h, path):
    try:
        from PIL import Image
        Image.frombytes("RGBA", (w, h), rgba).save(path)
        print(f"  wrote {path}")
    except ImportError:
        # Manually write PNG
        import struct, zlib
        def chunk(t, d):
            crc = zlib.crc32(t + d) & 0xffffffff
            return struct.pack(">I", len(d)) + t + d + struct.pack(">I", crc)
        ihdr = struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)
        # Build raw scanlines with filter byte 0
        raw = b"".join(b"\x00" + rgba[y * w * 4:(y + 1) * w * 4] for y in range(h))
        idat = zlib.compress(raw)
        with open(path, "wb") as f:
            f.write(b"\x89PNG\r\n\x1a\n")
            f.write(chunk(b"IHDR", ihdr))
            f.write(chunk(b"IDAT", idat))
            f.write(chunk(b"IEND", b""))
        print(f"  wrote {path}")


# Reads patched offsets from current diag_modded.py output (current dual-TFC state).
DIFFUSE_OFFSET = 0x25b9f92   # Force_Body_D in MyDecal_.tfc
DIFFUSE_SIZE   = 47959
MASK_OFFSET    = 0x19453de9  # Skin_Octane_Stars_RGB in MyDecal02.tfc
MASK_SIZE      = 16158

with TFC_DIFFUSE.open("rb") as f:
    print(f"--- Force_Body_D (diffuse) at MyDecal_.tfc:{DIFFUSE_OFFSET:#x} ---")
    bc3, meta = read_chunked_zlib(f, DIFFUSE_OFFSET, DIFFUSE_SIZE)
    print(f"  meta: {meta}, decompressed: {len(bc3)} bytes")
    if len(bc3) == 4194304:
        rgba = bc3_decode_to_rgba(bc3, 2048, 2048)
        save_png(rgba, 2048, 2048, OUT_DIR / "decoded_diffuse.png")

with TFC_MASK.open("rb") as f:
    print(f"--- Skin_Octane_*_RGB (mask) at MyDecal02.tfc:{MASK_OFFSET:#x} ---")
    bc3, meta = read_chunked_zlib(f, MASK_OFFSET, MASK_SIZE)
    print(f"  meta: {meta}, decompressed: {len(bc3)} bytes")
    if len(bc3) == 4194304:
        rgba = bc3_decode_to_rgba(bc3, 2048, 2048)
        save_png(rgba, 2048, 2048, OUT_DIR / "decoded_mask.png")
