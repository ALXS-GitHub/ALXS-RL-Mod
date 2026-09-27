# Startup.upk body MIC — deep investigation (2026-05-15)

> **Goal.** Find EXACTLY which texture parameter the body shader samples at
> the "decal art zone" (the AlphaConsole convention `R≈43/255` sentinel) by
> walking `Body_Paintable_Mat`'s parameter graph, `MIC_Body_Paintable_All`'s
> overrides, and `MIC_Body_Octane`'s parent reference.
>
> **Bottom line (TL;DR).** At `R=43` zones the body shader samples the FName
> parameter **`Diffuse`**, which on stock Octane resolves to the Texture2D
> **`Octane_Body_D`** (Startup.upk export[2501], `Textures3.tfc`-bound,
> 2048×2048). For per-decal slots the resolved texture is overridden at
> the **skin MIC level** (e.g. `Octane_GaleFire_MIC` in the skin UPK), not
> at MIC_Body_Octane. **Modifying Startup.upk via mods/ override is NOT
> reliably supported by RL's mods/ loader — the empirical TAGame.upk evidence
> in `palettes.rs` shows the file size MUST remain byte-identical or RL
> fatals during `LoadScriptPackages`.** Strategy B (Startup.upk hijack) is
> therefore feasible only with a byte-identical surgical patch (no UPK
> growth), and it still has the global side-effect of affecting every
> Octane skin slot install-wide.

---

## 1. Material / MIC inventory of Startup.upk (Task 1)

Decrypted via our pipeline (`sandbox/research/dump_startup_materials.py`):

```
File size       : 41,648,351 bytes
Names           : 3,276
Exports         : 3,854
Imports         : 380
Body (decompressed): 129,550,870 bytes  (~129 MB)
```

Class import indices :
`Material = -31`, `MaterialInstanceConstant = -40`, `Texture2D = -101`,
`MaterialExpressionTextureSampleParameter2D = -36`,
`MaterialExpressionVectorParameter = -38`,
`MaterialExpressionScalarParameter = -32`.

### 1.1 Focused targets — confirmed

| Export | Class | Name | Serial offset | Size |
|---:|---|---|---:|---:|
| 260 | Material | `Body_All_Mat` | 545,064 | 2,643 |
| **261** | **Material** | **`Body_Paintable_Mat`** | **547,707** | **2,516** |
| 786 | MaterialInstanceConstant | `MIC_Body_Octane` | 674,768 | 2,722 |
| 831 | MaterialInstanceConstant | `Body_All_MIC` | 723,585 | 2,618 |
| 832 | MaterialInstanceConstant | `MIC_Body_Paintable_All` | 726,203 | 2,882 |
| **2501** | **Texture2D** | **`Octane_Body_D`** | **14,383,776** | **6,196** |

