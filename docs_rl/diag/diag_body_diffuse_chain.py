"""End-to-end body-diffuse investigation.

Goals:
  1. Dump Body_All_Mat (Startup.upk export[260]) sub-expressions, esp.
     TextureSampleParameter2D nodes -> ParameterName/Group/ExpressionGUID/
     DefaultTexture. Detect the Diffuse GUID 71B78637-5FAD-894A-889C-A31EEC008A34.
  2. Dump MIC_Body_Octane (Startup.upk export[786]) -> Parent + ALL parameter arrays.
  3. Scan Body_Octane_SF.upk for body-related MICs (MIC_Body_Octane_* / Octane_*_Body_MIC).
     Dump their Parent + parameter arrays.
  4. List Texture2D exports in Startup.upk and Body_Octane_SF.upk whose names look like
     body-diffuse textures (Octane_Body_D, Pepe_Body_D, *_Body_D, Body_*_Octane).
     For each, capture the TextureFileCacheName + first mip offset_in_file/size.
  5. Walk full parent chain from modded Skin_Octane_Stars_SF.Octane_Stars_MIC up to first non-MIC.
  6. For the donor MIC's Diffuse GUID 71B78637-..., test whether Body_All_Mat
     defines a sub-expression with that GUID.

Output: a comprehensive printable trace + JSON dump at docs_rl/diag/body_diffuse_chain.json.
"""

import base64, json, struct, zlib
from pathlib import Path
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes

RL = Path(r"C:/Program Files/Epic Games/rocketleague/TAGame/CookedPCConsole")
KEYS = Path(__file__).parent.parent.parent / "src-tauri/resources/decryptor/keys.txt"
UPK_MAGIC = 0x9E2A83C1

DIFFUSE_GUID_STR = "71B78637-5FAD-894A-889C-A31EEC008A34"
SKIN_GUID_STR    = "F6B52F25-42AB-BD49-AA45-5F91F981F5C9"


def make_guid_variants(s):
    h = s.replace("-", "")
    raw = bytes.fromhex(h)
    return [
        ("MS GUID layout",   bytes(reversed(raw[:4])) + bytes(reversed(raw[4:6])) + bytes(reversed(raw[6:8])) + raw[8:]),
        ("UE3 4xUInt32-LE",  b"".join(bytes(reversed(raw[i:i+4])) for i in range(0,16,4))),
        ("raw bytes",        raw),
    ]


def aes_decrypt(data, key):
    return Cipher(algorithms.AES(key), modes.ECB()).decryptor().update(data) + b""


KEYS_LIST = [base64.b64decode(line.strip()) for line in KEYS.read_text().splitlines() if line.strip()]


