"""Scan every Octane skin UPK in CookedPCConsole to find which ones
reference Pepe_Body_D vs Octane_Body_D vs other body diffuse textures.
This tells us whether hijacking Pepe_Body_D's source UPK would affect
all Octane decals (bad) or only Force-style decals like GaleFire (ok)."""

import base64, struct
from pathlib import Path
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes

COOKED = Path(r"C:/Program Files/Epic Games/rocketleague/TAGame/CookedPCConsole")
KEYS = Path(__file__).parent.parent.parent / "src-tauri/resources/decryptor/keys.txt"

keys = [base64.b64decode(line.strip()) for line in KEYS.read_text().splitlines() if line.strip()]

def aes_decrypt(data, key):
    return Cipher(algorithms.AES(key), modes.ECB()).decryptor().update(data) + b""

def parse_upk_imports(path):
    try:
        buf = path.read_bytes()
        if struct.unpack_from("<I", buf, 0)[0] != 0x9E2A83C1:
            return None
        total_header = struct.unpack_from("<I", buf, 8)[0]
        folder_len = abs(struct.unpack_from("<i", buf, 12)[0])
        p = 16 + folder_len + 4
        name_count = struct.unpack_from("<I", buf, p)[0]; p += 4
        name_offset = struct.unpack_from("<I", buf, p)[0]; p += 4
        _ec = struct.unpack_from("<I", buf, p)[0]; p += 4
        _eo = struct.unpack_from("<I", buf, p)[0]; p += 4
        import_count = struct.unpack_from("<I", buf, p)[0]; p += 4
        import_offset = struct.unpack_from("<I", buf, p)[0]
        region_len = (total_header - name_offset) & ~15
        if region_len <= 0:
            return None
        region = buf[name_offset:name_offset+region_len]
        plain = None
        for key in keys:
            cand = aes_decrypt(region, key)
            n = struct.unpack_from("<i", cand, 0)[0]
            if 1 <= n <= 256 and all(b == 0 or 32 <= b < 127 for b in cand[4:4+n]):
                plain = cand; break
        if plain is None:
            return None
        names = []
        pos = 0
        for _ in range(name_count):
            if pos + 4 > len(plain): break
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
        for _ in range(import_count):
            if pos + 28 > len(plain): break
            obj_name_idx = struct.unpack_from("<i", plain, pos+20)[0]
            if 0 <= obj_name_idx < len(names):
                imports.append(names[obj_name_idx])
            pos += 28
        return imports
    except Exception as e:
        return None

# Scan all Octane skin UPKs
octane_skins = sorted(
    f for f in COOKED.iterdir()
    if f.is_file() and f.suffix == ".upk" and (f.name.lower().startswith("skin_octane") or f.name.lower() == "body_octane_sf.upk")
)

print(f"Scanning {len(octane_skins)} Octane-related UPKs...")
results = {"Pepe_Body_D": [], "Octane_Body_D": [], "Force_Body_D": [], "neither": []}
for upk in octane_skins:
    imports = parse_upk_imports(upk)
    if imports is None:
        continue
    refs = []
    if "Pepe_Body_D" in imports: refs.append("Pepe_Body_D")
    if "Octane_Body_D" in imports: refs.append("Octane_Body_D")
    if "Force_Body_D" in imports: refs.append("Force_Body_D")
    if not refs:
        results["neither"].append(upk.name)
    else:
        for r in refs:
            results[r].append(upk.name)

for key in ("Pepe_Body_D", "Octane_Body_D", "Force_Body_D", "neither"):
    bucket = results[key]
    print(f"\n--- {key}: {len(bucket)} UPKs ---")
    for name in bucket:
        print(f"  {name}")
