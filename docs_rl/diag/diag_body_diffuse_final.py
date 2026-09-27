"""Final body-diffuse investigation.

Produces a single comprehensive trace answering:
  Which Texture2D does the shader sample for the body's diffuse layer
  when a Force-style decal (e.g. Octane_Stars_MIC) is equipped on Octane?

Key insight from preliminaries:
  * Body_Paintable_Mat's `Diffuse` TextureSample2D (GUID D307E26F...) defaults to
    `Chasis_Pepe_D` — a chassis texture. Body_Paintable_Mat is the *parts*
    material, NOT the body material.
  * Body_All_Mat has NO TextureSampleParameter2D named 'Diffuse'. It has only a
    VectorParameter named 'Diffuse'. So body diffuse is sampled differently.
  * MIC_Body_Paintable_All (Startup.upk export[832]) is the parent of all
    body MICs and lives in package `Vehicle_Parent_Materials`.
  * Donor `Octane_Stars_MIC.Diffuse` GUID = 71B78637-... does NOT match any
    sub-expression in Body_All_Mat or Body_Paintable_Mat.

This script:
  1. Re-dumps Body_All_Mat & Body_Paintable_Mat sub-expressions with proper
     preamble=0 (Startup.upk convention).
  2. Parses Startup.upk MIC bodies by SCANNING for known tagged-prop FName tags
     (Parent, TexParamValues) directly inside the export's serial range, then
     validating that the parent's tag is preceded by a sensible property header
     (val_size=4, type=ObjectProperty).
  3. Walks the full parent chain from Octane_Stars_MIC through Startup.upk MICs.
  4. Lists every TextureSampleParameter2D in the entire Startup.upk that defaults
     to or has a name suggesting a body diffuse (Octane_Body_D, Pepe_Body_D, ...).
  5. Resolves the Diffuse GUID 71B78637-... by searching ALL of Startup.upk's
     sub-expressions across all Material exports.
"""

import base64, json, struct, zlib
from pathlib import Path
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes

RL = Path(r"C:/Program Files/Epic Games/rocketleague/TAGame/CookedPCConsole")
KEYS = Path(__file__).parent.parent.parent / "src-tauri/resources/decryptor/keys.txt"
UPK_MAGIC = 0x9E2A83C1

DIFFUSE_GUID_STR = "71B78637-5FAD-894A-889C-A31EEC008A34"
SKIN_GUID_STR    = "F6B52F25-42AB-BD49-AA45-5F91F981F5C9"


def aes_decrypt(data, key):
    return Cipher(algorithms.AES(key), modes.ECB()).decryptor().update(data) + b""


KEYS_LIST = [base64.b64decode(line.strip()) for line in KEYS.read_text().splitlines() if line.strip()]


def guid_variants(s):
    h = s.replace("-", "")
    raw = bytes.fromhex(h)
    return [
        ("MS",        bytes(reversed(raw[:4])) + bytes(reversed(raw[4:6])) + bytes(reversed(raw[6:8])) + raw[8:]),
        ("UE3LE",     b"".join(bytes(reversed(raw[i:i+4])) for i in range(0,16,4))),
        ("RAW",       raw),
    ]