class Upk:
    def __init__(self, path):
        self.path = Path(path)
        buf = self.path.read_bytes()
        self.buf = buf
        self.total_header = struct.unpack_from("<I", buf, 8)[0]
        folder_len = abs(struct.unpack_from("<i", buf, 12)[0])
        p = 16 + folder_len + 4
        self.name_count    = struct.unpack_from("<I", buf, p)[0]; p += 4
        self.name_offset   = struct.unpack_from("<I", buf, p)[0]; p += 4
        self.export_count  = struct.unpack_from("<I", buf, p)[0]; p += 4
        self.export_offset = struct.unpack_from("<I", buf, p)[0]; p += 4
        self.import_count  = struct.unpack_from("<I", buf, p)[0]; p += 4
        self.import_offset = struct.unpack_from("<I", buf, p)[0]
        region_len = (self.total_header - self.name_offset) & ~15
        region = buf[self.name_offset:self.name_offset+region_len]

        plain = None
        for key in KEYS_LIST:
            cand = aes_decrypt(region, key)
            n = struct.unpack_from("<i", cand, 0)[0]
            if 1 <= n <= 256 and all(b == 0 or 32 <= b < 127 for b in cand[4:4+n]):
                plain = cand; break
        if plain is None:
            raise RuntimeError(f"could not decrypt header of {path}")
        self.plain = plain

        # Names
        self.names = []
        pos = 0
        for _ in range(self.name_count):
            n = struct.unpack_from("<i", plain, pos)[0]; pos += 4
            if n > 0:
                s = plain[pos:pos+n-1].rstrip(b"\0").decode("latin-1", errors="replace")
                self.names.append(s); pos += n + 8
            elif n < 0:
                s = plain[pos:pos+(-n)*2].decode("utf-16-le").rstrip("\0")
                self.names.append(s); pos += (-n)*2 + 8
            else:
                self.names.append(""); pos += 8

        # Imports
        self.imports = []
        pos = self.import_offset - self.name_offset
        for i in range(self.import_count):
            pkg_idx       = struct.unpack_from("<i", plain, pos)[0]
            class_idx     = struct.unpack_from("<i", plain, pos+8)[0]
            outer_idx     = struct.unpack_from("<i", plain, pos+16)[0]
            obj_name_idx  = struct.unpack_from("<i", plain, pos+20)[0]
            self.imports.append({
                "idx":   i,
                "pkg":   self.names[pkg_idx]      if 0 <= pkg_idx     < len(self.names) else "?",
                "class": self.names[class_idx]    if 0 <= class_idx   < len(self.names) else "?",
                "name":  self.names[obj_name_idx] if 0 <= obj_name_idx< len(self.names) else "?",
                "outer": outer_idx,
            })
            pos += 28

        # Exports
        self.exports = []
        pos = self.export_offset - self.name_offset
        for i in range(self.export_count):
            class_idx     = struct.unpack_from("<i", plain, pos)[0]
            super_idx     = struct.unpack_from("<i", plain, pos+4)[0]
            outer_idx     = struct.unpack_from("<i", plain, pos+8)[0]
            name_idx      = struct.unpack_from("<i", plain, pos+12)[0]
            serial_size   = struct.unpack_from("<I", plain, pos+32)[0]
            serial_offset = struct.unpack_from("<Q", plain, pos+36)[0]
            net_count     = struct.unpack_from("<i", plain, pos+48)[0]
            entry_size = 52 + net_count*4 + 16 + 4
            self.exports.append({
                "idx":   i,
                "class": class_idx,
                "outer": outer_idx,
                "name":  self.names[name_idx] if 0 <= name_idx < len(self.names) else "?",
                "so":    serial_offset,
                "ss":    serial_size,
            })
            pos += entry_size

        # Body decompress
        body = bytearray()
        pp = self.total_header
        self.chunks = []
        while pp + 16 <= len(buf):
            if struct.unpack_from("<I", buf, pp)[0] != UPK_MAGIC: break
            blk_size = struct.unpack_from("<I", buf, pp+4)[0]
            u_total  = struct.unpack_from("<I", buf, pp+12)[0]
            nb = (u_total + blk_size - 1) // blk_size
            metas_off = pp + 16
            cur = metas_off + nb*8
            for bi in range(nb):
                cs = struct.unpack_from("<I", buf, metas_off + bi*8)[0]
                us = struct.unpack_from("<I", buf, metas_off + bi*8 + 4)[0]
                self.chunks.append((cur, cs, len(body), us))
                body += zlib.decompress(buf[cur:cur+cs])
                cur += cs
            pp = cur
        self.body = body

        # Preamble vote on Texture2D exports
        t2d_idx = None
        for imp in self.imports:
            if imp["name"] == "Texture2D":
                t2d_idx = -(imp["idx"]+1); break
        votes = {}
        if t2d_idx is not None:
            for e in self.exports:
                if e["class"] != t2d_idx: continue
                base = e["so"] - self.total_header
                for k in range(0, 65536, 4):
                    upos = base + k
                    if upos < 0 or upos + 0x10c > len(body): continue
                    if struct.unpack_from("<i", body, upos)[0] != -1: continue
                    nm = struct.unpack_from("<i", body, upos + 0x108)[0]
                    if 1 <= nm <= 24:
                        votes[k] = votes.get(k, 0) + 1
        if votes:
            self.preamble = min(k for k,v in votes.items() if v == max(votes.values()))
        else:
            self.preamble = 0
        self.t2d_class_idx = t2d_idx

    def class_name(self, idx):
        if idx == 0: return "None"
        if idx > 0:
            i = idx-1
            if 0 <= i < len(self.exports):
                return self.exports[i]["name"]
            return f"?ex{idx}"
        i = -idx - 1
        if 0 <= i < len(self.imports):
            return self.imports[i]["name"]
        return f"?im{idx}"

    def resolve(self, idx):
        if idx == 0: return "None"
        if idx > 0:
            i = idx-1
            if 0 <= i < len(self.exports):
                e = self.exports[i]
                return f"export[{i}]:{e['name']}({self.class_name(e['class'])})"
            return f"export[?{idx}]"
        i = -idx - 1
        if 0 <= i < len(self.imports):
            imp = self.imports[i]
            return f"import[{i}]:{imp['name']}/{imp['class']} (pkg={imp['pkg']})"
        return f"import[?{idx}]"

    def find_export_by_name(self, nm, class_filter=None):
        for e in self.exports:
            if e["name"] == nm and (class_filter is None or self.class_name(e["class"]) == class_filter):
                return e
        return None

    def export_serial(self, e, with_preamble=True):
        body_pos = e["so"] - self.total_header + (self.preamble if with_preamble else 0)
        return body_pos, bytes(self.body[body_pos:body_pos+e["ss"]])


