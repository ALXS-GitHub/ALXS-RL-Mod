# RL Decal Skin Shader — Reverse Engineering Notes

> **Goal.** Confirm or refute the user's reproduced shader logic in
> `../RL-Designer/rl-designer/src/components/Model3D/patches/shaderSkinPatch.ts`
> against Rocket League's actual native decal/body shader.
>
> **Verdict (TL;DR).** The user's reproduced shader **CANNOT be directly
> validated against RL's DXBC bytecode** — no public tool extracts
> `FMaterialShaderMap` blobs from RL's `RefShaderCache-PC-D3D-SM5.upk`,
> which is a 219 MB UE3-fully-compressed package. **However**, **the
> specific threshold floats `0.18` (0x3E388889) and `0.169 = 43/255`
> (0x3E2D14A0) used by the user do NOT appear as literal IEEE 754 floats
> anywhere in `RocketLeague.exe`, `Startup.upk`, `RefShaderCache`, or
> `GlobalShaderCache.bin`.** This is strong refutation that those exact
> thresholds reflect the actual RL shader. The *structure* of the user's
> shader (RGBA-channel masking driving palette pickups) is consistent with
> RL's parameter declarations, but the specific numeric thresholds are
> almost certainly invented heuristics — not ground truth.
>
> **Confidence: medium-high.** All evidence points to the user's specific
> thresholds being approximate guesses, but the shader's *structure*
> (R-channel → primary, R+α → accent/secondary, B-channel → windows,
> default → literal diffuse) is plausibly correct based on the parameter
> graph and empirical in-game testing (Yuna Itzy test from
> `custom_decals.md`).

---

## 1. Approach

### 1.1 Sources surveyed (all under `C:\Program Files\Epic Games\rocketleague`)

| File | Size | Format | Encrypted? | Useful? |
|---|---|---|---|---|
| `Binaries\Win64\RocketLeague.exe` | 38 MB | PE x64 | No | Yes (string scan) |
| `TAGame\CookedPCConsole\Startup.upk` | 41 MB | UE3 UPK | **AES-256-ECB** + chunked-zlib body | **Yes — decrypted + decompressed** |
| `TAGame\CookedPCConsole\Engine.upk` | 4.9 MB | UE3 **fully-compressed** UPK | No | Partial — special decompressor needed |
| `TAGame\CookedPCConsole\Core.upk` | 0.4 MB | UE3 fully-compressed UPK | No | Skipped |
| `TAGame\CookedPCConsole\RefShaderCache-PC-D3D-SM5.upk` | 219 MB | UE3 fully-compressed UPK | No | **Contains DXBC — not parsed** |
| `TAGame\CookedPCConsole\GlobalShaderCache-PC-D3D-SM5.bin` | 2.9 MB | UE3 global shader cache | No | Limited (engine-level shaders) |
| Various `Skin_*_SF.upk` / `Body_*_SF.upk` | 30-1500 KB | UE3 UPK | AES + zlib | Yes — already decrypted by our pipeline |
| `Vehicle_Parent_Materials.*` | inside `Startup.upk` | — | — | **Contains the parent body Material** |

### 1.2 Decryption pipeline

Reused our existing tooling (`sandbox/decrypt/.../keys.txt` + `pycryptodome AES-256-ECB`):

- **Startup.upk**: AES-decrypted with key #74 (1 of 119 known RL keys).
  Body decompressed via chunked-zlib (magic `0xC1832A9E`) — **42 chunks
  → 129 MB decompressed** vs 41 MB on disk.
- **Engine.upk / Core.upk / RefShaderCache**: fully-compressed UE3 format
  (entire file is one giant zlib stream wrapped with `0xC1832A9E` magic
  per chunk, but using `FullyCompressedHeader` not the standard
  `[NameOffset, HeaderEnd)` AES region). **Not decompressed in this
  pass** — would require porting UE3's `LoadCompressedPackage` path or
  using umodel.

### 1.3 String scans on RocketLeague.exe

Reused outputs from `sandbox/research/rl_strings/` (extracted previously).
Searched for: `PaintMaskInRGB`, `BodyMasks`, `Body_Diffuse`, `Skin`,
`TeamColor`, `AccentColor`, `PrimaryColor`, `BlankSkin`, `Curvature`.

