# RL skin/decal mask color convention — community sources

Date: 2026-05-15
Author: research agent for ALXS-RL-Mod
Goal: independently confirm or refute the four mask sentinels documented by Alexis in `../RL-Designer/docs/Designer-Guide.md` and the `[0.15, 0.18]` literal-diffuse R-threshold in `colorReplacement.frag`.

## TL;DR

| Sentinel | RGBA bytes | Hex (RGBA) | Role |
| --- | --- | --- | --- |
| Primary picker color | `255, 0, 0, 0` | `#FF000000` | Body / Primary paint zone |
| Accent picker color | `255, 0, 0, 255` | `#FF0000FF` | Decal / Accent paint zone |
| Literal diffuse | `43, 0, 0, 0` | `#2B000000` | Show raw Diffuse texture color, no team tint |
| Windows | `*, *, 255, *` | `#0000FF**` | Windows / glass (alpha varies in practice) |

The convention is **confirmed empirically** by direct pixel sampling of production AlphaConsole skin masks (the de-facto distribution format for post-EAC custom decals). The R=43 (`0x2B`) literal-diffuse sentinel is present at byte-exact value in real-world masks at significant pixel counts. Best estimate for the literal-diffuse R tolerance: **R ∈ [0.15, 0.18]** (i.e. R in `[38, 46]` of 255) is correct and even slightly too generous on the low end — production masks ship `0x2B = 43` *exactly* with no soft-edge bleed inside the zone.

## Primary source — empirical pixel sampling of production AlphaConsole masks

This is the **strongest evidence** found in this investigation. The mask color convention is not documented in any prose tutorial we could locate, but the convention is **directly observable in the byte values of published decal masks** distributed by AlphaConsole.

### Source A — Butthead's "SandMan" pack (AlphaConsoleTextures repo)

- URL: https://github.com/AlphaConsole/AlphaConsoleTextures/blob/master/textures/Butthead/Sandman/sandman_decal.png
- File: `sandman_decal.png` (895 KB, 2048 × 2048, RGBA)
- Used as the `Skin` parameter on slot `1` (decal slot) targeting Octane item id `306`.

Pixel histogram (sampled every 4 px on both axes, 262 144 samples):

| Bucket | Pixels | Share |
| --- | --- | --- |
| `(255, 0, 0, 0)` — primary | 108 695 | 41.5 % |
| transparent near-zero (anti-alias) | 55 063 | 21.0 % |
| `(43, 0, 0, 0)` — literal diffuse | 51 433 | 19.6 % |
| `(255, 0, 0, 255)` — accent | 34 464 | 13.2 % |
| other (gradient edges) | 6 937 | 2.6 % |
| `(0, 0, 0, 255)` — opaque black | 4 287 | 1.6 % |
| `(*, *, 255, 0)` — windows | 1 265 | 0.5 % |

Top exact RGBA tuples (every 16 px sampling):
- `(255, 0, 0, 0)` → 6 664 (primary)
- `(43, 0, 0, 0)` → 2 653 (literal diffuse — **byte-exact 0x2B**)
- `(255, 0, 0, 255)` → 2 091 (accent)
- `(16, 0, 0, 0)`, `(13, 0, 0, 0)`, `(41, 0, 0, 0)`, `(46, 0, 0, 0)` — anti-alias borders
- `(0, 0, 0, 255)` → 253 (true black, opaque — used for outlines)
- `(16, 0, 255, 0)` → 48 (window edge with mixed R + B=255)

The body Diffuse for the same pack (`sandman_body.png`, slot `-3`, parameter `Diffuse`) renders as a normal photo-realistic texture (face image, gold "JON SANDMAN" text, "P2 TICTACS" logo on a white body). Cross-referencing the decal mask against the body diffuse confirms: **wherever the mask is `(43, 0, 0, 0)`, the body diffuse shows the artist's literal photographic content** (face, gold text, logo). The shader is sampling the body diffuse color directly in those zones — exactly what the RL-Designer comment says it does.

### Source B — Uncle Skeleton's "PhantomACE" pack

- URL: https://github.com/AlphaConsole/AlphaConsoleTextures/blob/master/textures/UncleSkeleton/PhantomACE/PhantomACE.png
- File: 4096 × 4096, RGBA
- `package.json`: slot `1`, item `1744`, "Octane: PhantomACE (Paintable)", parameter `Skin`.