def walk_tagged_props(serial, names, max_iters=300):
    """Generic property walker. Returns list of dicts and end offset."""
    pp = 0
    props = []
    while pp + 24 <= len(serial) and len(props) < max_iters:
        n_idx = struct.unpack_from("<i", serial, pp)[0]
        if not (0 <= n_idx < len(names)):
            return props, pp, f"bad_n_idx={n_idx}@{pp:#x}"
        pname = names[n_idx]
        if pname == "None":
            return props, pp + 8, "ok"
        t_idx = struct.unpack_from("<i", serial, pp+8)[0]
        val_size = struct.unpack_from("<i", serial, pp+16)[0]
        arr_idx = struct.unpack_from("<i", serial, pp+20)[0]
        tname = names[t_idx] if 0 <= t_idx < len(names) else f"?{t_idx}"
        if val_size < 0 or val_size > (len(serial) - pp - 24):
            return props, pp, f"bad_val_size={val_size} @{pp:#x} pname={pname!r}"
        prop = {"name": pname, "type": tname, "off": pp, "val_size": val_size, "val_off": pp+24, "arr_idx": arr_idx}
        props.append(prop)
        if tname == "ByteProperty":
            new_pp = pp + 24 + 8 + val_size
        elif tname == "BoolProperty":
            new_pp = pp + 24 + 1
        elif tname == "StructProperty":
            new_pp = pp + 24 + 8 + val_size
            prop["struct_name_idx"] = struct.unpack_from("<i", serial, pp+24)[0]
        else:
            new_pp = pp + 24 + val_size
        if new_pp <= pp or new_pp > len(serial):
            return props, pp, f"bad_advance"
        pp = new_pp
    return props, pp, "limit"


def find_tag_offset(serial, name_idx, type_idx, val_size=None, min_tail=8):
    """Find a property tag with given name+type indices. Caller may require at least
    `min_tail` bytes available after the 24-byte tag (for the value payload)."""
    pat = struct.pack("<ii", name_idx, 0) + struct.pack("<ii", type_idx, 0)
    pos = 0
    while pos < len(serial):
        i = serial.find(pat, pos)
        if i < 0: return -1
        if i + 24 + min_tail > len(serial):
            return -1
        if val_size is not None:
            vs = struct.unpack_from("<i", serial, i+16)[0]
            if vs != val_size:
                pos = i + 1; continue
        return i
    return -1