class Upk:
    def __init__(self, path, preamble_override=None):
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
        if plain is None: raise RuntimeError("decrypt failed: "+str(path))
        self.plain = plain

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

        self.imports = []
        pos = self.import_offset - self.name_offset
        for i in range(self.import_count):
            pkg_idx      = struct.unpack_from("<i", plain, pos)[0]
            class_idx    = struct.unpack_from("<i", plain, pos+8)[0]
            outer_idx    = struct.unpack_from("<i", plain, pos+16)[0]
            obj_name_idx = struct.unpack_from("<i", plain, pos+20)[0]
            self.imports.append({
                "idx": i,
                "pkg":   self.names[pkg_idx] if 0 <= pkg_idx < len(self.names) else "?",
                "class": self.names[class_idx] if 0 <= class_idx < len(self.names) else "?",
                "name":  self.names[obj_name_idx] if 0 <= obj_name_idx < len(self.names) else "?",
                "outer": outer_idx,
            })
            pos += 28

        self.exports = []
        pos = self.export_offset - self.name_offset
        for i in range(self.export_count):
            class_idx     = struct.unpack_from("<i", plain, pos)[0]
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

        if preamble_override is not None:
            self.preamble = preamble_override
        else:
            # Compute Texture2D preamble vote, but USE 0 for Startup.upk
            t2d_idx = None
            for imp in self.imports:
                if imp["name"] == "Texture2D":
                    t2d_idx = -(imp["idx"]+1); break
            self.t2d_class_idx = t2d_idx
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
            self.preamble = min(k for k,v in votes.items() if v == max(votes.values())) if votes else 0

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
            top = self.import_top_pkg(idx)
            return f"import[{i}]:{imp['name']}/{imp['class']} (top_pkg={top!r})"
        return f"import[?{idx}]"

    def import_top_pkg(self, idx):
        if idx >= 0: return None
        cur = idx
        top = None
        for _ in range(8):
            if cur >= 0: break
            imp = self.imports[-cur - 1]
            top = imp["name"]
            cur = imp["outer"]
            if cur == 0: break
        return top

    def find_export_by_name(self, nm, class_filter=None):
        for e in self.exports:
            if e["name"] == nm and (class_filter is None or self.class_name(e["class"]) == class_filter):
                return e
        return None


def walk_tagged_props(serial, names, max_iters=300):
    pp = 0
    props = []
    while pp + 24 <= len(serial) and len(props) < max_iters:
        n_idx = struct.unpack_from("<i", serial, pp)[0]
        if not (0 <= n_idx < len(names)):
            return props, pp, f"bad_n_idx={n_idx}"
        pname = names[n_idx]
        if pname == "None":
            return props, pp + 8, "ok"
        t_idx = struct.unpack_from("<i", serial, pp+8)[0]
        val_size = struct.unpack_from("<i", serial, pp+16)[0]
        tname = names[t_idx] if 0 <= t_idx < len(names) else f"?{t_idx}"
        if val_size < 0 or val_size > (len(serial) - pp - 24):
            return props, pp, f"bad_val_size={val_size}"
        prop = {"name": pname, "type": tname, "off": pp, "val_size": val_size, "val_off": pp+24}
        props.append(prop)
        if tname == "ByteProperty":
            new_pp = pp + 24 + 8 + val_size
        elif tname == "BoolProperty":
            new_pp = pp + 24 + 1
        elif tname == "StructProperty":
            new_pp = pp + 24 + 8 + val_size
        else:
            new_pp = pp + 24 + val_size
        if new_pp <= pp or new_pp > len(serial):
            return props, pp, "bad_advance"
        pp = new_pp
    return props, pp, "limit"


def find_mic_tagged_block(upk, e):
    """In a MIC body, the tagged-prop block lives AFTER a binary preamble of
    NetIndex + ScriptSize + StaticPermutationResource + ParentRefFence + ...
    We find its start by scanning for a (FName, FName) header pattern where the
    type-name is one of {StructProperty, ObjectProperty, ArrayProperty, NameProperty,
    FloatProperty, IntProperty, BoolProperty, ByteProperty} AND where walk_tagged_props
    reaches a 'None' terminator.
    """
    body_pos = e["so"] - upk.total_header
    end = body_pos + e["ss"]
    type_names = {"StructProperty","ObjectProperty","ArrayProperty","NameProperty",
                  "FloatProperty","IntProperty","BoolProperty","ByteProperty","StrProperty"}
    type_idx_set = set()
    for tn in type_names:
        if tn in upk.names:
            type_idx_set.add(upk.names.index(tn))
    best = None
    for shift in range(0, min(8192, e["ss"] - 24), 4):
        bp = body_pos + shift
        n_idx = struct.unpack_from("<i", upk.body, bp)[0]
        n_num = struct.unpack_from("<i", upk.body, bp+4)[0]
        if not (0 <= n_idx < len(upk.names)) or n_num != 0: continue
        first = upk.names[n_idx]
        if not first or first == "None": continue
        t_idx = struct.unpack_from("<i", upk.body, bp+8)[0]
        if t_idx not in type_idx_set: continue
        val_size = struct.unpack_from("<i", upk.body, bp+16)[0]
        if val_size < 0 or val_size > (end - bp - 24): continue
        serial_slice = bytes(upk.body[bp:end])
        props, end_off, status = walk_tagged_props(serial_slice, upk.names)
        if status == "ok" and len(props) >= 1:
            score = len(props) * 1000 - shift  # prefer most props, earliest shift
            if best is None or score > best[0]:
                best = (score, shift, bp, props, serial_slice, status)
    return best