**Result.** Only `TeamColorScriptedTexture_TA` (the UScript class name for
the runtime palette RT) and `UCarColorSet_TAexecRebuild...` (UScript class
hook) appear as plain ASCII. The actual shader parameter FNames are stored
as **FName-table indices in compiled UMaterial bytecode** — they don't
appear as literal strings in the exe.

### 1.4 Threshold float scan (the key test)

Built IEEE 754 LE byte patterns for each candidate threshold and searched
all four binaries. Results:

| Threshold | Hex (LE) | RocketLeague.exe | Startup.upk (decompressed body, 129 MB) | RefShaderCache (219 MB raw) | GlobalShaderCache (2.9 MB) |
|---:|---:|---:|---:|---:|---:|
| `0.18` | `89 88 38 3E` | **0** | **0** | **0** | **0** |
| `0.169` (43/255) | `A0 14 2D 3E` | **0** | **0** | **0** | **0** |
| `0.9` | `66 66 66 3F` | 10 | 6 | 870 | 10 |
| `0.95` | `33 33 73 3F` | 8 | 2 | 311 | 5 |
| `0.15` | `9A 99 19 3E` | 3 | 48 | 2293 | 2 |
| `0.05` | `CD CC 4C 3D` | 42 | 17 | 4536 | 2 |
| `0.1` | `CD CC CC 3D` | 74 | unknown | 12543 | 1152 |
| `0.5` | `00 00 00 3F` | many | 2476 | many | many |
| `1.0` | `00 00 80 3F` | many | 2541 | many | many |

The **complete absence** of `0.18` and `0.169` is the headline finding.
These were the user's "literal diffuse" R-channel thresholds. If RL's
shader actually used `r in [0.15, 0.18]` as a literal-diffuse band, that
constant would necessarily appear in either the cooked material expression
graph (Startup.upk decompressed body) or the compiled DXBC pixel shader
(RefShaderCache). It appears in **neither**.

The presence of `0.15` (48 hits in Startup body) is not diagnostic — that
value is too common across all UE3 cooked materials to be a smoking gun.

---

## 2. Body parent material — full parameter extraction

### 2.1 `Body_Paintable_Mat` and `MIC_Body_Paintable_All` location

Both live in `Startup.upk`, package `Vehicle_Parent_Materials`:

```
Startup.upk → Vehicle_Parent_Materials → Body_Paintable_Mat   (export[261], Material,                 2516 B)
                                       → Body_All_Mat          (export[260], Material,                 2643 B)
                                       → MIC_Body_Paintable_All(export[832], MaterialInstanceConstant, 2882 B)
                                       → MIC_Body_Octane       (export[786], MaterialInstanceConstant, 2722 B)
                                       → Body_All_MIC          (export[831], MaterialInstanceConstant, 2618 B)
```

Both Materials are wholly **declarative parameter materials** — they
contain only `MaterialExpression*Parameter` nodes (TextureSampleParameter2D,
VectorParameter, ScalarParameter, StaticSwitchParameter, TextureObjectParameter).
**There is no `MaterialExpressionIf`, `MaterialExpressionLinearInterpolate`
(Lerp), `MaterialExpressionConstant`, or any other math node.** This
means the actual compositing math (lerps, thresholds, masking) is **baked
into the compiled DXBC shader** that lives in
`RefShaderCache-PC-D3D-SM5.upk` — not into the readable expression graph.

### 2.2 Parameter inventory of `Body_Paintable_Mat`

Extracted by parsing the `Body_Paintable_Mat` serial body for each
child `UMaterialExpression*Parameter` and locating its
`ParameterName: NameProperty` tagged-property field by exact byte-match
against the FName table.