def dump_material_subexpressions(upk: "Upk", mat_export_index, label):
    """Enumerate all sub-exports outer'd by mat_export_index+1, dump tex sample params."""
    print(f"\n{'#'*80}\n# {label}: {upk.path.name} export[{mat_export_index}] = {upk.exports[mat_export_index]['name']}\n{'#'*80}")
    outer_ref = mat_export_index + 1
    sub_exports = [(i,e) for i,e in enumerate(upk.exports) if e["outer"] == outer_ref]
    print(f"sub-exports: {len(sub_exports)}")
    target_classes = {
        "MaterialExpressionTextureSampleParameter2D",
        "MaterialExpressionTextureObjectParameter",
        "MaterialExpressionVectorParameter",
        "MaterialExpressionScalarParameter",
        "MaterialExpressionStaticSwitchParameter",
    }
    names = upk.names
    pn_pname  = names.index("ParameterName")       if "ParameterName"        in names else -1
    pn_group  = names.index("Group")               if "Group"                in names else -1
    pn_guid   = names.index("ExpressionGUID")      if "ExpressionGUID"       in names else -1
    pn_tex    = names.index("Texture")             if "Texture"              in names else -1
    pn_def    = names.index("DefaultTexture")      if "DefaultTexture"       in names else -1
    tn_name   = names.index("NameProperty")        if "NameProperty"         in names else -1
    tn_struct = names.index("StructProperty")      if "StructProperty"       in names else -1
    tn_object = names.index("ObjectProperty")      if "ObjectProperty"       in names else -1

    diffuse_guid_variants = make_guid_variants(DIFFUSE_GUID_STR)
    skin_guid_variants    = make_guid_variants(SKIN_GUID_STR)

    results = []
    matches = []
    for sub_ei, sub in sub_exports:
        cls = upk.class_name(sub["class"])
        if cls not in target_classes: continue
        _, serial = upk.export_serial(sub, with_preamble=False)
        # Material sub-expressions live inline (no preamble)
        rec = {"export": sub_ei, "name": sub["name"], "class": cls}
        if pn_pname >= 0 and tn_name >= 0:
            i = find_tag_offset(serial, pn_pname, tn_name, val_size=8, min_tail=8)
            if i >= 0:
                v_idx = struct.unpack_from("<i", serial, i+24)[0]
                rec["ParameterName"] = names[v_idx] if 0 <= v_idx < len(names) else None
        if pn_group >= 0 and tn_name >= 0:
            i = find_tag_offset(serial, pn_group, tn_name, val_size=8, min_tail=8)
            if i >= 0:
                v_idx = struct.unpack_from("<i", serial, i+24)[0]
                rec["Group"] = names[v_idx] if 0 <= v_idx < len(names) else None
        if pn_guid >= 0 and tn_struct >= 0:
            i = find_tag_offset(serial, pn_guid, tn_struct, val_size=16, min_tail=24)
            if i >= 0:
                guid_bytes = serial[i+32:i+48]
                rec["ExpressionGUID_hex"] = guid_bytes.hex().upper()
                for lab, gv in diffuse_guid_variants:
                    if guid_bytes == gv:
                        rec["MATCH_DIFFUSE"] = lab
                        matches.append(("DIFFUSE", sub_ei, sub["name"], cls, rec.get("ParameterName"), rec.get("Group"), lab))
                for lab, gv in skin_guid_variants:
                    if guid_bytes == gv:
                        rec["MATCH_SKIN"] = lab
        if (pn_tex >= 0 or pn_def >= 0) and tn_object >= 0:
            for nidx in (pn_tex, pn_def):
                if nidx < 0: continue
                i = find_tag_offset(serial, nidx, tn_object, val_size=4, min_tail=4)
                if i >= 0:
                    tex_idx = struct.unpack_from("<i", serial, i+24)[0]
                    rec["BoundTexture_idx"]  = tex_idx
                    rec["BoundTexture_name"] = upk.resolve(tex_idx)
                    break
        results.append(rec)
        line = f"  [{sub_ei}] {sub['name']} ({cls})"
        if "ParameterName" in rec: line += f"  Param={rec['ParameterName']!r}"
        if "Group" in rec:         line += f"  Group={rec['Group']!r}"
        if "ExpressionGUID_hex" in rec: line += f"  GUID={rec['ExpressionGUID_hex']}"
        if "MATCH_DIFFUSE" in rec:  line += f"  <<< DIFFUSE_GUID_MATCH ({rec['MATCH_DIFFUSE']})"
        if "MATCH_SKIN" in rec:     line += f"  <<< SKIN_GUID_MATCH ({rec['MATCH_SKIN']})"
        if "BoundTexture_name" in rec: line += f"  -> {rec['BoundTexture_name']}"
        print(line)
    return results, matches