def parse_mic(upk, e, indent="  "):
    """Return dict with parent_idx, parent_resolved, texture_params, vector_params, scalar_params."""
    result = {"name": e["name"], "export_idx": e["idx"], "parent": None,
              "texture_params": [], "vector_params": [], "scalar_params": [],
              "tagged_block_shift": None}
    best = find_mic_tagged_block(upk, e)
    if not best:
        print(f"{indent}<could not locate tagged-prop block>")
        return result
    score, shift, bp, props, serial, status = best
    result["tagged_block_shift"] = shift
    print(f"{indent}tagged block starts at +{shift} ({bp:#x}), {len(props)} props ({status})")
    for pr in props:
        if pr["name"] == "Parent" and pr["type"] == "ObjectProperty":
            v = struct.unpack_from("<i", serial, pr["val_off"])[0]
            result["parent"] = {"idx": v, "resolved": upk.resolve(v)}
            print(f"{indent}Parent -> {result['parent']['resolved']}")
        if pr["name"] in ("TextureParameterValues","VectorParameterValues","ScalarParameterValues") and pr["type"] == "ArrayProperty":
            ecount = struct.unpack_from("<i", serial, pr["val_off"])[0]
            raw = serial[pr["val_off"]+4 : pr["val_off"]+pr["val_size"]]
            if ecount == 0:
                print(f"{indent}{pr['name']}: 0 entries"); continue
            stride = len(raw) // ecount
            print(f"{indent}{pr['name']}: {ecount} entries (stride={stride})")
            for i in range(ecount):
                base = i*stride
                if base+32 > len(raw): break
                pname_idx = struct.unpack_from("<i", raw, base+24)[0]
                pname = upk.names[pname_idx] if 0 <= pname_idx < len(upk.names) else "?"
                entry = {"index": i, "ParameterName": pname}
                if pr["name"] == "TextureParameterValues" and base + 60 <= len(raw):
                    obj_idx = struct.unpack_from("<i", raw, base+56)[0]
                    entry["ParameterValue_idx"] = obj_idx
                    entry["ParameterValue_resolved"] = upk.resolve(obj_idx)
                    # GUID at +24+8+24+4+24+8 = 92
                    guid_off = base + 92
                    if guid_off + 16 <= len(raw):
                        entry["ExpressionGUID_hex"] = raw[guid_off:guid_off+16].hex().upper()
                    print(f"{indent}  [{i}] {pname!r} -> {entry['ParameterValue_resolved']}  GUID={entry.get('ExpressionGUID_hex','')}")
                    result["texture_params"].append(entry)
                elif pr["name"] == "VectorParameterValues":
                    # LinearColor (4 floats = 16 bytes) inside StructProperty
                    print(f"{indent}  [{i}] {pname!r} (vec)")
                    result["vector_params"].append(entry)
                else:
                    print(f"{indent}  [{i}] {pname!r} (scl)")
                    result["scalar_params"].append(entry)
    return result


