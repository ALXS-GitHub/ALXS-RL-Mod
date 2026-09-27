"""Scan every UPK in CookedPCConsole for Pepe_Body_D as an EXPORT
(not just an import reference). The UPK that exports it is where the
actual Texture2D data lives, and that's the file we'd need to hijack."""

import base64, struct
from pathlib import Path
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes

COOKED = Path(r"C:/Program Files/Epic Games/rocketleague/TAGame/CookedPCConsole")
KEYS = Path(__file__).parent.parent.parent / "src-tauri/resources/decryptor/keys.txt"

keys = [base64.b64decode(line.strip()) for line in KEYS.read_text().splitlines() if line.strip()]

def aes_decrypt(data, key):
    return Cipher(algorithms.AES(key), modes.ECB()).decryptor().update(data) + b""

def check_upk(path):
    try:
        buf = path.read_bytes()
        if len(buf) < 1024 or struct.unpack_from("<I", buf, 0)[0] != 0x9E2A83C1:
            return None
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
        # Parse exports, look for Pepe_Body_D
        pos = export_offset - name_offset
        for _ in range(export_count):
            if pos + 52 > len(plain): break
            name_idx = struct.unpack_from("<i", plain, pos+12)[0]
            net_count = struct.unpack_from("<i", plain, pos+48)[0]
            entry_size = 52 + net_count*4 + 16 + 4
            if 0 <= name_idx < len(names) and names[name_idx] == "Pepe_Body_D":
                return "EXPORT"
            pos += entry_size
        # Also check if it's in the name table at all (could be a redirected import host)
        if "Pepe_Body_D" in names:
            return "name_table_only"
        return None
    except Exception:
        return None

# Scan body texture UPKs first (most likely)
candidates = sorted(f for f in COOKED.iterdir() if f.is_file() and f.suffix == ".upk")
print(f"Scanning {len(candidates)} UPKs for Pepe_Body_D export...")
exports_in = []
namesonly_in = []
for i, f in enumerate(candidates):
    result = check_upk(f)
    if result == "EXPORT":
        exports_in.append(f.name)
        print(f"  EXPORT found in: {f.name}")
    elif result == "name_table_only":
        namesonly_in.append(f.name)
    if (i+1) % 200 == 0:
        print(f"  ... scanned {i+1}/{len(candidates)}")
print(f"\nDone. Exports in {len(exports_in)} UPKs. Just in name table of {len(namesonly_in)} more.")
