# AC pack ground truth — JSON parameters → BakkesMod SDK textures → in-game shader

> **Investigation date:** 2026-05-15
> **Goal:** locate the EXACT mechanism by which AC's `#2B0000` (R=43) mask
> zones display literal Diffuse RGB in-game, and identify the gap between
> AC's runtime behavior and our file-only TFC-hijack pipeline.

---

## 1. The BakkesMod SDK contract (ground truth)

Source: `BakkesModSDK/include/bakkesmod/core/custom_decals_structs.h` and
`BakkesModSDK/include/bakkesmod/utilities/DecalUtilities.h` (mirrored to
`sandbox/custom_decals_structs.h`).

```cpp
namespace pluginsdk {
    using Tex             = std::shared_ptr<ImageWrapper>;
    using TextureOverride = std::map<std::string, Tex>;     // key = FName of MIC param
    using ColorOverride   = std::map<std::string, LinearColor>;
    using ScalarOverride  = std::map<std::string, float>;

    struct ShaderOverride {
        TextureOverride textures; ColorOverride colors; ScalarOverride scalar;
    };
    struct BodyShaderOverride {
        ShaderOverride body_mic_override;     // applied to Body MIC
        ShaderOverride chassis_mic_override;  // applied to Chassis MIC
        int body_id = -1;
        int skin_id = -1;
    };
}

class DecalUtilities {
    static expected<ApplyDecalToCarResult, string>
        ApplyDecalToCar(const CarWrapper&, const BodyShaderOverride&);
};
```

Key facts:

1. The SDK exposes **a `std::map<string, Texture>`** keyed by arbitrary string.
2. The string MUST equal the **FName of a TextureSampleParameter2D on the body's MIC** (`Body_Paintable_Mat`) for the override to take effect.
3. Valid keys (from `Body_Paintable_Mat`'s parameter list — see `rl_decal_shader_re.md` §2.2): `Diffuse`, `Masks`, `Glass`, `ColorLookup`, `PortalRingTexture`, `PortalRingTexture_02`.
4. `Skin` exists too, **but as a VectorParameter** (a LinearColor) — NOT a TextureSampleParameter2D. So `Body.Skin = <Texture>` would not bind to a texture slot in the parent material.

> The SDK does NOT auto-bind by filename convention. `body_diffuse.png` next
> to a JSON does NOT get auto-bound. **Every texture must be explicitly named
> in the JSON.**

---

## 2. Two distinct JSON schemas

There are **two completely different JSON formats** in the wild. Both end up
calling the same `DecalUtilities::ApplyDecalToCar(BodyShaderOverride)` at the
SDK boundary, but they look nothing alike on disk.

### Schema 1 — AlphaConsole pack manifest (the `package.json` in AC's repo)

```json
{
  "id": 1908, "name": "...", "author": "...",
  "items": [{
    "id": 0, "slot": <int>, "item": <ProductID>,
    "parameters": [{ "name": "Diffuse|Skin|...", "image": "file.png" }]
  }]
}
```

Used by all 32 packs in <https://github.com/AlphaConsole/AlphaConsoleTextures>.
This format encodes **AC product slots** (`slot`, `item`) and is consumed by the
closed-source AC plugin, which translates each item to one or more SDK calls.

### Schema 2 — BakkesMod-SDK-native (what ubelhj / Martinii89 use)

```json
{
  "PackName": {
    "BodyID": 23, "SkinID": 0,
    "Body":    { "Diffuse": "path/diffuse.png", "Skin": "path/skin.png" },
    "Chassis": { ... }
  }
}
```

Source: `ubelhj/CustomDecalLogos/decalbuilder.js` lines 117-124 and
`Martinii89/ReplayManipulatorOpenSource/.../CustomTextures.cpp` (mirrored to
`sandbox/CustomTextures.cpp`). The `Body` and `Chassis` keys map **directly**
into `BodyShaderOverride.body_mic_override.textures` and
`BodyShaderOverride.chassis_mic_override.textures` — one-to-one with the
SDK's `std::map<string, Texture>`.

**Implication:** AC's plugin is doing a NON-TRIVIAL translation from
Schema 1 to the SDK contract. Pattern A and Pattern B are different
*translation strategies* AC applies internally.

---

## 3. AC's two patterns — translation rules

### Pattern A — FullColor (`slot=-3 item=-1`)

Examples: `Famiy/Dominus/EsportsFullColor`, `Butthead/Sandman`,
`KernalPad/HyperBeastDecals`, `AC/MainBodyUtilTextures`.

Pack ships **two distinct PNG roles**, in different `package.json` items:

| pack item             | slot | item | parameter name | file                         |
| --------------------- | ---: | ---: | -------------- | ---------------------------- |
| FullColor decal       | `-3` | `-1` | `Diffuse`      | `DominusAllegiance.png`      |
| MainBody-Only mask    | `1`  | `504`| `Skin`         | `MainBodyOnly.png` `(0,0,0,255)` |

User has to equip BOTH items in-game (the description literally says so:
"Equip the decal called 'Dominus: Main Body Only' and pick the skin you want
from the Main Body dropdown box").

What we believe AC does internally to translate:

```cpp
BodyShaderOverride {
    body_mic_override.textures = {
        { "Diffuse", pack_PNG_decoded },           // from slot=-3 item=-1
        { "Skin",    MainBodyOnly_PNG_decoded },   // from slot=1 item=504 companion
        // possibly also "Masks" if separately set
    },
    body_id = 504, skin_id = 0,
}
```

Both texture overrides go into the **same `body_mic_override.textures` map**
because AC's plugin coalesces overrides by `(body_id, skin_id)` before
calling `ApplyDecalToCar`.

> Confidence: **HIGH**. Backed by (a) the SDK contract,
> (b) ubelhj's open-source `decalbuilder.js` which generates EXACTLY this
> pair `{"Diffuse": ..., "Skin": ...}` for Pattern A,
> (c) the file inventory of every Pattern A pack showing both a FullColor
> PNG and a `MainBodyOnly.png` companion.

### Pattern B — Paintable (`slot=1 item=<bodyID>` with only `Skin`)

Example: `Famiy/Dominus/EsportsPaintable` ships **17 PNGs total** (16 decal
masks + 1 preview), `package.json` references each via:

```json
{ "slot": 1, "item": 1773, "parameters": [{ "name": "Skin", "image": "DomXxx.png" }] }
```

There is **NO companion** and **NO additional PNG**. We verified the directory
listing via the GitHub API — `EsportsPaintable/` contains only `DomXxx.png`
files + `package.json` + `preview.png`. No `*_diffuse.png`, no hidden file.

What AC does internally:

```cpp
BodyShaderOverride {
    body_mic_override.textures = {
        { "Skin", DomAllegiance_PNG_decoded }
        // NO Diffuse override — the body MIC keeps its stock Diffuse
    },
    body_id = 1773, skin_id = 0,
}
```

**This is the critical observation:** in Pattern B, AC overrides only `Skin`
on the body MIC. The body's stock `Diffuse` texture (whatever was baked into
`Body_Paintable_Mat`'s default for this body's MIC, e.g. `Body_Octane_BlankSkin`)
stays bound. At pixels where the user's Skin mask has R=43/255 ≈ 0.169
("literal diffuse" sentinel — see `rl_designer_threshold_origin.md`), the
compiled DXBC pixel shader samples the stock `Diffuse` texture and outputs
it without tint.

> Confidence: **HIGH** on the SDK call shape. Confidence **medium** on what
> the user observes at R=43 zones — the user said "literal RGB diffuse art"
> appears; in Pattern B that's the **stock body's diffuse** (factory paint),
> not the decal artwork. If the user is reporting decal-art-like content
> showing at R=43 zones in a Paintable pack, that probably comes from the
> Skin mask's own RGB content being interpreted as "diffuse-pass-through"
> by the shader, not from a separate Diffuse override.

---

## 4. The actual shader behavior at R=43

Source: `docs_rl/rl_decal_shader_re.md` §1.4 — exhaustive float scan of
`RocketLeague.exe`, `Startup.upk` body, `RefShaderCache`, `GlobalShaderCache`.

- IEEE 754 LE of `0.169 (43/255) = A0 14 2D 3E` → **0 hits** in every binary
- IEEE 754 LE of `0.18              = 89 88 38 3E` → **0 hits**

So the `[0.15, 0.18]` literal-diffuse band that RL-Designer uses is **not** a
direct quote of any constant in the compiled shader. However, the user's
in-game testing in `TODO.md` line 35 confirms `#2B0000` empirically produces
literal-diffuse output. The most plausible source: a DXBC shader instruction
chain that uses an `abs(r - 43/255) < tol` or `floor(r * N) == k` test
where the constants are not exactly `0.169` / `0.18` but some equivalent
fixed-point comparison baked into the DXBC.

What matters for us: **at the affected pixels the shader reads the BOUND
`Diffuse` texture parameter and outputs its RGB unmodified**.

---

## 5. Exact file mappings from package.json → SDK call

### Butthead/Sandman (Pattern A, Octane)

```
items[0]: slot=-3 item=-1 Diffuse sandman_body.png   → body_mic.textures["Diffuse"] = sandman_body
items[1]: slot=1  item=306 Skin    sandman_decal.png → body_mic.textures["Skin"]    = sandman_decal
                                                       body_id=23 (Octane), skin_id=0
```
(`body_id=23` is the Octane body asset ID; `item=306` is the equip product ID
for "Octane: Sandman" — distinct from the body asset ID. AC's plugin maps
product-ID → body-ID internally before calling the SDK.)

### Famiy/Dominus/EsportsFullColor (Pattern A, Dominus)

```
items[0..5]: slot=-3 item=-1 Diffuse DominusXxx.png  → body_mic.textures["Diffuse"]
items[6]:    slot=1  item=504 Skin   MainBodyOnly.png → body_mic.textures["Skin"]
                                                       body_id=Dominus, skin_id=0
                                                       MainBodyOnly = (0,0,0,255) constant
```

### Famiy/Dominus/EsportsPaintable (Pattern B)

```
items[0..16]: slot=1 item=1773 Skin DomXxx.png       → body_mic.textures["Skin"]
                                                       body_id=Dominus, skin_id=PaintableVariant
                                                       NO Diffuse override
```

---

## 6. The KEY DIFFERENCE — why our pipeline shows white where AC shows literal RGB

Our current `Skin_Octane_Stars_SF.upk` hijack pipeline
(`src-tauri/src/commands/custom_decals.rs` + `decal_swap.rs`):

1. Renames `skin_octane_galefire_SF.upk`'s package + MIC + texture FNames to
   `Stars`.
2. Hijacks `Textures.tfc` → `MyDecal_.tfc` to carry the **user's PNG as the
   Diffuse texture** (the texture exported as `Skin_Octane_Stars_RGB` is
   bound to the MIC's `Diffuse` parameter, contrary to its `_RGB` suffix —
   see the dump in `docs_rl/custom_decals_FINAL_PLAN.md` TL;DR).
3. Hijacks `Textures7.tfc` → `MyDecal02.tfc` to carry a Skin/mask texture
   (when user provides a mask PNG; else absent).

This is structurally **identical to AC's Pattern A FullColor** — same two
texture parameters on the same body MIC. So why does ours render white?

### Hypothesis (high confidence): the `Skin` texture override is missing OR is white-default

The `Octane_GaleFire_MIC` we hijack has the following stock parameters
(per `custom_decals_FINAL_PLAN.md` line 10-15):

```
[0] CurvaturePack → Body_Force_Curvature
[1] Diffuse       → Force_Body_D            ← we replace with user PNG
[2] Skin          → Force_Body_BlankSkin    ← stock value if user provides no mask
```

`Force_Body_BlankSkin` is the **blank-skin base**, which is a near-white
texture. In the Body_Paintable_Mat shader, when `Skin` has high RGB values
everywhere, the team/custom-color paint pass DOES run and the literal-diffuse
band `r ≈ 0.169` never gets hit (because the stock BlankSkin's R is ~0
everywhere, but the paint shader treats r ≈ 0 as "fully primary paint zone"
— not "literal diffuse").

**Without a mask PNG that has `R = 43/255` ≈ 0.169 in the affected zones,
the shader never enters the literal-diffuse code path, and the paint shader
overwrites the Diffuse output with the player's (or fallback white) team
color.**

### What AC's `MainBodyOnly.png` actually is at the byte level

From `alphaconsole_packs_forensics.md` §3:

| File                                     | Mode | Per-pixel RGBA       |
| ---------------------------------------- | ---- | -------------------- |
| `Famiy/Dominus/EsportsFullColor/MainBodyOnly.png` | RGBA | **(0, 0, 0, 255)**   |
| `AC/MainBodyUtilTextures/MainBodyOnly.png`        | RGB  | (mostly 0, watermark) |

Pure `(0, 0, 0, 255)` everywhere. Critically: **R = 0, not R = 43.**

That's a contradiction with the literal-diffuse-band hypothesis above.

### Reconciliation — two distinct shader code paths

There are TWO ways the body shader can output literal diffuse:

| Mechanism                         | Trigger                                                  | Where it comes from                |
| --------------------------------- | -------------------------------------------------------- | ---------------------------------- |
| **`#2B0000` literal-diffuse band**| `R = 43/255 ≈ 0.169` on the Skin mask at a pixel         | Per-pixel shader branch            |
| **`MainBodyOnly` neutralization** | `R = 0` (no primary paint) AND `B = 0` (no secondary) AND `A = 255` (no decal cutout) | Per-pixel shader: paint contribution is `lerp(stock, primary, R) + lerp(stock, secondary, B)`. With R=B=0 the paint contribution **collapses to the stock Diffuse**. |

These mechanisms are NOT equivalent:

- The `#2B0000` band is a **per-pixel opt-in flag** designed for use inside
  paintable decals (you can paint your car normally; only the marked pixels
  show literal diffuse).
- `MainBodyOnly.png` `(0,0,0,255)` does the same thing for the WHOLE TEXTURE:
  every pixel ends up showing the stock Diffuse texture, with no paint
  influence at all.

In both cases the **bound `Diffuse` texture must contain the user's art**
for the user to see it.

### Concrete claim — what we need to do that we're not doing

1. Verify our `Diffuse` hijack is actually being read by the shader. The
   donor's `Skin_Octane_GaleFire_RGB` (bound to `Diffuse`, despite the name)
   may be loaded from a different TFC than `Textures.tfc`. If the `_RGB`
   texture's `TextureFileCacheName` post-rename is NOT pointing at
   `MyDecal_`, our PNG bytes never reach GPU.

2. The mask we provide (Textures7 → `MyDecal02`) must be `(0,0,0,255)` for
   FullColor mode, exactly mirroring AC's `MainBodyOnly.png`. If we're
   shipping no mask, the stock `Force_Body_BlankSkin` stays bound and its
   R-channel content drives paint, masking our Diffuse.

3. The PARENT material of the hijacked MIC matters. `MIC_Body_Paintable_All`
   reads `Diffuse` as a TextureSampleParameter2D. If `Octane_Stars_MIC`'s
   parent chain doesn't lead back to `Body_Paintable_Mat`, the `Diffuse`
   parameter we set has no effect — it goes to dev/null.

---

## 7. A TEST TO VERIFY THE HYPOTHESIS

**Test:** Install Famiy/Dominus/EsportsFullColor through AC (offline, since
EAC is back) and capture the actual loaded textures on Dominus's body MIC at
the moment the FullColor decal is equipped.

If we can't run AC live, the **file-only equivalent** test:

1. Build a known-good red-only Diffuse PNG: `2048² (R=255, G=0, B=0, A=255)`.
2. Build the AC-canonical Skin mask: `2048² (R=0, G=0, B=0, A=255)` (the
   `MainBodyOnly.png` byte-equivalent).
3. Run our pipeline with **both** PNGs in FullColor mode (the diffuse goes
   to `MyDecal_.tfc`, the mask to `MyDecal02.tfc`).
4. Equip Stars in-game. Expected: solid red body. Observed:
   - If **red** → our pipeline replicates AC correctly; user's "white"
     observation was due to NOT supplying the mask.
   - If **still white** → the `Skin` parameter binding on `Octane_Stars_MIC`
     is broken (not pointing at our hijacked TFC), and we need to inspect
     the post-rename `TextureFileCacheName` of `Skin_Octane_Stars_RGB`
     and `Skin_Octane_Stars_BlankSkin` exports.
   - If **partially red with some hue tint** → the `Diffuse` binding works
     but a parent-material default ScalarParameter is modulating it (e.g.
     `SolidTeamColor` static switch is True somewhere up the MIC chain).

The cheapest signal: byte-inspect the modified `Skin_Octane_Stars_SF.upk`
after the rename and confirm:

- The MIC named `Octane_Stars_MIC` has 3 `TextureParameterValues`:
  `CurvaturePack`, `Diffuse`, `Skin`.
- The Texture2D export bound to `Diffuse` has
  `TextureFileCacheName = "MyDecal_"`.
- The Texture2D export bound to `Skin` has
  `TextureFileCacheName = "MyDecal02"`.
- Both TFCs (`MyDecal_.tfc`, `MyDecal02.tfc`) exist on disk in `CookedPCConsole/`.

If any of those fails, the user is seeing white because **the shader is
falling back to the parent-material defaults**, not because the shader
disagrees with what AC does.

---

## 8. Bottom-line answer to the original question

> "What's the EXACT mapping from JSON parameters → BakkesMod SDK textures
> map for Pattern B paintable?"

**Pattern B (Paintable) maps to:**

```
BodyShaderOverride {
    body_mic_override.textures = { "Skin": <pack PNG> },
    chassis_mic_override.textures = {},
    body_id = <body asset ID>, skin_id = <variant>,
}
```

**ONLY `Skin`. No `Diffuse`.** The body's stock `Diffuse` (factory paint
texture for that body, e.g. `Octane_Body_D`) stays bound. The shader's per-
pixel literal-diffuse code path activates at `r ≈ 43/255` in the supplied
Skin mask, and at those pixels the stock Diffuse is sampled and output.