def dump_material_sub(upk, mat_idx, target_diffuse_guids):
    """Walk all sub-exports of a material, find textures and detect GUID matches."""
    outer = mat_idx + 1
    subs = [(i,e) for i,e in enumerate(upk.exports) if e["outer"] == outer]
    print(f"  ({len(subs)} sub-exports)")
    matches = []
    names = upk.names
    for sub_ei, sub in subs:
        cls = upk.class_name(sub["class"])
        if cls not in ("MaterialExpressionTextureSampleParameter2D","MaterialExpressionTextureObjectParameter","MaterialExpressionVectorParameter"):
            continue
        body_pos = sub["so"] - upk.total_header
        serial = bytes(upk.body[body_pos:body_pos+sub["ss"]])
        # find ParameterName + GUID + DefaultTexture
        pname = None; guid = None; tex_idx = None
        if "ParameterName" in names and "NameProperty" in names:
            n = names.index("ParameterName"); t = names.index("NameProperty")
            pat = struct.pack("<iiii", n, 0, t, 0)
            i = serial.find(pat)
            if i >= 0 and i + 32 <= len(serial):
                v_idx = struct.unpack_from("<i", serial, i+24)[0]
                if 0 <= v_idx < len(names): pname = names[v_idx]
        if "ExpressionGUID" in names and "StructProperty" in names:
            n = names.index("ExpressionGUID"); t = names.index("StructProperty")
            pat = struct.pack("<iiii", n, 0, t, 0)
            i = serial.find(pat)
            if i >= 0 and i + 48 <= len(serial):
                guid = serial[i+32:i+48]
        for tex_name in ("Texture","DefaultTexture"):
            if tex_name in names and "ObjectProperty" in names:
                n = names.index(tex_name); t = names.index("ObjectProperty")
                pat = struct.pack("<iiii", n, 0, t, 0)
                i = serial.find(pat)
                if i >= 0 and i + 28 <= len(serial):
                    val_size = struct.unpack_from("<i", serial, i+16)[0]
                    if val_size == 4:
                        tex_idx = struct.unpack_from("<i", serial, i+24)[0]
                        break
        guid_hex = guid.hex().upper() if guid else ""
        guid_match = ""
        for lab, gv in target_diffuse_guids:
            if guid == gv:
                guid_match = f" <<DIFFUSE_GUID_MATCH({lab})>>"
                matches.append((sub_ei, sub["name"], cls, pname, lab))
        line = f"    [{sub_ei}] {sub['name']} ({cls})"
        if pname: line += f"  Param={pname!r}"
        if guid_hex: line += f"  GUID={guid_hex}"
        if tex_idx is not None and tex_idx != 0:
            line += f"  -> {upk.resolve(tex_idx)}"
        line += guid_match
        print(line)
    return matches


def scan_all_materials_for_diffuse_guid(upk, target_guids):
    """Scan EVERY Material export's sub-expressions for the target Diffuse GUID."""
    print(f"\n{'#'*80}\n# Scanning ALL Materials in {upk.path.name} for Diffuse GUID {DIFFUSE_GUID_STR}\n{'#'*80}")
    mat_class_idx = None
    for imp in upk.imports:
        if imp["name"] == "Material":
            mat_class_idx = -(imp["idx"]+1); break
    mats = [e for e in upk.exports if e["class"] == mat_class_idx]
    print(f"  {len(mats)} Material exports in this UPK")
    all_matches = []
    for m in mats:
        outer = m["idx"] + 1
        for sub_ei, sub in enumerate(upk.exports):
            if sub["outer"] != outer: continue
            cls = upk.class_name(sub["class"])
            if cls not in ("MaterialExpressionTextureSampleParameter2D","MaterialExpressionTextureObjectParameter","MaterialExpressionVectorParameter"):
                continue
            body_pos = sub["so"] - upk.total_header
            serial = bytes(upk.body[body_pos:body_pos+sub["ss"]])
            if "ExpressionGUID" in upk.names and "StructProperty" in upk.names:
                n = upk.names.index("ExpressionGUID"); t = upk.names.index("StructProperty")
                pat = struct.pack("<iiii", n, 0, t, 0)
                i = serial.find(pat)
                if i >= 0 and i + 48 <= len(serial):
                    guid_bytes = serial[i+32:i+48]
                    for lab, gv in target_guids:
                        if guid_bytes == gv:
                            # also extract ParameterName + bound tex
                            pname = None; tex = None
                            if "ParameterName" in upk.names:
                                pat2 = struct.pack("<iiii", upk.names.index("ParameterName"), 0, upk.names.index("NameProperty"), 0)
                                j = serial.find(pat2)
                                if j >= 0 and j + 32 <= len(serial):
                                    v = struct.unpack_from("<i", serial, j+24)[0]
                                    pname = upk.names[v] if 0 <= v < len(upk.names) else None
                            for tn in ("Texture","DefaultTexture"):
                                if tn in upk.names:
                                    pat3 = struct.pack("<iiii", upk.names.index(tn), 0, upk.names.index("ObjectProperty"), 0)
                                    j = serial.find(pat3)
                                    if j >= 0 and j + 28 <= len(serial):
                                        vs = struct.unpack_from("<i", serial, j+16)[0]
                                        if vs == 4:
                                            tex = struct.unpack_from("<i", serial, j+24)[0]; break
                            print(f"  HIT: Material={m['name']} sub_ei={sub_ei} {sub['name']} ({cls}) Param={pname!r} BoundTex={upk.resolve(tex) if tex else 'None'} variant={lab}")
                            all_matches.append({"material": m["name"], "sub_ei": sub_ei, "cls": cls, "param": pname, "tex": tex, "variant": lab})
    if not all_matches:
        print("  NO MATCH ANYWHERE in this UPK")
    return all_matches


