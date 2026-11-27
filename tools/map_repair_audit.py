"""Audit custom maps against the current game and draft map repair rules.

Usage (see docs_rl/map_repair.md):
  python tools/map_repair_audit.py <game_index.tsv> <imports_dir> <CookedPCConsole> [v|rules]

  game_index.tsv : `cargo run --release --example asset_probe -- <keys> index <cooked> <tsv>`
  imports_dir    : one `<map>.imports` per map (`asset_probe ... imports <map.upk>`)
  v              : list every unresolved reference
  rules          : print the "moved" references as mapping rules (JSON)

Classes of unresolved imports:
  moved : the same inner path (without the top package) exists in another file
  same  : an object with the same name and class exists elsewhere (manual review)
  gone  : nothing with that name and class (or a class the index skips)
"""
import os
import re
import sys
from collections import Counter, defaultdict

index_path, imports_dir, cooked = sys.argv[1], sys.argv[2], sys.argv[3]
mode = sys.argv[4] if len(sys.argv) > 4 else ""
verbose = mode == "v"
ALWAYS_TOP = {"startup.upk", "engine.upk", "tagame.upk", "core.upk"}
rules = []

by_path = defaultdict(list)   # inner path lower -> [(path, class, file)]
by_name = defaultdict(list)
for line in open(index_path, encoding="utf-8", errors="replace"):
    parts = line.rstrip("\n").split("\t")
    if len(parts) != 3:
        continue
    path, cls, f = parts
    by_path[path.lower()].append((path, cls, f))
    by_name[path.rsplit(".", 1)[-1].lower()].append((path, cls, f))

files = {f.lower() for f in os.listdir(cooked)}
ALWAYS = {"startup.upk", "engine.upk", "core.upk", "tagame.upk", "gfxui.upk"}
PACK = {"editorlandscaperesources", "editormaterials", "editormeshes", "editorresources", "enginedebugmaterials",
        "enginematerials", "engineresources", "maptemplateindex", "maptemplates", "nodebuddies",
        "engine_mi_shaders", "enginevolumetrics", "enginebuildings", "mods"}
NATIVE = {"core", "engine", "tagame", "gfxui", "unrealed", "ipdrv", "akaudio", "onlinesubsystemsteamworks", "windrv"}


def status(path, cls):
    top, _, rest = path.partition(".")
    t = top.lower()
    if t in NATIVE:
        return "ok", None
    # in memory under the full path (forced export in an always-loaded file)
    if any(f.lower() in ALWAYS for (_, c, f) in by_path.get(path.lower(), []) if c == cls):
        return "ok", None
    if t in PACK and (t + ".upk") in files:
        return "pack", None
    if (t + ".upk") in files and any(f.lower() == t + ".upk" and c == cls for (_, c, f) in by_path.get(rest.lower(), [])):
        return "ok", None
    moved = [e for e in by_path.get(rest.lower(), []) if e[1] == cls]
    if moved:
        moved.sort(key=lambda e: (e[2].lower() not in ALWAYS, os.path.getsize(os.path.join(cooked, e[2]))))
        return "moved", moved[0]
    same = [e for e in by_name.get(path.rsplit(".", 1)[-1].lower(), []) if e[1] == cls]
    if same:
        same.sort(key=lambda e: (e[2].lower() not in ALWAYS, os.path.getsize(os.path.join(cooked, e[2]))))
        return "same", same[0]
    return "gone", None


total = Counter()
for fn in sorted(os.listdir(imports_dir)):
    if not fn.endswith(".imports"):
        continue
    counts, details = Counter(), []
    for line in open(os.path.join(imports_dir, fn), encoding="utf-8", errors="replace"):
        m = re.match(r"(.+) \[(.+)\]$", line.strip())
        if not m:
            continue
        path, cls = m.groups()
        if cls in ("Package", "Class") or path.rsplit(".", 1)[-1].startswith("Default__"):
            continue
        st, cand = status(path, cls)
        counts[st] += 1
        if st in ("moved", "same", "gone"):
            details.append((st, path, cls, cand))
        if st == "moved":
            inner, f = cand[0], cand[2]
            to = inner if f.lower() in ALWAYS_TOP else f[:-4] + "." + inner
            if not any(r["from"].lower() == path.lower() for r in rules):
                rules.append({"from": path, "to": to, "class": cls})
    total.update(counts)
    print(f"== {fn[:-8][:50]}: " + ", ".join(f"{k} {v}" for k, v in sorted(counts.items())))
    if verbose:
        for st, path, cls, cand in details:
            where = f"{cand[0]} @ {cand[2]}" if cand else "-"
            print(f"     {st:5} {path} [{cls}] -> {where}")
print("TOTAL", dict(total))
if mode == "rules":
    import json
    print(json.dumps(sorted(rules, key=lambda r: r["from"].lower()), indent=1))