> "What MIC does AC's body_mic_override actually target?"

**`Body_Paintable_Mat` (or one of its instances like `MIC_Body_Octane`,
`Octane_GaleFire_MIC`, etc.) — the MIC currently bound at the body mesh's
SkinMaterialIndex slot.** AC's plugin finds the right MIC by looking up
the car's currently-equipped decal's MIC. Confidence: **HIGH** — this is
how `BodyShaderOverride` is defined in the SDK; the body's MIC at the time
of `ApplyDecalToCar` is whatever decal the user has equipped.

> "Key difference between AC's runtime and our file-only pipeline?"

AC overrides `Skin` (and optionally `Diffuse`) on whatever MIC the user
already has equipped — could be any decal. Our pipeline overrides those
parameters on **only** `Octane_Stars_MIC`, and only after we've renamed a
GaleFire donor UPK to look like Stars. If the user doesn't equip Stars
specifically, our overrides don't activate. If they do equip Stars but
we ship no Skin mask (FullColor mode pipeline must auto-generate one), the
stock `Force_Body_BlankSkin` stays bound and the paint shader contributes
tint that whites out the Diffuse output.

> "A test to verify?"

Run our pipeline in FullColor mode with a **solid-red Diffuse** + the
**AC-canonical `(0,0,0,255)` Skin mask** as described in §7. The expected
result is a solid-red Octane body when Stars is equipped.

---

## Appendix — files for future reference

- `sandbox/CustomTextures.h` / `.cpp` — Martinii89's full source (BakkesMod-native schema)
- `sandbox/custom_decals_structs.h` — BakkesMod SDK ground-truth contract
- `sandbox/DecalUtilities.h` — SDK entry point
- `sandbox/ubelhj_decalbuilder.js` — proves `{"Diffuse": …, "Skin": …}` is the canonical Pattern A SDK call shape
- `docs_rl/ac_pkgs/textures_Famiy_Dominus_EsportsPaintable_package.json` — Pattern B reference
- `docs_rl/ac_pkgs/textures_Famiy_Dominus_EsportsFullColor_package.json` — Pattern A reference
- `docs_rl/ac_pkgs/textures_Butthead_Sandman_package.json` — Pattern A reference (Octane)
- `docs_rl/alphaconsole_packs_forensics.md` — full pixel-level analysis of MainBodyOnly.png
- `docs_rl/rl_decal_shader_re.md` — DXBC threshold scan (proves 0.169 / 0.18 not literal constants)
- `docs_rl/rl_designer_threshold_origin.md` — origin of the `#2B0000` convention
- `docs_rl/custom_decals_FINAL_PLAN.md` — current dual-TFC FullColor architecture