def dump_mic(upk: "Upk", mic_export, label):
    print(f"\n{'='*80}\n= {label}: {upk.path.name} export[{mic_export['idx']}] = {mic_export['name']}\n{'='*80}")
    # MICs live inline (no Texture2D-style preamble). Body offset = so - total_header.
    body_pos = mic_export["so"] - upk.total_header
    serial = bytes(upk.body[body_pos:body_pos + mic_export["ss"]])
    print(f"  body_pos={body_pos:#x} ss={mic_export['ss']}")
    print(f"  first 32 bytes (raw): {serial[:32].hex()}")
    # MIC body: NetIndex(4 bytes) then tagged props
    props, end, status = walk_tagged_props(serial[4:], upk.names)
    print(f"  walked {len(props)} props (status={status})")
    out = {"parent": None, "texture_params": [], "vector_params": [], "scalar_params": [], "all_props": []}
    for pr in props:
        out["all_props"].append({"name": pr["name"], "type": pr["type"], "val_size": pr["val_size"]})
        if pr["name"] == "Parent" and pr["type"] == "ObjectProperty":
            v = struct.unpack_from("<i", serial[4:], pr["val_off"])[0]
            out["parent"] = {"idx": v, "resolved": upk.resolve(v)}
            print(f"    Parent -> {out['parent']['resolved']}")
        elif pr["type"] == "ArrayProperty" and pr["name"] in ("TextureParameterValues","VectorParameterValues","ScalarParameterValues"):
            ecount = struct.unpack_from("<i", serial[4:], pr["val_off"])[0]
            raw = serial[4:][pr["val_off"]+4 : pr["val_off"] + pr["val_size"]]
            print(f"    {pr['name']}: count={ecount}, raw_payload_size={len(raw)}")
            if ecount == 0: continue
            # entry stride heuristic: TextureParameterValues=116, Vector=??, Scalar=??
            stride = len(raw) // ecount if ecount else 0
            print(f"      stride (raw/ecount) = {stride}")
            for i in range(ecount):
                base = i * stride
                if base + 32 > len(raw): break
                pname_idx = struct.unpack_from("<i", raw, base + 24)[0]
                pname = upk.names[pname_idx] if 0 <= pname_idx < len(upk.names) else "?"
                # bytes after ParameterName: 4-byte NumberSuffix (val[0..4]), then likely a tag for ParameterValue
                entry = {"index": i, "ParameterName": pname}
                if pr["name"] == "TextureParameterValues":
                    obj_idx = struct.unpack_from("<i", raw, base + 56)[0]
                    entry["ParameterValue_idx"] = obj_idx
                    entry["ParameterValue_resolved"] = upk.resolve(obj_idx)
                    # GUID is at base + 24 (tag) + ParameterValue + ExpressionGUID tag
                    # entry layout: PN tag (24) + val(8) | PV tag (24) + val(4) | EG tag (24) + struct name (8) + guid (16) | None (8) = 116
                    guid_off = 24 + 8 + 24 + 4 + 24 + 8
                    if base + guid_off + 16 <= len(raw):
                        entry["ExpressionGUID_hex"] = raw[base + guid_off : base + guid_off + 16].hex().upper()
                    print(f"      [{i}] tex Param={pname!r} -> {entry['ParameterValue_resolved']}  GUID={entry.get('ExpressionGUID_hex','')}")
                    out["texture_params"].append(entry)
                elif pr["name"] == "VectorParameterValues":
                    # LinearColor (4 floats) right after PN. Layout: PN tag (24) + val (8) | "ParameterValue" StructProperty tag (24) + struct name FName (8) + 16 bytes
                    # Plus ExpressionGUID + None. For our purposes, just print pname.
                    print(f"      [{i}] vec Param={pname!r}")
                    out["vector_params"].append(entry)
                elif pr["name"] == "ScalarParameterValues":
                    print(f"      [{i}] scl Param={pname!r}")
                    out["scalar_params"].append(entry)
    return out


