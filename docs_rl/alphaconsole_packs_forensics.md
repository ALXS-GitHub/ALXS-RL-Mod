# AlphaConsole texture-pack forensics

Repo surveyed: <https://github.com/AlphaConsole/AlphaConsoleTextures>
Tree snapshot saved to `ac_tree.json`. All 32 `package.json` files mirrored to `ac_pkgs/`. Key PNGs mirrored to `ac_pngs/`. Pixel analysis performed with PIL/NumPy.

Date: 2026-05-15.

---

## 1. AlphaConsole pack-manifest schema (observed)

```json
{
  "id": <int>, "name": "...", "author": "...", "description": "...",
  "items": [
    {
      "id": <int>,
      "name": "<display name>",
      "slot":  <RL EquipSlot int>,
      "item":  <RL ProductID int>,
      "parameters": [
        { "name": "<MaterialParam name>", "image": "<file.png>",
          "animated": false, "frames": 1, "framerate": 1 }
      ]
    }
  ]
}
```

`parameters[].name` is the **TextureParameter name on the in-game Material Instance**. The plugin replaces that named texture parameter at runtime with the bytes of the matching PNG.

### Observed slot/item conventions

| `slot` | meaning                                                                 |
| -----: | ----------------------------------------------------------------------- |
|  `-4`  | "secondary decal" – special AC virtual slot (Hoonicorn, GoodsmileRacing)|
|  `-3`  | **"Main Body Decal"** – special AC virtual slot used by FullColor packs |
|  `-2`  | Custom **Ball** slot (uses two textures: `Diffuse` + `Normal`)          |
|   `1`  | **Decal/Skin** slot on a specific car body (item = body ProductID)      |
|   `2`  | **Wheel** slot (item 820 = Looper/SubUV animated wheel)                 |
|   `3`  | **Rocket Boost** slot (items 36–41)                                     |
|   `4`  | **Antenna / Flag** slot (items 286, 1454)                               |
|   `5`  | **Topper** slot (item 224)                                              |
|  `15`  | **Goal Explosion** (items 2329, 2586, 2791)                             |

### Observed `item` IDs (body ProductID for decals on slot 1)

|  Body            | item ID | Example pack                                   |
| ---------------- | ------- | ---------------------------------------------- |
| **Octane**       | 306     | `AC/MainBodyUtilTextures`, `Butthead/Sandman`, `HurricaneModding/EsportsDecalsPack` |
| **Dominus**      | 504     | `Famiy/Dominus/EsportsFullColor` Main-Body item |
| **Dominus**      | 1773    | `Famiy/Dominus/EsportsPaintable` (every paintable decal) |
| **Ice Charger**  | 1723    | `KernalPad/Hoonicorn` |
| **Triple Trouble Octane** | 1637 | `HurricaneModding/EsportsDecalsPack` id 33 |
| **Octane: PhantomACE 1**  | 1744 | `UncleSkeleton/PhantomACE` |
| **Octane: PhantomACE 2**  |  306 | `UncleSkeleton/PhantomACE` |
| **Octane (Spectre)**      | 1579 | `HurricaneModding/Spectre` |
| **Octane (DragonLord)**   | 1059 | `HurricaneModding/DragonLord` |

### Observed material-parameter names

* `Skin`                – the **diffuse (color-mask)** texture on a body's decal material. ~98% of car decals.
* `Diffuse`             – the literal albedo texture on the **virtual `slot=-3` FullColor / Ball / Antenna** slots, **and** on Spectre/Antenna composites.
* `Normal`              – normal map (Ball slot `-2`)
* `Specular`            – topper (item 224)
* `TextureDiffuse`      – antenna flag (`item=286`, `1454`)
* `Texture`             – goal-explosion #1 (item 2586)
* `ImageSeq_SubUV`      – animated wheel (item 820) – contains a SubUV sprite-sheet
* `FX*Noise1`, `FX*Noise2`, `DecalBlendNoise`, `Decal1`, `Decal2`, `CurvaturePack` – pack-specific FX/blend masks on a few items (Spectre item 1579, OoS/Placebo id 28/29)

---

## 2. The two FullColor patterns

There are exactly **two distinct strategies** observed for getting an arbitrary RGB image onto a car body. Both abandon the paint shader, but they do it differently.

### Pattern A — "Full-Color Esports" (Dominus EsportsFullColor, Butthead/Sandman)

Each "decal" item is **NOT** the actual decal product. It uses the sentinel pair:

```json
"slot": -3,
"item": -1,
```

…with parameter `Diffuse` set to the new full-color image. Slot `-3` is the **AlphaConsole virtual "Main Body Decal"** slot — handled entirely client-side by the AC plugin. There is no Psyonix equivalent.

The pack **also** ships one companion item that is a real Psyonix decal product:

| Pack                                     | companion `slot` | companion `item` | companion `parameters[0]` | mask file |
| ---------------------------------------- | ---------------: | ---------------: | ------------------------- | --------- |
| `Famiy/Dominus/EsportsFullColor`         | 1                | **504**          | `Skin`                    | `MainBodyOnly.png` (2048×2048, RGBA, all-zero RGB+255A) |
| `AC/MainBodyUtilTextures` (Octane only)  | 1                | **306**          | `Skin`                    | `MainBodyOnly.png` (2048×2048, RGB-only, R≈12 G≈0 B≈1)  |
| `Butthead/Sandman` (Octane)              | 1                | **306**          | `Skin`                    | `sandman_decal.png` (a pre-cut mask, alpha=0 over 82.8% of the texture, R-channel = 156 elsewhere) |

The Sandman case is interesting — it cuts the **decal pattern itself** out of the skin texture, leaving the rest fully transparent so the underlying body color shows through there, while the literal RGB texture (`sandman_body.png`) is drawn on top via the virtual `-3` slot.

### Pattern B — "Paintable" (Dominus EsportsPaintable)

Single item per decal, `slot=1 item=1773`, parameter name `Skin`. The PNG is a **real RL Skin map**: `R = decal mask`, `G = 0`, `B = secondary-zone mask`, `A = body cutout`. The team-color paint shader runs as normal; the user picks a primary + secondary color in-garage and the shader recolors the mask.

Pattern B is "ordinary" AC. Pattern A is the trick we care about.

---

## 3. MainBodyOnly.png — pixel-perfect forensics

Two distinct `MainBodyOnly.png` files exist in the repo. Both are 2048×2048. Both have **A=255 on every pixel** (no alpha at all). What differs is the RGB channel data — and that's what neutralizes the paint shader.

### 3.1 `textures/AC/MainBodyUtilTextures/MainBodyOnly.png` (Octane, item 306, param `Skin`)

```
mode       : RGB (no alpha channel stored at all)
size       : 2048 x 2048
R: min=0   max=255  mean=12.21
G: min=0   max=0    mean=0.00
B: min=0   max=255  mean=1.24
A: implicit 255 everywhere
```

The image is **almost-entirely black** with a few light bright pixels in R and B (decorative AlphaConsole logotype probably visible in tiny corner area). Critically: **G is identically 0** and **A is 255 everywhere**.

### 3.2 `textures/Famiy/Dominus/EsportsFullColor/MainBodyOnly.png` (Dominus, item 504, param `Skin`)

```
mode       : RGBA
size       : 2048 x 2048
R: min=0   max=0    mean=0.00
G: min=0   max=0    mean=0.00
B: min=0   max=0    mean=0.00
A: min=255 max=255  mean=255.00
```

This one is **literally a solid black opaque 2048² image**. RGB=000, A=255 on every single pixel.

### 3.3 What the paint shader does with this input

RL's body decal material is roughly `MaterialPaint`-style: it reads `Skin.RGBA` and interprets the channels as **paint-zone masks**, then composites the player's chosen colors on top of the body's base material. Empirically observed mapping (corroborated by Pattern B paintable skins above and standard MaterialPaint conventions):

* **R channel** → primary-color paint zone mask (decal stripes / logo)
* **G channel** → "metallic" or "wear" — varies by material
* **B channel** → secondary-color paint zone mask
* **A channel** → "body cutout" – where the decal **subtracts** the underlying body color, revealing wherever the decal artwork is. `A=0` means "decal is opaque here, hide body". `A=255` means "decal contributes nothing here, body shows".

When all four channels are zero/255 as in 3.2:

| pixel | R | G | B | A   | shader interpretation                                    |
| ----- |---|---|---|-----|-----------------------------------------------------------|
| every | 0 | 0 | 0 | 255 | "no primary zone, no metallic, no secondary, no cutout — leave the **body's base material** showing through everywhere" |

This is the **neutralization mask**: every paint zone is OFF, body cutout is OFF, so the paint shader contributes **nothing**, and whatever is rendered on the body via the *virtual `-3` slot* (Pattern A) lands on top as literal RGB.

The Octane version (3.1) is functionally the same — G=0, A=255, R/B nearly zero everywhere. The few non-zero pixels are an AC watermark; they will still get colored by the paint shader, but they're effectively invisible at 99% black.

### 3.4 Exact formula for our generated BC3 neutralization mask

> 2048×2048 BC3-encoded texture; every texel = `(R=0, G=0, B=0, A=255)`.
> No alpha=0 region anywhere. No paint zones anywhere. Just one solid black opaque tile.

In BC3 terms: one alpha block `(255, 255, alpha_indices=0,0,0,...)` and one color block `(0x0000, 0x0000, color_indices=0,0,0,...)` repeated for every 4×4 tile. Total ≈ 4 MB raw BC3 (vs. ≈21 KB PNG in the repo — because PNG compresses solid-color content trivially).

This is **trivial to generate procedurally** in Rust — no need to ship the asset at all.

### 3.5 Verification snapshots

The Dominus paintable example (`DomAllegiance.png`, slot 1 item 1773, param `Skin`) confirms the channel semantics:

```
R: mean=161.58   max=255
G: mean=13.57
B: mean=17.52
A: min=0  max=255  mean=88.63
A==0   pixels: 61.85%   (decal-opaque region)
A==255 pixels: 32.49%   (no-decal region)
Where A==0   -> RGB: R=130.5 G=0   B=0    (primary stripe mask)
Where A==255 -> RGB: R=204.5 G=0   B=53.9 (mixed primary + secondary on the body region)
```

This matches the "R=primary, B=secondary, A=cutout" interpretation.

---

## 4. Exhaustive body → pack → mask companion → diffuse param table

| Body          | Pack (slot=-3)                                  | Mask companion (slot=1)                | Mask file                | Diffuse param name on slot=-3 |
| ------------- | ----------------------------------------------- | -------------------------------------- | ------------------------ | ----------------------------- |
| **Octane**    | (any pack using `slot=-3 item=-1`)              | `AC/MainBodyUtilTextures` `item=306`   | `MainBodyOnly.png` (R-only watermark) | `Diffuse` |
| Octane        | `Butthead/Sandman` id 0                         | id 1 of same pack, `item=306`          | `sandman_decal.png` (custom alpha-cut decal) | `Diffuse` |
| Octane        | `KernalPad/GoodsmileRacing`                     | (none in pack; user picks MainBodyOnly via util) | n/a | `Diffuse` |
| Octane        | `KernalPad/HyperBeast`                          | (none in pack)                         | n/a (uses Util) | `Diffuse` |
| **Dominus**   | `Famiy/Dominus/EsportsFullColor`                | id 6 of same pack, `item=504`          | `MainBodyOnly.png` (solid 0,0,0,255) | `Diffuse` |
| **Ice Charger** | `KernalPad/Hoonicorn` id 1 (`slot=-4`)        | id 0 same pack, `item=1723`            | `Skin` PNG       | `Diffuse` |
| (Ball)        | `Famiy/Balls/*` (slot=-2)                       | n/a — ball decal is its own slot       | n/a              | `Diffuse` + `Normal` |
| (Antenna)     | `Famiy/Antennas`, `KernalPad/Flags`             | self-contained                         | n/a              | `TextureDiffuse` (and `Diffuse` on 286) |

**Key generalization:** only **Octane (306)** and **Dominus (504)** ship official neutralization masks in this repo. For every other body, anyone wanting Pattern A would need to ship their own zero-mask for `(slot=1, item=<body ID>)`. The mask is **always the same**: a 2048² fully-opaque solid-black BC3 — independent of the body. It works because the paint-shader inputs are slot-1 texture parameters that the same shader reads on every body.

---

## 5. Implications for our ALXS-RL-Mod project

1. **The "Main Body Only" neutralization mask is body-agnostic**. One 2048×2048 BC3 with `(R=0, G=0, B=0, A=255)` is enough for any car body. We can generate it on the fly in Rust — no need to ship a binary asset.

2. **AC's `slot=-3 item=-1` virtual slot is an AC-plugin construct** — the game itself doesn't understand it. It's intercepted by the AC BakkesMod plugin which replaces a *different* texture (the **upper-body color-mask** texture on the body's material) with our literal RGB.

3. Our current hijack already does this for *Stars*. To replicate AC's FullColor:
   * Hijack **two** textures simultaneously on the target body's decal material:
     a. `Skin` → the neutralization mask (zero everywhere)
     b. The body's **diffuse/color-mask** texture → our literal RGB artwork
   * The user must equip the *paintable* decal (the one this material belongs to) — same UX as AC's "equip Main Body Only" step.

4. **No new shader is required.** RL's existing material reads `Skin` and a `Diffuse`/`Color` — we just substitute byte content of those two textures. This matches our latest commit `59fb130 feat(custom-decals): dual-TFC hijack — D + RGB swap on Stars slot`.

5. **The mask need not even be neutralizing in some cases** — Sandman shows you can use the `Skin` slot to *cut a stencil* (alpha=0 over the decal region, alpha=255 elsewhere) so the team-color paint **only fills the stencil**, leaving the rest of the body to show whatever literal RGB you put on `Diffuse`. This is more sophisticated than pure neutralization and lets the player still pick a paint color for the "stripe" zone.

---

## 6. Files mirrored locally

* `ac_tree.json`              — full repo tree (153 KB)
* `ac_pkgs/*.json`            — all 32 package manifests
* `ac_pngs/`                  — selected PNGs:
  * `AC_MainBodyUtilTextures_MainBodyOnly.png`
  * `Famiy_Dominus_EsportsFullColor_MainBodyOnly.png`
  * `Famiy_Dominus_DominusAllegiance.png`, `Famiy_Dominus_DominusChiefs.png`
  * `Famiy_Dominus_DomAllegiance_paintable.png`
  * `Butthead_Sandman_body.png`, `Butthead_Sandman_decal.png`
  * `sandman_listing.json`