def main():
    print("Loading UPKs...")
    startup     = Upk(RL / "Startup.upk", preamble_override=0)
    body_octane = Upk(RL / "Body_Octane_SF.upk")
    modded      = Upk(RL / "mods/Skin_Octane_Stars_SF.upk")
    print(f"  Startup.upk:     names={len(startup.names)} imports={len(startup.imports)} exports={len(startup.exports)} preamble={startup.preamble}")
    print(f"  Body_Octane_SF:  names={len(body_octane.names)} imports={len(body_octane.imports)} exports={len(body_octane.exports)} preamble={body_octane.preamble}")
    print(f"  modded Stars:    names={len(modded.names)} imports={len(modded.imports)} exports={len(modded.exports)} preamble={modded.preamble}")

    diffuse_variants = guid_variants(DIFFUSE_GUID_STR)
    print(f"\nDiffuse GUID variants:")
    for lab, gv in diffuse_variants:
        print(f"  {lab:6} {gv.hex().upper()}")

    summary = {}

    # ----- 1. Body_All_Mat sub-expressions -----
    print(f"\n{'#'*80}\n# Body_All_Mat sub-expressions\n{'#'*80}")
    bam = startup.find_export_by_name("Body_All_Mat", class_filter="Material")
    if bam:
        m = dump_material_sub(startup, bam["idx"], diffuse_variants)
        summary["Body_All_Mat_matches"] = m
    print(f"\n{'#'*80}\n# Body_Paintable_Mat sub-expressions\n{'#'*80}")
    bpm = startup.find_export_by_name("Body_Paintable_Mat", class_filter="Material")
    if bpm:
        m = dump_material_sub(startup, bpm["idx"], diffuse_variants)
        summary["Body_Paintable_Mat_matches"] = m

    # ----- 2. Startup MICs (Body_Octane, Paintable_All etc.) -----
    print(f"\n{'='*80}\n= Startup MICs (body chain)\n{'='*80}")
    targets = []
    for nm in ("MIC_Body_Octane","OctaneChassis_MIC","Body_All_MIC","MIC_Body_Paintable_All"):
        e = startup.find_export_by_name(nm, class_filter="MaterialInstanceConstant")
        if e: targets.append(e)
    summary["startup_mics"] = []
    for e in targets:
        print(f"\n--- {e['name']} (export[{e['idx']}]) so={e['so']:#x} ss={e['ss']} ---")
        info = parse_mic(startup, e, indent="  ")
        summary["startup_mics"].append(info)

    # ----- 3. Search ALL Materials for Diffuse GUID -----
    summary["diffuse_guid_hits_startup"] = scan_all_materials_for_diffuse_guid(startup, diffuse_variants)
    summary["diffuse_guid_hits_body_octane"] = scan_all_materials_for_diffuse_guid(body_octane, diffuse_variants)

    # ----- 4. List body-diffuse Texture2Ds and their TFC bindings -----
    print(f"\n{'-'*80}\n- Body-diffuse Texture2D candidates in Startup.upk\n{'-'*80}")
    t2d_idx = None
    for imp in startup.imports:
        if imp["name"] == "Texture2D":
            t2d_idx = -(imp["idx"]+1); break
    tfc_name_idx = startup.names.index("TextureFileCacheName")
    patterns = ["body_d", "body_blank", "octane_body", "pepe_body", "chasis_pepe"]
    summary["body_diffuse_textures"] = []
    for e in startup.exports:
        if e["class"] != t2d_idx: continue
        if not any(p in e["name"].lower() for p in patterns): continue
        body_pos = e["so"] - startup.total_header
        if body_pos < 0 or body_pos + e["ss"] > len(startup.body): continue
        serial = bytes(startup.body[body_pos:body_pos + e["ss"]])
        tfc = None
        tag = struct.pack("<i", tfc_name_idx) + b"\0\0\0\0"
        i = serial.find(tag)
        if i >= 0 and i+32 <= len(serial):
            v_idx = struct.unpack_from("<i", serial, i+24)[0]
            tfc = startup.names[v_idx] if 0 <= v_idx < len(startup.names) else None
        # NumMips at +0x108
        nm = None; mip0 = None
        if body_pos + 0x10c <= len(startup.body):
            nm = struct.unpack_from("<i", startup.body, body_pos+0x108)[0]
            if 1 <= nm <= 14 and body_pos+0x10c+28 <= len(startup.body):
                p = body_pos+0x10c
                flags = struct.unpack_from("<I", startup.body, p)[0]
                elem  = struct.unpack_from("<I", startup.body, p+4)[0]
                sd    = struct.unpack_from("<I", startup.body, p+8)[0]
                oif   = struct.unpack_from("<Q", startup.body, p+12)[0]
                dim   = int(round(elem ** 0.5))
                mip0 = {"flags": flags, "elem": elem, "size_disk": sd, "offset_in_file": oif, "approx_dim": dim}
        rec = {"export": e["idx"], "name": e["name"], "tfc": tfc, "num_mips": nm, "first_mip": mip0}
        summary["body_diffuse_textures"].append(rec)
        s = f"  export[{e['idx']:4}] {e['name']:38} tfc={tfc} mips={nm}"
        if mip0: s += f" mip0={mip0['approx_dim']}px @{mip0['offset_in_file']:#x} size={mip0['size_disk']} flags={mip0['flags']:#x}"
        print(s)

    # ----- 5. Parent chain trace -----
    print(f"\n{'+'*80}\n+ Full parent chain from modded Octane_Stars_MIC\n{'+'*80}")
    upk_chain = [("Skin_Octane_Stars_SF", modded), ("Startup", startup)]
    cur_upk = modded
    cur_name = "Octane_Stars_MIC"
    cur_eidx = None
    for ei, e in enumerate(cur_upk.exports):
        if e["name"] == cur_name:
            cur_eidx = ei; break
    chain_records = []
    for depth in range(8):
        if cur_eidx is None:
            print(f"  [stop] could not resolve current export"); break
        e = cur_upk.exports[cur_eidx]
        cls = cur_upk.class_name(e["class"])
        print(f"\n  [{depth+1}] {cur_upk.path.name}::export[{cur_eidx}] {e['name']} ({cls})")
        info = parse_mic(cur_upk, e, indent="      ")
        chain_records.append({"depth": depth+1, "upk": cur_upk.path.name, "export": cur_eidx, "name": e["name"], "class": cls, "info": info})
        if cls != "MaterialInstanceConstant":
            print(f"      [stop] not a MIC"); break
        parent = info.get("parent")
        if not parent:
            print(f"      [stop] no Parent property"); break
        pidx = parent["idx"]
        if pidx == 0:
            print(f"      [stop] Parent is None"); break
        if pidx > 0:
            cur_eidx = pidx - 1; continue
        # negative: import — try to resolve in Startup.upk by name
        imp = cur_upk.imports[-pidx - 1]
        target_name = imp["name"]
        top_pkg = cur_upk.import_top_pkg(pidx)
        print(f"      import target={target_name!r} top_pkg={top_pkg!r} -> trying to resolve in Startup.upk")
        nxt_eidx = None
        for ei2, e2 in enumerate(startup.exports):
            if e2["name"] == target_name:
                # match outer chain to top_pkg if possible
                outer_chain = []
                cur = startup.exports[ei2]["outer"]
                while cur > 0 and len(outer_chain) < 10:
                    outer_chain.append(startup.exports[cur-1]["name"])
                    cur = startup.exports[cur-1]["outer"]
                if top_pkg is None or top_pkg in outer_chain:
                    nxt_eidx = ei2; break
        if nxt_eidx is None:
            print(f"      [stop] {target_name!r} not found in Startup.upk")
            break
        cur_upk = startup
        cur_eidx = nxt_eidx
    summary["chain_trace"] = chain_records

    # Write JSON
    out_path = Path(__file__).with_name("body_diffuse_final.json")
    with out_path.open("w", encoding="utf-8") as fp:
        json.dump(summary, fp, indent=2, default=str)
    print(f"\nJSON summary -> {out_path}")


if __name__ == "__main__":
    main()
