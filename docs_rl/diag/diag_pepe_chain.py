"""Trace the full import chain for Pepe_Body_D in the donor UPK so we
know which source UPK actually contains it. If it's a small/dedicated
UPK we can hijack it without the global Startup.upk side effects."""

import base64, struct
from pathlib import Path
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes

UPK = Path(r"C:/Program Files/Epic Games/rocketleague/TAGame/CookedPCConsole/skin_octane_galefire_SF.upk")
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

# Parse all imports
imports = []
pos = import_offset - name_offset
for i in range(import_count):
    pkg_idx = struct.unpack_from("<i", plain, pos)[0]
    pkg_num = struct.unpack_from("<i", plain, pos+4)[0]
    class_idx = struct.unpack_from("<i", plain, pos+8)[0]
    class_num = struct.unpack_from("<i", plain, pos+12)[0]
    outer_idx = struct.unpack_from("<i", plain, pos+16)[0]
    obj_name_idx = struct.unpack_from("<i", plain, pos+20)[0]
    obj_name_num = struct.unpack_from("<i", plain, pos+24)[0]
    imports.append({
        "idx": i,
        "pkg": names[pkg_idx] if 0 <= pkg_idx < len(names) else f"?{pkg_idx}",
        "class": names[class_idx] if 0 <= class_idx < len(names) else f"?{class_idx}",
        "outer": outer_idx,
        "name": names[obj_name_idx] if 0 <= obj_name_idx < len(names) else f"?{obj_name_idx}",
    })
    pos += 28

def resolve_outer(idx):
    """Recursively resolve an outer chain to a list of names."""
    chain = []
    cur = idx
    iters = 0
    while cur != 0 and iters < 20:
        if cur > 0:
            chain.append(f"export[{cur-1}]?")
            break
        i = -cur - 1
        if not (0 <= i < len(imports)):
            chain.append(f"?{cur}")
            break
        imp = imports[i]
        chain.append(f"{imp['name']}/{imp['class']} (from {imp['pkg']})")
        cur = imp["outer"]
        iters += 1
    return chain

# Find Pepe_Body_D
print("--- All imports referencing 'Pepe' or 'Body' or 'Octane' ---")
for imp in imports:
    if "Pepe" in imp["name"] or "Octane" in imp["name"] or "Body_D" in imp["name"]:
        chain = resolve_outer(imp["outer"])
        print(f"  import[{imp['idx']:3}] {imp['name']:30} ({imp['class']:20}) pkg={imp['pkg']:30} outer chain:")
        for c in chain:
            print(f"      {c}")