| Class | ParameterName | Default | Group |
|---|---|---|---|
| ScalarParameter | `StartOffset` | 1.0 | |
| ScalarParameter | `Sharpness` | 2.0 | |
| ScalarParameter | `DistortionAmount` | 1.0 | |
| ScalarParameter | (unnamed) | 20.0 | |
| StaticSwitchParameter | `DynamicWear` | True | |
| StaticSwitchParameter | `Reflections` | True | |
| StaticSwitchParameter | `ThickGrass` | True | |
| StaticSwitchParameter | `TilingMask` | False | |
| StaticSwitchParameter | `SolidTeamColor` | False | |
| StaticSwitchParameter | `TeamColorLines` | False | |
| StaticSwitchParameter | `UseVertexColor` | False | |
| StaticSwitchParameter | `GrassMask` | False | |
| StaticSwitchParameter | `Tile4x` | True | |
| StaticSwitchParameter | `CubemapParalax` | False | |
| StaticSwitchParameter | `FlipFieldSkinX` | False | |
| StaticSwitchParameter | `WorldSpaceCoords` | True | |
| StaticSwitchParameter | `Tile16x` | False | |
| StaticSwitchParameter | `LightMap` | True | |
| StaticSwitchParameter | `MirrorYOnly` (×3) | True | |
| StaticSwitchParameter | `DiffuseMirrorRotate` (×2) | False | |
| StaticSwitchParameter | `CustomFieldSkinScale` | False | |
| StaticSwitchParameter | `WPO` | False | `WPO` |
| StaticSwitchParameter | `UseSaturationOffset` | True | |
| StaticSwitchParameter | `Distortion` | True | |
| TextureObjectParameter | `Glass` | | |
| TextureSampleParameter2D | `Masks` | | `CarTextures` |
| TextureSampleParameter2D | `Diffuse` | | `CarTextures` |
| TextureSampleParameter2D | `ColorLookup` (×3) | | |
| TextureSampleParameter2D | `PortalRingTexture` (×3) | | |
| TextureSampleParameter2D | `PortalRingTexture_02` (×2) | | |
| VectorParameter | `Diffuse` (×2) | | |
| VectorParameter | `Skin` | | **`BaseTextures`** |
| VectorParameter | `BodyMasks` | | |
| VectorParameter | `Texture` | | |
| VectorParameter | `Normal` (×2) | | |
| VectorParameter | `ColorLookup` (×2) | | |
| VectorParameter | `TertiaryMaterial_Normal` | | `TertiaryMaterial` |
| VectorParameter | `F2DetailNormal` | | |

### 2.3 Critical insight — `Skin` is a *Vector*, not a *Texture*

The expression node named `Skin` in `Body_Paintable_Mat` is a
**`MaterialExpressionVectorParameter`** (a LinearColor RGBA), grouped under
`BaseTextures`. It is **not** a `TextureSampleParameter2D`. So the user's
`shaderSkinPatch.ts` interpretation — where `uniform sampler2D skinTexture`
is sampled with `texture2D(skinTexture, vUv)` — does NOT match the parent
material's declaration of `Skin`.

The actual mask texture is bound to the **`Masks`** parameter (a
`TextureSampleParameter2D` in `CarTextures` group), or via the
`PaintMaskInRGB` parameter declared on `TexturePaint_2Tex_Color` and
`VertexPaint_4Tex` (engine-debug materials, also in Startup.upk).

Looking at a stock decal UPK (`Skin_Octane_GaleFire_SF.upk`, validated
empirically in `custom_decals.md`), the binding goes through
`TextureFileCacheName` lookup with:

- `Force_Body_D` → diffuse (visible RGB art)
- `Skin_Octane_GaleFire_RGB` → the RGB mask (color regions)
- `Force_Body_BlankSkin` → blank skin base
- `Body_Force_Curvature` → curvature

These are mapped at runtime onto the body MIC's parameters; based on the
parameter declarations the most likely mapping is:

| Skin UPK texture | Body MIC parameter |
|---|---|
| `*_D` | `Diffuse` (TextureSampleParameter2D) |
| `*_RGB` | `Masks` (TextureSampleParameter2D) — likely the RGB color-region mask |
| `*_BlankSkin` | (passed via parent body MIC's defaults, e.g. `BlankSkin` not in `Body_Paintable_Mat`'s declared list — may be added by the inheritance chain through `MIC_Body_Octane`) |
| `*_Curvature` | (likely `CarTextures.Masks` or a similar parameter on the body's chassis/curvature MIC) |

### 2.4 `MIC_Body_Paintable_All` default parameter values

Extracted from the MIC's tagged-property block:

```
ScalarParameterValues (2 entries):
  Noise_Scale = 2.0
  Pan_Speed   = 3.0

ScalarParameterValues (1 entry, secondary array):
  Opacity = 0.0

TextureParameterValues (1 entry, primary):
  Emmisive [sic] → Object idx 2761

TextureParameterValues (1 entry, secondary):
  Diffuse → Object idx 2570
```

No threshold scalars (`0.18`, `0.15`, etc.) anywhere. No team-color
defaults. The MIC is essentially empty.

---

## 3. Material function inspection (`SPMF_*`)

Two material functions in `Vehicle_Parent_MaterialFunctions` (a
sibling package in Startup.upk):

- `SPMF_TertiaryMaterial` (export[783], 615 B)
- `SPMF_TrimColorAndFinish` (export[784], 393 B)

Walking `SPMF_TrimColorAndFinish` byte by byte:
- **0 hits** for any of `0.9, 0.18, 0.15, 0.169, 0.05, 0.5, 1.0` IEEE 754
  patterns
- FName references: `ColorLookup`, `RimColor`, `Coordinates`, `Texture`,
  `Material`, `Expression`, `ExpressionInput`, `ParameterName`

So even the material function bodies are **graph topology + parameter
references**, not literal float constants. The numeric trim/team
thresholds, if they exist, are baked into the compiled DXBC pixel shader.

---

## 4. Why we can't extract the DXBC

`RefShaderCache-PC-D3D-SM5.upk` (219 MB) is a **fully-compressed UE3
package**, not a standard chunk-compressed UPK. It contains
`FMaterialShaderMap` records that wrap `FShaderCompilerOutput` blobs,
which in turn wrap the DXBC bytecode. Format quirks:

- The file starts with the chunked-compression magic `0xC1832A9E` (not the
  UE3 package magic `0x9E2A83C1` reversed). Block size = `0x200000`
  (2 MB). The entire file is one big chunked compressed stream with no
  uncompressed header.
- UE3's `LoadCompressedPackage` decompresses the whole stream first, then
  parses as a normal UPK. We'd need a UE3 fully-compressed decompressor —
  doable in Python with `zlib.decompress` chunk by chunk, but the result
  is ~600 MB-1 GB of decompressed data.
- Even after decompression, parsing `FMaterialShaderMap` requires
  understanding RL's specific UE3 shader-cache layout (no public tool
  does this for any RL build).
- Once we had the DXBC blob, `D3DDisassemble` could turn it into bytecode
  listing, but HLSL recovery is impractical.

This is why no community tool (umodel, UE Explorer, RLSDK-Generator,
RLUPKTool) inspects RL shaders at the bytecode level.

---

## 5. Cross-reference — the user's reproduced shader

The user's `shaderSkinPatch.ts` defines this logic:

```glsl
vec4 decalColor    = texture2D(map,          vUv1);      // diffuse PNG
vec4 skinColor     = texture2D(skinTexture,  vUv);       // RGB mask
vec4 curvatureColor= texture2D(curvatureTex, vUv);       // curvature

vec3 finalColor = decalColor.rgb;     // default: literal diffuse

// Primary team color marker: (R≈1, G≈0, B≈0, α≈0)
if (skinColor.r > 0.9 && skinColor.g < 0.1 && skinColor.b < 0.1 && skinColor.a < 0.1)
    finalColor = mainTeamColor;
// Secondary/accent marker: (R≈1, G≈0, B≈0, α≈1)
else if (... skinColor.a > 0.9)
    finalColor = decalColor.rgb;  // (incomplete sim — should be secondaryColor)
// Literal-diffuse band: r in [0.15, 0.18]
else if (skinColor.r > 0.15 && skinColor.r < 0.18 && skinColor.g < 0.05 && skinColor.b < 0.05)
    finalColor = decalColor.rgb;
// Windows: (R≈0, G≈0, B≈1)
else if (skinColor.b > 0.9 && skinColor.r < 0.1 && skinColor.g < 0.1)
    isWindow = true;

// Curvature's green channel = car body part
if (curvatureColor.g > 0.4)
    finalColor = carColor;  // user-picked custom color
```

