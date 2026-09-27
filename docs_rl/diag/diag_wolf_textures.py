"""Enumerate all Texture2D exports in the Wolf donor UPK + their dimensions.
We need to know if Wolf has multiple local Texture2Ds we can use as
distinct Diffuse and Skin targets."""

import base64, struct, zlib
from pathlib import Path
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes

UPK = Path(r"C:/Program Files/Epic Games/rocketleague/TAGame/CookedPCConsole/skin_octane_wolf_SF.upk")
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

imports = []
pos = import_offset - name_offset
for i in range(import_count):
    class_idx = struct.unpack_from("<i", plain, pos+8)[0]
    obj_name_idx = struct.unpack_from("<i", plain, pos+20)[0]
    imports.append({"class": names[class_idx] if 0 <= class_idx < len(names) else "?",
                    "name": names[obj_name_idx] if 0 <= obj_name_idx < len(names) else "?"})
    pos += 28

t2d_imp = None
for i, imp in enumerate(imports):
    if imp["name"] == "Texture2D":
        t2d_imp = -(i+1); break
print(f"Texture2D import idx: {t2d_imp}")

mic_imp = None
for i, imp in enumerate(imports):
    if imp["name"] == "MaterialInstanceConstant":
        mic_imp = -(i+1); break

exports = []
pos = export_offset - name_offset
for _ in range(export_count):
    class_idx = struct.unpack_from("<i", plain, pos)[0]
    name_idx = struct.unpack_from("<i", plain, pos+12)[0]
    serial_size = struct.unpack_from("<I", plain, pos+32)[0]
    serial_offset = struct.unpack_from("<Q", plain, pos+36)[0]
    net_count = struct.unpack_from("<i", plain, pos+48)[0]
    entry_size = 52 + net_count*4 + 16 + 4
    exports.append({"class_idx": class_idx, "name": names[name_idx] if 0 <= name_idx < len(names) else "?",
                    "so": serial_offset, "ss": serial_size})
    pos += entry_size

print(f"Total exports: {len(exports)}")
print(f"\n--- All Texture2D exports ---")
for i, e in enumerate(exports):
    if e["class_idx"] == t2d_imp:
        print(f"  export[{i}]: {e['name']!r}  so={e['so']:#x} ss={e['ss']}")

print(f"\n--- All MIC exports ---")
for i, e in enumerate(exports):
    if e["class_idx"] == mic_imp:
        print(f"  export[{i}]: {e['name']!r}")

print(f"\n--- All exports by class ---")
class_counts = {}
for e in exports:
    cls = e["class_idx"]
    if cls < 0 and -cls - 1 < len(imports):
        cls_name = imports[-cls - 1]["name"]
    elif cls > 0 and cls - 1 < len(exports):
        cls_name = f"export[{cls-1}]={exports[cls-1]['name']}"
    else:
        cls_name = f"?{cls}"
    class_counts[cls_name] = class_counts.get(cls_name, 0) + 1
for cls, cnt in sorted(class_counts.items(), key=lambda x: -x[1]):
    print(f"  {cls}: {cnt}")