def scan_body_diffuse_textures(upk: "Upk", label):
    print(f"\n{'-'*80}\n- Texture2D candidates in {label}: {upk.path.name}\n{'-'*80}")
    if upk.t2d_class_idx is None:
        print("  (no Texture2D class import found in this UPK)")
        return []
    patterns = ["Body_D", "body_D", "BodyD", "_Body_", "Octane_Body", "Pepe_Body"]
    hits = []
    tfc_name_idx = upk.names.index("TextureFileCacheName") if "TextureFileCacheName" in upk.names else -1
    for e in upk.exports:
        if e["class"] != upk.t2d_class_idx: continue
        if not any(p.lower() in e["name"].lower() for p in patterns): continue
        body_pos = e["so"] - upk.total_header + upk.preamble
        if body_pos < 0 or body_pos + e["ss"] > len(upk.body): continue
        serial = bytes(upk.body[body_pos:body_pos + e["ss"]])
        tfc = None
        if tfc_name_idx >= 0:
            tag = struct.pack("<i", tfc_name_idx) + b"\0\0\0\0"
            idx = serial.find(tag)
            if idx >= 0 and idx + 32 <= len(serial):
                v_idx = struct.unpack_from("<i", serial, idx+24)[0]
                tfc = upk.names[v_idx] if 0 <= v_idx < len(upk.names) else None
        # NumMips at body_pos + 0x108
        num_mips = None; first_mip = None
        if body_pos + 0x10c <= len(upk.body):
            num_mips = struct.unpack_from("<i", upk.body, body_pos + 0x108)[0]
            if 1 <= num_mips <= 14 and body_pos + 0x10c + 28 <= len(upk.body):
                p = body_pos + 0x10c
                flags = struct.unpack_from("<I", upk.body, p)[0]
                elem  = struct.unpack_from("<I", upk.body, p+4)[0]
                sd    = struct.unpack_from("<I", upk.body, p+8)[0]
                oif   = struct.unpack_from("<Q", upk.body, p+12)[0]
                bc3_dim = int(round(elem ** 0.5))
                first_mip = {"flags": flags, "elem": elem, "size_disk": sd, "offset_in_file": oif, "approx_dim": bc3_dim}
        rec = {"export": e["idx"], "name": e["name"], "tfc": tfc, "num_mips": num_mips, "first_mip": first_mip}
        hits.append(rec)
        fm = ""
        if first_mip:
            fm = f"  mip0={first_mip['approx_dim']}px size={first_mip['size_disk']} off={first_mip['offset_in_file']:#x} flags={first_mip['flags']:#x}"
        print(f"  export[{e['idx']:4}] {e['name']:42}  tfc={tfc}  mips={num_mips}{fm}")
    return hits