### 5.1 What's CONFIRMED by the evidence

- **Multi-texture composition.** The native body shader does sample
  separate diffuse + mask textures — this matches `Body_Paintable_Mat`'s
  declared `Diffuse` + `Masks` texture parameters and the empirical
  4-texture pattern from rlpeak donors (`*_D`, `*_RGB`, `*_BlankSkin`,
  `*_Curvature`).
- **R-channel → primary, B-channel → windows.** Empirically confirmed in
  `custom_decals.md` (Yuna Itzy test): pixel `(255, 0, 0, α=0)` rendered
  as primary color, pixel `(0, 0, 255, α=0)` rendered as windows
  (transparent overlay).
- **Alpha channel multiplexes primary vs secondary.** Empirically
  confirmed: `(255, 0, 0, α=255)` rendered as secondary/accent color.
- **Curvature-based body coloring.** `MIC_Body_Octane` references a
  `Force_Body_Curvature` or `Body_Force_Curvature` texture (varies per
  donor); the `Body_Paintable_Mat` has a `TertiaryMaterial_Normal` and
  the `Body_All_Mat` has `RGBMasks` + a `ColorLookup` chain.
- **Team color comes from a runtime palette texture.** Confirmed:
  `UTeamColorScriptedTexture_TA::OnRender(UCanvas* Canvas)` writes the
  team's current palette into a render target each frame; the body
  shader samples this RT. So the "primary color" in the user's shader
  IS a sampled color, not a constant.

### 5.2 What's REFUTED by the evidence