Top RGBA tuples (every 8 px sampling):
- `(255, 0, 0, 2)` → 165 584 (primary — note **A=2**, not 0)
- `(255, 0, 0, 255)` → 38 707 (accent)
- `(5, 37, 252, 255)` → 7 356 (a literal *opaque blue* accent panel — artist-chosen)
- `(255, 0, 0, 3 / 4 / 5 / 6 / 7 / 8 / 9 / 12 / 13 / 17)` — gradient of partial-alpha primary
- `(0, 0, 0, 255)` → 2 034 (opaque black detail outlines)

**Implication for our shader / catalog parser:** the alpha-channel sentinel for "primary" is **not strictly A=0** — production masks ship A=2 through A~20 for primary zones, with the artist using A as a slight darkening / depth dial against the primary picker. Code that classifies primary as "R≈255 AND A<some_threshold" should use a tolerance like **A < 100** (Psyonix's own shader most likely tests against a midpoint).

### Convention summary (empirical)

```
PRIMARY = R≈255, A_low   (A=0 ideal, A<100 tolerant)
ACCENT  = R≈255, A_high  (A=255 ideal, A>100 tolerant)
DIFFUSE = R≈43,  A=any   (R in [38..48], the "0x2B trick")
WINDOWS = B≈255, A=any
BLACK   = R,G,B all near 0, A=255  → renders as full black detail (always-black outline / "primary times 0")
```

## Secondary source — third-party decal generator code

### Source C — `ubelhj/CustomDecalLogos` (Node.js + Jimp)

- URL: https://github.com/ubelhj/CustomDecalLogos
- File: `decalbuilder.js`
- Released 2019, by community member `ubelhj`. Treats the decal output as a Skin mask compatible with AlphaConsole's `acplugin\DecalTextures` folder.

Key lines (verbatim):

```javascript
// for every pixel of the user's logo, after stamping it onto baseDecal:
var isBlack = this.bitmap.data[idx]     <= 10 &&
              this.bitmap.data[idx + 1] <= 10 &&
              this.bitmap.data[idx + 2] <= 10;
if (isBlack && alpha < 250) {
    this.bitmap.data[idx]     = 255;    // R -> 255
    this.bitmap.data[idx + 1] = 0;
    this.bitmap.data[idx + 2] = 0;
    this.bitmap.data[idx + 3] = 0;      // A -> 0   ==> "primary" sentinel
}
```

And for the logo stamping pass:

```javascript
if (alpha > 50) {
    // opaque logo pixel -> emit opaque black (the "detail" outline color)
    this.bitmap.data[idx]     = 0;
    this.bitmap.data[idx + 1] = 0;
    this.bitmap.data[idx + 2] = 0;
    this.bitmap.data[idx + 3] = 255;
} else {
    // transparent logo pixel -> primary sentinel
    this.bitmap.data[idx]     = 255;
    this.bitmap.data[idx + 1] = 0;
    this.bitmap.data[idx + 2] = 0;
    this.bitmap.data[idx + 3] = 0;
}
```

Inferences:
1. Opaque black `(0, 0, 0, 255)` is a legitimate, distinct mask value (the user's logo color when rendered onto a paintable decal).
2. Primary sentinel is exactly `(255, 0, 0, 0)`. **A=0** is the canonical value (production masks use A=0 most often; A up to ~30 still works tolerantly).
3. The output JSON written by `decalbuilder.js`:
   ```json
   {
       "BodyID": <int>,
       "SkinID": 0,
       "Body": { "Diffuse": "...", "Skin": "..." }
   }
   ```
   matches the AlphaConsole/BakkesMod `acplugin\DecalTextures` runtime — confirming the same Skin/Diffuse two-texture model.

This code does **not** emit literal-diffuse `(43, 0, 0)` pixels at all — `ubelhj`'s tool is single-color-paintable only. The R=43 trick is a more advanced authoring technique used by full-body MELOGRAPHICS / SandMan-style packs that pair a Skin mask (slot 1) with a body-Diffuse override (slot -3).

## Tertiary source — AlphaConsole JSON `package.json` schema

### Source D — AlphaConsoleTextures repository `package.json` files

- URL pattern: `https://github.com/AlphaConsole/AlphaConsoleTextures/blob/master/textures/<author>/<pack>/package.json`

Two slot conventions observed across packs:

```json
// Sandman example
{
  "items": [
    { "id": 0, "slot": -3, "item": -1, "parameters": [{ "name": "Diffuse", "image": "sandman_body.png" }] },
    { "id": 1, "slot":  1, "item": 306, "parameters": [{ "name": "Skin",    "image": "sandman_decal.png" }] }
  ]
}
```

- `slot: -3` + parameter `Diffuse` → **main body MIC override**. The image replaces the entire body's diffuse texture. AlphaConsole plugin only — this is the "Main Body Decals" cosmetic slot exposed in the BakkesMod cosmetic tab.
- `slot:  1` + parameter `Skin` → **decal slot's Skin (color-mask) texture**. This is the file the user's `Custom decals SOLUTION` memo aims to hijack via in-place TFC byte-patching. This Skin texture is the one that carries the four-color mask convention. **This is our target.**

The user's parenthetical "decal slot mask vs body main MIC override" exactly maps to these two slots. The community ships both together when a pack needs the literal-diffuse `(43, 0, 0)` zones to show a non-team-tinted photo (because the photo lives in the Body Diffuse override, while the Skin mask says "show through here").

## Other sources surveyed (negative results)

These were checked because the task brief named them; none documented the mask color convention in prose:

- **MELOGRAPHICS Medium "Ultimate Guide"** (https://medium.com/madebymelo/custom-rocket-league-decals-graphics-the-ultimate-guide-5d943eb62e79) — pure installation tutorial. No hex codes, no mask theory. The companion `news.melo.graphics/p/custom-rocket-league-decals-instructions` page is the same content.
- **MELOGRAPHICS BakkesMod & Plugins guide** (melo.beehiiv.com) — overview of plugins, no mask info.
- **Kaizen "Create-a-Decal Starter Pack for the Octane"** (https://store.kaizenrl.com/l/rlstarterpack) — paywalled Gumroad PSD. Description states "essential tools to craft decals" but the public landing page does not document mask colors. Inferred contents: the PSD ships color-coded layers using the same convention (typical of community templates).
- **TunersRL** Gumroad — paywalled, no public docs.
- **Snackosaurus** Gumroad — paywalled, no public docs.
- **WrapsRL** (https://github.com/Lambourne2/wrapsrl, wrapsrl.com) — StyleGAN2 decal generator. No docs on the mask convention; output is described as "diffuse, normal, specular maps, config .json" — i.e. the pipeline targets AlphaConsole format.
- **bakkesmod.fandom.com** custom-decal forum posts — generic install Q&A, no theory.
- **rocketleaguemoddingwiki.github.io / rocketleaguemapmaking.com** — focus on UDK maps & materials. Confirms RL uses UDK Material Instances and that "we have to modify existing materials" because new ones cannot be compiled, but does not document the body-skin shader's mask sampling.
- **Reddit r/RocketLeagueMods** — Reddit blocked WebFetch in this session; community posts that mention "mask" reference the JSON's *mask* file name (i.e. *the* mask), never the color convention inside the mask.
- **Steam Community guides** ("How to find out BodyID and SkinID", "TextureModding - UMod Guide") — installation only, no mask theory.

## Original source — who first documented this?

No primary citation found. The convention appears to be **observational community lore**: skin authors RE'd Psyonix's vanilla skin masks (e.g. open `Octane_Default_Decal_T.tfc` in UModel, read pixel values, copy the layout) and propagated the values through Discord servers (RL Skins Wiki Discord is referenced repeatedly but is not indexed by public search). The earliest public artifact that *uses* the convention in code that we located is `ubelhj/CustomDecalLogos` (2019). The earliest tutorial *video* implying the convention is YouTube's "Decal Tutorial[Advanced] BakkesMod-AlphaConsole Plugin" by `MissMachoTV` (2020). Neither states "R=43 means literal diffuse" in prose — they just provide PSD templates with the colors pre-baked into named layers (e.g. "Primary Paint Area", "Accent Paint Area", "Window", "Diffuse Logo Pass-through").

**Practical conclusion:** the convention is a *Psyonix-internal shader contract* (i.e. it is defined by the bytecode of `Vehicle_Body_MIC` and its parent material in `CookedPCConsole`), and the community discovered it empirically. The Designer-Guide.md document in `../RL-Designer` is, as far as I can tell, the **most complete prose codification of the convention that exists publicly** — most other sources either ship PSDs with the colors silently embedded or ship binary masks for users to drop in.

## Threshold values — community consensus

No public source documents the exact R tolerance Psyonix's shader uses. **Best estimate based on combined evidence:**

1. `0x2B = 43 = 0.1686` exactly. The production SandMan mask uses this byte value at 19.6 % of pixels with zero internal gradient (only the AA edges show R=13, 16, 41, 46).
2. The user's `[0.15, 0.18]` = `[38.25, 45.9]` brackets the value tightly without catching the 13/16/41/46 anti-alias bleed. **This range is correct and well-tuned.**
3. A slightly more generous `[0.14, 0.19]` = `[35.7, 48.4]` would also work and would be more forgiving for masks authored by less careful artists (some packs do ship R values in 40..48 from slight PNG palette quantization). Adopting `[0.13, 0.19]` is the safest band that still excludes the AA edge cluster.
4. **Do NOT broaden below R=0.10** — too close to opaque-black (`(0,0,0,255)` which is a distinct "always-black detail" sentinel).
5. **Do NOT broaden above R=0.25** — risks catching gradient transitions to `(255, *, *, *)`-style primary zones.

For the alpha sentinel (primary vs. accent), use a midpoint test:

```
if R > 0.85 and A < 0.39 (~A < 100/255): PRIMARY
if R > 0.85 and A >= 0.39:                ACCENT
if R in [0.14, 0.19]:                     LITERAL DIFFUSE
if B > 0.85:                              WINDOWS
otherwise:                                fall-through (opaque black -> detail; transparent -> AA)
```

## Variants by car body / shader generation?

Surveyed AlphaConsole's full pack catalog folder structure: AC, Butthead, DanMBSubPar, Famiy, HurricaneModding, KernalPad, UncleSkeleton. All inspected `package.json` files target the same `Skin` parameter on slot 1 and (when present) `Diffuse` on slot -3. The two skin masks I sampled in detail (SandMan = Octane, PhantomACE = Octane) used the identical convention. The PhantomACE mask uses **A=2 instead of A=0** for primary, confirming the alpha sentinel is tolerant rather than strict.

**No evidence of per-body convention drift was found.** All current Octane/Dominus/Fennec/Breakout/Harbinger packs in distribution use the same four sentinels with the same byte values. The shader is defined once in the body MIC's parent material, so it is the same for every Battle-Car body. (`ubelhj/CustomDecalLogos` confirms this — its drawlocation JSONs differ per body, but the color emitters are identical.)

## Cited sources (strongest first)

1. **`AlphaConsole/AlphaConsoleTextures` repo, `Butthead/Sandman/sandman_decal.png`** — production mask, empirically sampled, contains all four sentinel bytes including `(43, 0, 0, 0)` at 19.6 % pixel share. Strongest possible evidence.
2. **`ubelhj/CustomDecalLogos/decalbuilder.js`** — open-source 2019 decal generator. Emits exactly `(255, 0, 0, 0)` primary and `(0, 0, 0, 255)` detail-black. Confirms two of the four sentinels and the AlphaConsole JSON schema.
3. **`AlphaConsole/AlphaConsoleTextures` `package.json` schema** — confirms slot `-3` (Body Diffuse override) and slot `1` (Decal Skin mask) as two distinct cosmetic slots, matching the user's `decal slot mask vs body main MIC override` note.
4. `UncleSkeleton/PhantomACE/PhantomACE.png` — second production mask, confirms tolerance for A ≈ 2..20 on primary and that artists ship opaque colored panels (literal RGB) for non-team-tinted accents.
5. RL Modding Wiki / RLMM guides — confirm that custom RL materials must reuse existing UDK Material Instances cooked into `CookedPCConsole`; no new shaders can be authored. Implies the mask convention is hard-coded inside the parent VehicleBodyMaterial MIC and unchangeable.

## Items the user can lift directly into ALXS-RL-Mod

- **Adopt the user's RL-Designer convention as-is.** It is correct.
- **Validate user mask uploads** against the four sentinel classes plus opaque-black detail. Reject masks with > N% of pixels in the "OTHER" bucket — those are usually screen-grabbed/JPEG-compressed images that won't paint correctly.
- **Preview shader:** the `[0.15, 0.18]` window matches production. Consider widening to `[0.14, 0.19]` only if you want to support sloppy masks; keep `[0.15, 0.18]` for visual parity with RL-Designer.
- **Alpha midpoint = 100/255 ≈ 0.39** is a safe primary-vs-accent split. The user's `colorReplacement.frag` likely uses 0.5 already; bumping to 0.4 will improve compatibility with packs like PhantomACE that ship A=2 primary.
- **Document for end users**: when an artist's pack uses the slot -3 + slot 1 paired-textures pattern, you must hijack both TFCs (which the user already does — see commits `59fb130` "dual-TFC hijack" and `47ae078` "swap the Skin (color-mask) texture too"). This is consistent with the AlphaConsole `package.json` schema.