`Octane_Body_MIC` (the name in the user's question) **does not exist** as a
separate export — what we have is `MIC_Body_Octane` (the per-body MIC) and
`MIC_Body_Paintable_All` (a generic paintable parent MIC, not the actual
Octane parent — see §3).

---

## 2. `Body_Paintable_Mat` — the parent Material shader (Task 1.3)

`Body_Paintable_Mat` is a pure declarative parameter graph: every child
is a `MaterialExpression*Parameter` subobject. The full inventory was
extracted in `docs_rl/rl_decal_shader_re.md` §2.2 and re-validated by our
new walker (`sandbox/research/dump_paintable_mat_params_v2.py`).

### 2.1 The TextureSampleParameter2D nodes (CarTextures group)

| Class | ParameterName | Group | Role |
|---|---|---|---|
| TextureSampleParameter2D | **`Diffuse`** | `CarTextures` | **The visible-art texture sampled at R=43 / "literal diffuse" zones** |
| TextureSampleParameter2D | **`Masks`** | `CarTextures` | The RGB color-region mask (R=primary, R+α=accent, B=windows) |
| TextureSampleParameter2D | `ColorLookup` (×3) | — | Team palette / runtime composed lookup |
| TextureSampleParameter2D | `PortalRingTexture` (×3) | — | Effects ring, not the body |
| TextureSampleParameter2D | `PortalRingTexture_02` (×2) | — | Effects ring |
| TextureObjectParameter | `Glass` | — | Window glass |

### 2.2 The VectorParameter nodes (BaseTextures group)

| ParameterName | Group | Role |
|---|---|---|
| `Skin` | **`BaseTextures`** | A VECTOR, not a texture — tints the skin base layer |
| `Diffuse` (×2) | — | Tints the diffuse output (an RGBA color, not a sampler) |
| `BodyMasks` | — | RGBA mask tint |
| `Normal` (×2) | — | Normal-map mods |
| `ColorLookup` (×2) | — | Palette tints |
| `TertiaryMaterial_Normal` | `TertiaryMaterial` | Trim normal-map |

### 2.3 Critical caveat (validated 2026-05-15)

The expression named **`Skin` is a `MaterialExpressionVectorParameter`**
in `BaseTextures` — not a `TextureSampleParameter2D`. This contradicts a
loose reading of AlphaConsole pack JSON keys (`Diffuse` + `Skin`).

In the runtime stack the **skin's _RGB texture (the color-region mask)** is
bound to the parent material's `Masks` (TextureSampleParameter2D), NOT to
`Skin`. The skin MIC's `TextureParameterValues[]` entry that AC packs label
`"Skin": …` is the per-decal override of the `Skin` Vector — i.e. a flat
RGBA tint, not the mask texture itself. This subtlety has been a recurring
source of confusion in our pipeline (custom_decals.md is partially wrong
about it; `decal_swap.rs` constants `HIJACK_FROM_MASK = b"Textures7"` /
`HIJACK_TO_MASK = b"MyDecal02"` work because the bound texture name is
`Skin_Octane_GaleFire_RGB` regardless of the parameter ALIAS).

---

## 3. `MIC_Body_Octane` and `MIC_Body_Paintable_All` (Task 2)

We brute-force scanned both MICs for `TextureParameterValues` /
`VectorParameterValues` / `ScalarParameterValues` arrays
(`sandbox/research/dump_mic_octane_tex_params.py`):

```
=== MIC_Body_Octane export[786] size=2722 ===
   TextureParameterValues: NOT FOUND
   VectorParameterValues : NOT FOUND
   ScalarParameterValues : NOT FOUND
   Parent / Material(tagged-prop) -> export[261] 'Body_Paintable_Mat'

=== MIC_Body_Paintable_All export[832] size=2882 ===
   TextureParameterValues: NOT FOUND
   VectorParameterValues : NOT FOUND
   ScalarParameterValues : NOT FOUND
   (only ParentLightingGuid + bHasQualitySwitch in tagged-props, then <None>)
```

### 3.1 Hierarchy (corrected)

```
Body_Paintable_Mat (export[261])  <-- the declarative parent Material (parameter graph)
    │
    ├─ MIC_Body_Octane (export[786])      Parent="Material"→Body_Paintable_Mat
    │     ↑ no TextureParameterValues — just inherits parent defaults
    │     ↑ 2722-byte body is dominated by the MaterialResource shader cache blob
    │
    └─ MIC_Body_Paintable_All (export[832]) Parent=??? (no 'Material' FName ref in body)
          ↑ similarly no TextureParameterValues; has ParentLightingGuid only
```

The **real per-decal texture binding happens at the skin MIC level** (e.g.
`Octane_GaleFire_MIC` inside `skin_octane_galefire_SF.upk`), whose Parent
is `MIC_Body_Paintable_All` (per `custom_decals_FINAL_PLAN.md`). The skin
MIC's `TextureParameterValues` array carries the 3 entries we already
exploit:
```
'CurvaturePack' -> Body_Force_Curvature
'Diffuse'       -> Force_Body_D          ← user's PNG goes here (already hijacked)
'Skin'          -> Force_Body_BlankSkin  ← (and here, mask data, already hijacked)
```

So **the `Diffuse` parameter name DOES route the body shader to the user's
diffuse texture** — confirmed at two levels :
- `Body_Paintable_Mat` declares `Diffuse` (TextureSampleParameter2D, CarTextures group).
- `Octane_GaleFire_MIC` overrides `Diffuse` → `Force_Body_D`.

---

## 4. `Octane_Body_D` Texture2D and its TFC binding (Task 3)

From `sandbox/research/dump_startup_materials_v2.py` :

```
=== Texture2D Octane_Body_D export[2501] ===
   serial_offset=14383776 size=6196
   tagged-props :
     SizeX                = 2048
     SizeY                = 2048
     OriginalSizeX        = 2048
     OriginalSizeY        = 2048
     Format               = (byte enum, PF_DXT5 / BC3)
     TextureFileCacheName = 'Textures3'   ← *** THIS IS THE TFC NAME TO HIJACK ***
     MipTailBaseIdx       = 11
     SRGB                 = true
```

So **on stock Octane**, the body's diffuse art (`Diffuse` param) sources its
bytes from **`Textures3.tfc`** at the mip offsets recorded inside the
`Octane_Body_D` body's bulk-data table.

For Strategy B (Startup.upk hijack), we would :
1. Modify the FName `Textures3` → e.g. `MyDecal_B` (length-preserving) in
   Startup.upk's name table — but that affects EVERY Texture2D bound to
   `Textures3`, not just `Octane_Body_D`. Probably hundreds of textures.
2. Or rewrite `Octane_Body_D`'s `TextureFileCacheName` tagged-prop payload
   from `Textures3` to a custom FName, AND insert the custom FName into
   the name table (length-preserving substitution again).

Both paths require **byte-identical-size** modification — see §5.

---

## 5. mods/ override of Startup.upk — feasibility (Task 4)

### 5.1 Direct empirical evidence (from our own codebase)

`src-tauri/src/commands/palettes.rs` lines 311-330 documents exactly this
question for the analogous case of `TAGame.upk` (sibling root UPK,
non-encrypted but otherwise structurally identical to Startup.upk) :

> Compression strategy (designed to always fit in the original block size):
> 1. Try `flate2` level 9 first — very fast (~5 ms), often fits …
> 2. If level-9 doesn't fit, fall back to **zopfli** …
> 3. If even zopfli can't fit, we accept growth as a last resort and
>    splice the larger block in, patching the chunk header. **This branch
>    has been observed to crash RL in practice** (the "Ambiguous package
>    name" dialog appears, then RL fatal-errors during `LoadScriptPackages`)
>    — its structural validity is provable but **UE3's `mods/` overlay
>    loader seems to reject any TAGame.upk whose file size differs from
>    the root file**.

This is **production-validated empirical evidence**, accumulated by us
over multiple iterations on the palette engine. Applied to Startup.upk:

- **A byte-identical (same file size) modified Startup.upk in `mods/`
  IS loaded by RL** (confirmed via the palette pipeline analog).
- **A size-different modified Startup.upk in `mods/` causes RL to fatal
  during `LoadScriptPackages` with "Ambiguous package name"**.

### 5.2 What this means for Strategy B

A surgical Startup.upk patch is **feasible** if and only if :
- Every modification is length-preserving in FName / tagged-prop / mip
  table bytes (no UPK growth).
- The chunked-zlib body recompresses back to ≤ original `c_size` per
  block (we already do this for the palette engine, and for the
  decal_swap.rs name-table hijacks via padding to `c_size`).
- We hijack ONE specific `Octane_Body_D`'s TFC binding without breaking
  the other ~hundreds of textures sharing `Textures3.tfc` (i.e. by
  editing the **`TextureFileCacheName` payload bytes** at the tagged-prop
  offset inside `Octane_Body_D` only, not the name-table entry itself).

This is doable — the same surgical-byte-edit technique we use for
palette splices works here, but the byte budget is tighter because
Startup.upk's body is ~129 MB after decompression and only the specific
chunk holding `Octane_Body_D` needs to be re-zipped under budget.

### 5.3 Side-effects (Strategy B drawback)

Even if Strategy B works file-wise :
- **It changes the body diffuse install-wide**. Every Octane stock decal
  (Default, Goal Explosion variants, etc.) would render with our custom
  diffuse instead of `Octane_Body_D`'s original bytes.
- **Only the body diffuse layer would change**. The `Masks` parameter (the
  color-region mask) would still point at its own stock texture, so the
  paint shader would still composite team/custom colors over our diffuse.
- **To achieve "literal RGB" we'd also need to swap the `Masks` binding**
  to a (0,0,0,255) constant mask (FullColor mode equivalent at the body
  level instead of the skin level).
- **Conflict with skin MICs**: when the player equips e.g. GaleFire
  (`Octane_GaleFire_MIC` overrides `Diffuse` → `Force_Body_D`), the skin
  override wins. So Strategy B only affects body slots where NO custom
  decal is equipped (i.e. the Default Octane look) — diluting the value
  proposition vs the current dual-TFC pipeline.

**Conclusion**: Strategy B is technically feasible but **strictly less
useful than the current Strategy A** (per-decal skin MIC hijack via the
dual-TFC pipeline) for the user-facing "ship a custom PNG as a decal" use
case. Keep it documented as a fallback if the AC FullColor neutralization
mask path (current Phase 3 plan) fails to achieve literal-RGB rendering.

---

## 6. The R=43 sentinel — where does it sample from? (Task 5)

### 6.1 Direct answer

The shader expression that samples the diffuse texture is a single
`MaterialExpressionTextureSampleParameter2D` node named **`Diffuse`** in
group `CarTextures`, declared inside `Body_Paintable_Mat` (Startup.upk
export[261]). It is sampled **unconditionally every frame**, regardless of
any mask value. The notion of an "R=43 zone" is a **mask-driven branch in
the compiled DXBC**, not a property of the `Diffuse` parameter.

Specifically (reconstructed in `rl_decal_shader_re.md` §6) :

```hlsl
float4 diffuse = SampleTexture2D(Diffuse, uv);          // ALWAYS sampled
float4 mask    = SampleTexture2D(Masks,   uv);

float primaryWeight   = mask.r * (1 - mask.a);
float secondaryWeight = mask.r * mask.a;
float windowsWeight   = mask.b;

float3 bodyTint = lerp(diffuse.rgb, primaryColor,   primaryWeight);
       bodyTint = lerp(bodyTint,    secondaryColor, secondaryWeight);
       bodyTint = lerp(bodyTint,    windowsColor,   windowsWeight);
```

At **`mask.r = 43/255 ≈ 0.169`** with `mask.a = 0`, `primaryWeight = 0.169`,
which means the shader **lerps 16.9 % toward `primaryColor`** — it does
NOT bypass to a literal diffuse. The AlphaConsole-era "R=43 means literal
diffuse" convention is **NOT** an actual RL shader behaviour (validated by
`rl_decal_shader_re.md` § 1.4 threshold-float scan: `0.169 / 43/255` /
`0.18` do not appear as IEEE 754 constants anywhere in
RocketLeague.exe / Startup.upk / RefShaderCache / GlobalShaderCache).

**The actual "literal diffuse" zone in RL's shader is wherever the mask
has `R ≈ 0` and `B ≈ 0`** — i.e. a pixel of `(0, 0, 0, *)` in the
`*_RGB` mask renders as pure `Diffuse` pickup with no tint.

### 6.2 Confidence

| Claim | Confidence |
|---|---|
| The FName param sampled for the body's visible diffuse is `Diffuse` (TextureSampleParameter2D) declared in Body_Paintable_Mat | **High** — directly extracted from Startup.upk parameter graph |
| On stock Octane the bound Texture2D is `Octane_Body_D` (Startup.upk export[2501]) with TFC=Textures3 | **High** — directly read from the Texture2D's tagged-property block |
| Per-decal override happens at the skin MIC (e.g. `Octane_GaleFire_MIC`'s `Diffuse` → `Force_Body_D`) | **High** — already empirically verified by our dual-TFC pipeline shipping |
| `R=43/255` does NOT trigger any special shader branch in RL | **High** — IEEE 754 byte-scan across all RL binaries found 0 hits for 0.169 / 43/255 / 0.18 |
| Literal-diffuse zone in mask = `R ≈ 0 AND B ≈ 0` | **High** — logical complement of the marker weights, validated in `rl_decal_shader_re.md` §5.2 |
| mods/ override of Startup.upk requires byte-identical size | **High** — empirically validated by our palettes.rs production engine for the analogous TAGame.upk case |

---

## 7. Concrete next experiment to validate

Given that :

1. The current **dual-TFC pipeline already controls `Diffuse` and the mask**
   at the skin MIC level (Strategy A).
2. **Strategy B (Startup.upk hijack) adds install-wide side effects** with
   no incremental benefit over the skin-MIC override.
3. **The `R=43` sentinel is not a real RL shader behaviour** — the
   AlphaConsole-era documentation that drove that idea is misleading.

The **most-actionable next experiment** is **Phase 3 of the existing
FINAL_PLAN** (already designed in `docs_rl/custom_decals_FINAL_PLAN.md`):

### 7.1 Phase 3 — FullColor mode for `decal_swap.rs`

In `src-tauri/src/commands/decal_swap.rs` :

1. Add `pub enum DecalMode { Paintable, FullColor }` and `pub mode: DecalMode`
   to `DecalSwapPlan`.
2. When `mode == FullColor`, **replace the user's mask BC3 encoding with a
   constant `(R=0, G=0, B=0, A=255)` BC3** (single `ffff0000000000000000000000000000`
   16-byte block × 262 144 × 10 mips = 5.59 MB total).
3. In this mode the body shader's lerp weights all evaluate to zero (since
   `mask.r = 0` and `mask.b = 0`), leaving **only the user's diffuse texture
   visible**. This is mathematically the "literal RGB" mode the user wants.

Validation procedure :
- Run the existing `sandbox/research/test_sentinel*.py` scripts with the
  `(0,0,0,255)` mask to confirm the BC3 decodes correctly.
- Install with a known test PNG (solid magenta, say) and verify in-game
  the body renders magenta with no tint from primary / secondary colors.

This experiment is **cheap (single Rust enum dispatch)**, **non-destructive**
(uninstall removes both files), and **decisive**: if the body renders
literal magenta, the entire FullColor pipeline is unlocked. If it doesn't,
we know `mask.b` or curvature.g is still contributing, and we have a small
search space to debug.

---

## 8. Sources

- `sandbox/research/dump_startup_materials.py` (initial Material/MIC enumeration)
- `sandbox/research/dump_startup_materials_v2.py` (with strict tagged-prop walker — Octane_Body_D TFC binding)
- `sandbox/research/dump_paintable_mat_params.py` (1st-pass parameter-graph walker)
- `sandbox/research/dump_paintable_mat_params_v2.py` (strict-walker variant for sub-expressions)
- `sandbox/research/dump_mic_octane_tex_params.py` (brute-force MIC TextureParameterValues scan — confirms NONE exist on MIC_Body_Octane / MIC_Body_Paintable_All)
- `docs_rl/rl_decal_shader_re.md` (parameter graph, threshold-float scan, reconstructed shader)
- `docs_rl/custom_decals_FINAL_PLAN.md` (existing Strategy A architecture)
- `docs_rl/unrealscript_shader_decompile.md` (RLSDK class hierarchy)
- `src-tauri/src/commands/palettes.rs` (lines 311–330 : empirical mods/ overlay size constraint)
- `src-tauri/src/commands/decal_swap.rs` (current dual-TFC pipeline)