def chain_resolve(start_upk, start_export_name, all_upks_by_pkg):
    """Walk MIC.Parent chain starting from <start_upk>::<start_export_name>.
    When a Parent points to an import, jump to that import's owning UPK if available.
    """
    print(f"\n{'+'*80}\n+ Parent chain trace from {start_upk.path.name}::{start_export_name}\n{'+'*80}")
    cur_upk = start_upk
    cur_obj_name = start_export_name
    cur_export_idx = None
    for ei, e in enumerate(cur_upk.exports):
        if e["name"] == cur_obj_name:
            cur_export_idx = ei; break
    if cur_export_idx is None:
        print(f"  {cur_obj_name} not found in {cur_upk.path.name}"); return
    visited = set()
    depth = 0
    while depth < 8:
        depth += 1
        key = (cur_upk.path.name, cur_export_idx)
        if key in visited:
            print("  cycle, stop"); return
        visited.add(key)
        e = cur_upk.exports[cur_export_idx]
        cls = cur_upk.class_name(e["class"])
        print(f"  [{depth}] {cur_upk.path.name}::export[{cur_export_idx}] {e['name']} ({cls})")
        if cls not in ("MaterialInstanceConstant", "Material"):
            print(f"      [stop] not a MIC/Material"); return
        body_pos, serial = cur_upk.export_serial(e, with_preamble=(cls == "MaterialInstanceConstant"))
        # walk props
        if cls == "MaterialInstanceConstant":
            inner = serial[4:]
        else:
            inner = serial  # Material has different layout but Parent only on MIC
        props, _, _ = walk_tagged_props(inner, cur_upk.names)
        parent_idx = None
        for pr in props:
            if pr["name"] == "Parent" and pr["type"] == "ObjectProperty":
                parent_idx = struct.unpack_from("<i", inner, pr["val_off"])[0]
                break
            if pr["name"] == "ParameterName" or pr["type"] == "ArrayProperty":
                # report any texture params along the way
                if pr["type"] == "ArrayProperty" and pr["name"] == "TextureParameterValues":
                    ecount = struct.unpack_from("<i", inner, pr["val_off"])[0]
                    raw = inner[pr["val_off"]+4 : pr["val_off"]+pr["val_size"]]
                    stride = len(raw)//ecount if ecount else 0
                    for i in range(ecount):
                        base = i*stride
                        if base + 60 > len(raw): break
                        pname_idx = struct.unpack_from("<i", raw, base+24)[0]
                        obj_idx   = struct.unpack_from("<i", raw, base+56)[0]
                        pname = cur_upk.names[pname_idx] if 0 <= pname_idx < len(cur_upk.names) else "?"
                        print(f"        TexParam {pname!r} -> {cur_upk.resolve(obj_idx)}")
        if parent_idx is None or parent_idx == 0:
            # also check Material: its sub-expressions are the actual texture sources
            if cls == "Material":
                print(f"      [stop] Material (root) — see sub-expression dump separately")
            else:
                print(f"      [stop] no Parent property")
            return
        print(f"      Parent ObjectIndex = {parent_idx} -> {cur_upk.resolve(parent_idx)}")
        if parent_idx > 0:
            cur_export_idx = parent_idx - 1
            continue
        # Negative -> import. Need to resolve which UPK has that object.
        imp = cur_upk.imports[-parent_idx - 1]
        target_obj = imp["name"]
        target_pkg = imp["pkg"]
        # walk import outer chain to find top-level package
        cur_idx = imp["outer"]
        top_pkg = target_pkg
        while cur_idx != 0:
            if cur_idx > 0: break
            outer_imp = cur_upk.imports[-cur_idx - 1]
            top_pkg = outer_imp["name"]
            cur_idx = outer_imp["outer"]
        print(f"      import top package = {top_pkg!r}, target obj = {target_obj!r}")
        # find that UPK
        next_upk = all_upks_by_pkg.get(top_pkg.lower())
        if next_upk is None:
            # Try fallback: maybe pkg points to top-level pkg, not file. Try matching by file name pattern.
            print(f"      [stop] no opened UPK matches package {top_pkg!r}")
            return
        nxt_ei = None
        for ei, e2 in enumerate(next_upk.exports):
            if e2["name"] == target_obj:
                nxt_ei = ei; break
        if nxt_ei is None:
            print(f"      [stop] {target_obj} not exported by {next_upk.path.name}")
            return
        cur_upk = next_upk
        cur_export_idx = nxt_ei
    print("  depth limit reached")


