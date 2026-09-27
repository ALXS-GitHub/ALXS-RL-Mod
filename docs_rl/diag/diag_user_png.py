"""Inspect the user's Yuna pack PNGs:
- alpha channel statistics
- a few sample RGBA values at known UV locations matching the mask's
  literal-diffuse zones
to figure out why those zones render white in-game."""

from pathlib import Path
from PIL import Image
import collections

YUNA_DIR = Path(r"%APPDATA%\bakkesmod\bakkesmod\data\acplugin\DecalTextures\Born To Be Itzy\Octane")
DIFFUSE = YUNA_DIR / "oct_diffuse.png"
MASK = YUNA_DIR / "oct_decal.png"

def inspect(name, path):
    img = Image.open(path)
    print(f"\n=== {name}: {path.name} ===")
    print(f"  size = {img.size}, mode = {img.mode}")
    if img.mode != "RGBA":
        img = img.convert("RGBA")
    px = list(img.getdata())
    n = len(px)
    # Alpha histogram (bucketed)
    alpha_hist = collections.Counter()
    for _, _, _, a in px:
        alpha_hist[a // 32 * 32] += 1
    print(f"  alpha histogram (bucketed by /32):")
    for bucket in sorted(alpha_hist.keys()):
        pct = 100 * alpha_hist[bucket] / n
        print(f"    A={bucket:3}–{bucket+31}: {alpha_hist[bucket]:>8}  ({pct:.1f}%)")
    # Distinct values
    n_distinct = len(set(px))
    print(f"  distinct RGBA tuples: {n_distinct}")
    # RGB samples at corners + center
    w, h = img.size
    samples = [
        ("top-left", 0, 0),
        ("top-mid", w // 2, 0),
        ("center", w // 2, h // 2),
        ("Yuna face area (estimated)", int(w * 0.5), int(h * 0.4)),
        ("bottom-center 'ITZY' area", w // 2, int(h * 0.85)),
        ("left side 'YUNA' area", int(w * 0.2), int(h * 0.5)),
    ]
    for label, x, y in samples:
        r, g, b, a = img.getpixel((x, y))
        print(f"  ({x:5},{y:5}) {label}: R={r:3} G={g:3} B={b:3} A={a:3}")

inspect("DIFFUSE", DIFFUSE)
inspect("MASK", MASK)

# Full channel histograms for MASK
print("\n=== MASK channel histograms ===")
img = Image.open(MASK).convert("RGBA")
px = list(img.getdata())
n = len(px)
r_hist = collections.Counter()
g_hist = collections.Counter()
b_hist = collections.Counter()
for r, g, b, a in px:
    r_hist[r // 16 * 16] += 1
    g_hist[g // 16 * 16] += 1
    b_hist[b // 16 * 16] += 1
for label, hist in [("R", r_hist), ("G", g_hist), ("B", b_hist)]:
    print(f"\n  {label} channel buckets (>=0.1%):")
    for bucket in sorted(hist.keys()):
        pct = 100 * hist[bucket] / n
        if pct >= 0.1:
            print(f"    {bucket:3}-{bucket+15:3}: {hist[bucket]:>9}  ({pct:.2f}%)")

# Count "zone" pixels per AC convention
counts = {
    "primary R=255 A=0":   0,  # not relevant in 3-channel RGB
    "primary R>=200":      0,
    "accent G>=200":       0,
    "literal_diff R~43":   0,  # R in [35, 55]
    "near_zero R<=5":      0,  # R near 0 = true literal diffuse per shader
    "windows B>=200":      0,
    "other":               0,
}
for r, g, b, a in px:
    if r >= 200 and g < 50 and b < 50: counts["primary R>=200"] += 1
    elif g >= 200 and r < 50 and b < 50: counts["accent G>=200"] += 1
    elif 35 <= r <= 55 and g < 30 and b < 30: counts["literal_diff R~43"] += 1
    elif r <= 5 and g <= 5 and b <= 5: counts["near_zero R<=5"] += 1
    elif b >= 200 and r < 50 and g < 50: counts["windows B>=200"] += 1
    else: counts["other"] += 1
print("\n  Zone classification per AC convention:")
for label, c in counts.items():
    pct = 100 * c / n
    print(f"    {label:25}: {c:>9}  ({pct:.2f}%)")