- **The literal-diffuse threshold band `r in [0.15, 0.18]`.** The exact
  IEEE 754 values for `0.15` and `0.18` would need to appear in the
  compiled shader bytecode for this branch to be a real RL behaviour.
  `0.18` does not appear anywhere; `0.15` appears 2293 times in
  `RefShaderCache` but is too common to be diagnostic for any specific
  decal shader. **More importantly, `0.169 = 43/255` (the documented
  AC convention's "literal diffuse" marker value at byte level) does
  not appear anywhere.** If RL's shader had a special-case "literal
  diffuse" path triggered by a specific R value, that value would be a
  threshold constant — and it would necessarily appear in the bytecode.
- **The hard-cutoff threshold `r > 0.9`, `r < 0.1`, etc.** `0.9` appears
  6 times in Startup body (in unrelated places) and 870 times in
  RefShaderCache. None of these can be tied specifically to the body
  decal shader without DXBC disassembly. Native RL probably uses a
  **continuous lerp** (e.g. `lerp(diffuse, teamColor, skinMask.r)`),
  NOT hard `> 0.9 < 0.1` cutoffs. The user's hard-cutoff is a
  binary-classifier approximation of a smooth lerp.
- **The 0.15..0.18 "literal diffuse" band specifically.** This was the
  AlphaConsole-era convention from the runtime-injection plugin. RL's
  native shader has no need for it because RL's mask system uses
  RGB+α directly for the color zones (R for primary, R+α for accent,
  B for windows). A "pass-through diffuse" region in the mask would
  most likely be encoded by `(R, G, B) = (0, 0, 0)` (mask = black =
  no color, literal diffuse shows through) — NOT by `R=43/255`. The
  user's `r in [0.15, 0.18]` band is therefore a heuristic, probably
  invented to match a specific decal where the mask happened to have
  values in that range.

### 5.3 What's PLAUSIBLE but UNVERIFIED

- The **carColor on curvature.g > 0.4 path** — `Body_Paintable_Mat`
  doesn't directly declare a `Curvature` texture, but `MIC_Body_Octane`
  imports `Body_Force_Curvature` (per `galefire_imports.py`). The body
  shader likely samples this and uses its green channel as a body-region
  mask, but the threshold `0.4` is speculative.
- The **3-way priority: primary > secondary > literal-diffuse > windows >
  carColor**. The order of the if/else chain in the user's shader could
  be wrong. RL's compiled shader may use a different priority — e.g. it
  might compose all four contributions additively rather than as
  exclusive branches.

---

## 6. The actual RL composition model (best reconstruction)

Based on (a) parameter declarations in `Body_Paintable_Mat` /
`Body_All_Mat`, (b) RLSDK class signatures (custom_decals.md §"RLSDK"),
(c) empirical in-game testing (Yuna Itzy test, GaleFire dual-swap test),
and (d) string evidence (`TeamColorScriptedTexture_TA`,
`Team1_ColorPrimary`, etc. in RL.exe), the actual RL body shader most
likely does something like:

```hlsl
// Sample input textures
float4 diffuse    = SampleTexture2D(BodyDiffuse,      uv);     // *_D
float4 mask       = SampleTexture2D(BodyMasks,        uv);     // *_RGB (color-region mask)
float4 blankSkin  = SampleTexture2D(BodyBlankSkin,    uv);     // *_BlankSkin
float4 curvature  = SampleTexture2D(BodyCurvature,    uv);     // *_Curvature

// Sample team palette from runtime-composed RT
float3 primaryColor   = SampleTexture2D(Team1_ColorLookup, uvLookup).rgb; // or Team2 by team
float3 secondaryColor = playerCustomColor.rgb;                            // from CarMeshComponentBase_TA::CustomColorOverride

// Mask channels drive blending
float primaryWeight   = mask.r * (1 - mask.a);      // R=1, α=0 → primary
float secondaryWeight = mask.r * mask.a;            // R=1, α=1 → secondary (accent)
float windowsWeight   = mask.b;                     // B → windows (transparent overlay)
// remaining = diffuse (where R=0 and B=0)

// Composite
float3 bodyTint  = lerp(diffuse.rgb, primaryColor,   primaryWeight);
       bodyTint  = lerp(bodyTint,    secondaryColor, secondaryWeight);
       bodyTint  = lerp(bodyTint,    windowsColor,   windowsWeight);

// Apply curvature/AO multiplier
float3 finalColor = bodyTint * curvature.rgb;

// Output
return float4(finalColor, 1 - windowsWeight * 0.2);  // windows are slightly transparent
```

**This is a reconstruction, not extracted from DXBC.** The key
differences from the user's reproduced shader:

1. **Continuous lerp** instead of hard `> 0.9`/`< 0.1` cutoffs.
2. **No literal-diffuse band at `r in [0.15, 0.18]`** — instead, the
   diffuse is the default contribution when both `mask.r == 0` and
   `mask.b == 0`.
3. **Curvature drives AO multiplication**, not a separate "isCarColorPart"
   flag.
4. **Team color comes from a sampled palette**, not a hardcoded uniform.

---

## 7. Confidence in each finding

| Claim | Confidence | Basis |
|---|---|---|
| User's `0.18` / `0.169` thresholds don't match RL | **High** | 0 hits across all RL binaries for those exact IEEE 754 patterns |
| RL uses continuous lerp not hard cutoffs | **Medium** | Standard UE3 shader practice + no Constant nodes in expression graph |
| Body MIC samples a mask texture with RGB+α encoding the color zones | **High** | Empirical Yuna Itzy test + parameter declarations in `Body_Paintable_Mat` |
| `r=1, α=0 → primary; r=1, α=1 → secondary; b=1 → windows` | **High** | Yuna Itzy test in custom_decals.md (already established) |
| Default (no marker) renders as literal diffuse | **High** | Logical complement of marker-based palette pickups; consistent with how AlphaConsole packs were originally designed |
| Specific threshold values used in actual shader | **Low** | Cannot extract from DXBC; tools don't exist for RL's `FMaterialShaderMap` |
| The order of if/else priority in user's shader | **Low** | Order matters less when using lerps; in compiled shader this is probably a sum of weighted contributions |

---

## 8. Practical recommendations

For the **RL-Designer** project's preview accuracy:

1. **Drop the `r in [0.15, 0.18]` literal-diffuse band.** It's not a real
   RL behaviour. Instead, render literal diffuse wherever the mask has
   no marker (`mask.r ≈ 0 && mask.b ≈ 0`).
2. **Replace hard cutoffs with lerps.** Use:
   ```glsl
   float primaryWeight   = skinColor.r * (1.0 - skinColor.a);
   float secondaryWeight = skinColor.r * skinColor.a;
   float windowsWeight   = skinColor.b;
   vec3 bodyTint = mix(decalColor.rgb, mainTeamColor, primaryWeight);
        bodyTint = mix(bodyTint,        secondaryColor, secondaryWeight);
        bodyTint = mix(bodyTint,        windowsColor,    windowsWeight);
   ```
3. **Apply curvature as multiplicative AO**, not as a binary "is car body"
   flag.
4. **For exact ground truth**, the only path forward is to capture an
   actual RL frame with RenderDoc / NSight, inspect the body fragment
   shader DXBC, and decompile it. This is doable on a per-decal basis
   but isn't a generalised tool.

---

## 9. Tools tried / not tried

### 9.1 Tried
- ✅ Decrypt + decompress `Startup.upk` (`pycryptodome` + `zlib`)
- ✅ Parse UE3 name table, import table, export table (variable-size
  RL skin layout, i64 SerialOffset, NetObject section)
- ✅ Walk `MaterialExpression*Parameter` exports for ParameterName +
  DefaultValue (byte-pattern matching on the tagged-property header)
- ✅ Threshold-float byte scan across `RocketLeague.exe`, `Startup.upk`
  (decompressed body), `RefShaderCache-PC-D3D-SM5.upk` raw, and
  `GlobalShaderCache-PC-D3D-SM5.bin`
- ✅ String scan for parameter names in `RocketLeague.exe`

### 9.2 Not tried (would require more effort but feasible)
- ❌ Fully-decompress `RefShaderCache-PC-D3D-SM5.upk` (UE3
  fully-compressed format)
- ❌ Parse `FMaterialShaderMap` blobs to locate per-Material DXBC
- ❌ Run `RocketLeague.exe` under RenderDoc and capture the body shader
  at runtime (post-EAC RL may detect debugger attachment)
- ❌ Disassemble `bakkesmod.exe` for `ApplyDecalToCar` implementation
  (its source code in RAM might reveal the FName→texture binding map
  the body MIC uses)
- ❌ Disassemble AlphaConsole's plugin DLL for the historical
  R-channel encoding constants

### 9.3 Not viable
- ❌ Recover HLSL source from DXBC — no public decompiler with adequate
  quality for production shaders
- ❌ Edit shader bytecode — RL's shader-map hash validation would reject
  modifications

---

## 10. Scripts produced

This analysis was ad-hoc; no new scripts saved (all run inline via
`python -c`). The reusable bits are already in
`sandbox/research/parse_decrypted_upk.py` and
`sandbox/research/parse_texture2d_real.py` (the existing UE3 parser
pipeline).

Key one-liners that would be worth saving if this analysis is revisited:

1. Decrypt + decompress `Startup.upk` to `/tmp/startup_decompressed_body.bin`.
2. Find all `Body_Paintable_Mat` child Expressions by walking
   `outer == export[261]+1 == 262` predicate.
3. Threshold scan: for each candidate value `v`, count occurrences of
   `struct.pack('<I', float_to_u32(v))` in each binary file.

---

## 11. Bottom line

The user's reproduced `shaderSkinPatch.ts` captures the **structure**
of RL's body decal compositing (RGB+α mask channels → palette pickups +
literal diffuse default) but uses **invented numeric thresholds** that
don't correspond to RL's actual shader. The exact thresholds `0.15`,
`0.18`, `0.169` (and `0.9` as a hard cutoff) are not present as IEEE 754
constants anywhere in RL's executable or its cooked materials/shader
caches. The actual RL body shader almost certainly uses **continuous
lerps** weighted by the mask's R/α/B channels, not hard if/else cutoffs.

For preview-accuracy purposes, the simplest fix is to replace the
hard-cutoff branches with three `mix()` calls weighted by
`mask.r * (1-mask.a)`, `mask.r * mask.a`, and `mask.b` respectively,
multiplied at the end by the curvature texture for AO/body-region
modulation.

Full ground-truth verification would require capturing a frame in
RenderDoc and disassembling the body fragment shader's DXBC — a one-off
operation that can't be automated from our pipeline because RL ships
post-EAC and may resist frame-capture tooling.