def main():
    startup     = Upk(RL / "Startup.upk")
    body_octane = Upk(RL / "Body_Octane_SF.upk")
    modded      = Upk(RL / "mods/Skin_Octane_Stars_SF.upk")

    print(f"\nStartup.upk     : {len(startup.names)} names, {len(startup.imports)} imports, {len(startup.exports)} exports, preamble={startup.preamble}")
    print(f"Body_Octane_SF  : {len(body_octane.names)} names, {len(body_octane.imports)} imports, {len(body_octane.exports)} exports, preamble={body_octane.preamble}")
    print(f"modded Stars    : {len(modded.names)} names, {len(modded.imports)} imports, {len(modded.exports)} exports, preamble={modded.preamble}")

    summary = {}

    # ----- 1. Body_All_Mat ------------
    bam = startup.find_export_by_name("Body_All_Mat", class_filter="Material")
    if bam:
        print(f"\nBody_All_Mat = export[{bam['idx']}] so={bam['so']:#x} ss={bam['ss']}")
        res, matches = dump_material_subexpressions(startup, bam["idx"], "Body_All_Mat sub-expressions")
        summary["Body_All_Mat"] = {"export_idx": bam["idx"], "sub_expressions": res, "matches": matches}
    else:
        print("Body_All_Mat NOT FOUND")
        summary["Body_All_Mat"] = None

    # ----- 1b. Body_Paintable_Mat (sanity reference) ----
    bpm = startup.find_export_by_name("Body_Paintable_Mat", class_filter="Material")
    if bpm:
        print(f"\nBody_Paintable_Mat = export[{bpm['idx']}] so={bpm['so']:#x} ss={bpm['ss']}")
        res, matches = dump_material_subexpressions(startup, bpm["idx"], "Body_Paintable_Mat sub-expressions")
        summary["Body_Paintable_Mat"] = {"export_idx": bpm["idx"], "sub_expressions": res, "matches": matches}

    # ----- 2. MIC_Body_Octane in Startup.upk -----
    print("\n--- All MaterialInstanceConstant exports in Startup.upk matching 'Body' ---")
    mic_class_idx = None
    for imp in startup.imports:
        if imp["name"] == "MaterialInstanceConstant":
            mic_class_idx = -(imp["idx"]+1); break
    body_mics = [e for e in startup.exports if e["class"] == mic_class_idx and "body" in e["name"].lower()]
    for m in body_mics:
        print(f"  export[{m['idx']}] {m['name']}")
    summary["startup_body_mics"] = []
    for m in body_mics:
        info = dump_mic(startup, m, f"Startup.upk {m['name']}")
        summary["startup_body_mics"].append({"name": m["name"], "export_idx": m["idx"], **info})

    # ----- 3. Body_Octane_SF.upk Materials + MICs -----
    print("\n--- ALL Material/MaterialInstanceConstant exports in Body_Octane_SF.upk ---")
    bo_mic_idx = None
    bo_mat_idx = None
    for imp in body_octane.imports:
        if imp["name"] == "MaterialInstanceConstant":
            bo_mic_idx = -(imp["idx"]+1)
        if imp["name"] == "Material":
            bo_mat_idx = -(imp["idx"]+1)
    bo_mics = [e for e in body_octane.exports if e["class"] == bo_mic_idx]
    bo_mats = [e for e in body_octane.exports if e["class"] == bo_mat_idx]
    print(f"  Materials ({len(bo_mats)}):")
    for m in bo_mats:
        print(f"    export[{m['idx']}] {m['name']}")
    print(f"  MICs ({len(bo_mics)}):")
    for m in bo_mics:
        print(f"    export[{m['idx']}] {m['name']}")
    summary["body_octane_materials"] = [{"name": m["name"], "idx": m["idx"]} for m in bo_mats]
    summary["body_octane_mics"] = []
    for m in bo_mics:
        info = dump_mic(body_octane, m, f"Body_Octane_SF.upk {m['name']}")
        summary["body_octane_mics"].append({"name": m["name"], "export_idx": m["idx"], **info})
    # Dump materials' sub-expressions too
    for m in bo_mats:
        res, matches = dump_material_subexpressions(body_octane, m["idx"], f"Body_Octane_SF.upk::{m['name']} sub-expressions")
        summary[f"body_octane_mat_{m['name']}"] = {"export_idx": m["idx"], "sub_expressions": res, "matches": matches}

    # ----- 4. Body diffuse texture candidates -----
    summary["startup_body_textures"]     = scan_body_diffuse_textures(startup, "Startup.upk")
    summary["body_octane_body_textures"] = scan_body_diffuse_textures(body_octane, "Body_Octane_SF.upk")

    # ----- 5. Full parent chain from modded MIC -----
    all_upks_by_pkg = {
        "startup":         startup,
        "body_octane_sf":  body_octane,
        "skin_octane_stars_sf": modded,
    }
    chain_resolve(modded, "Octane_Stars_MIC", all_upks_by_pkg)

    # Write JSON dump
    out_path = Path(__file__).with_name("body_diffuse_chain.json")
    with out_path.open("w", encoding="utf-8") as fp:
        json.dump(summary, fp, indent=2, default=str)
    print(f"\nJSON summary -> {out_path}")


if __name__ == "__main__":
    main()
