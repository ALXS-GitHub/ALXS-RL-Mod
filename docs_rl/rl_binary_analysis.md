# Static analysis of `RocketLeague.exe` — engine-level levers for file-only custom decals

**Date:** 2026-05-15
**Target:** `C:\Program Files\Epic Games\rocketleague\Binaries\Win64\RocketLeague.exe`
**Size:** 38.78 MB · **Image base:** `0x140000000` · **PDB:** `D:\build\Inc\Sync\Binaries\win64\TAGame-Win64-Shipping.pdb`
**Goal:** find any static mechanism (FName, sentinel, flag, slot ID, path) the engine reads from a cooked `.upk` that lets us bypass the R-channel paint-shader for custom decals **without runtime injection**.

Scripts used:
- `sandbox/research/rl_strings.py` — strings extraction + category bins
- `sandbox/research/rl_pe_introspect.py` — PE section/import/export dump
- `sandbox/research/rl_focus_grep.py` — focused grep on high-signal needles

All raw outputs live under `sandbox/research/rl_strings/`.

---

## 1. PE structure

| Section | VA | VSize | Notes |
| --- | --- | --- | --- |
| `.text`  | 0x1000     | 27.1 MB | code |
| `.rdata` | 0x19E0000  | 8.3 MB  | RO data — almost all our strings live here |
| `.data`  | 0x21CA000  | 2.8 MB  | RW globals |
| `.pdata` | 0x2476000  | 1.3 MB  | unwind |
| `_RDATA` | 0x25C5000  | 78 KB   | MSVC C++ RTTI |
| `.rsrc`  | 0x25D9000  | 124 KB  | version info |
| `.reloc` | 0x25F8000  | 712 KB  | base relocations |

**Imports (30 DLLs):** WSOCK32, dbghelp, WINMM, KERNEL32, USER32, SHELL32, ole32, ADVAPI32, GDI32, OLEAUT32, MSVCP140, CRYPT32, IPHLPAPI, IMM32, faultrep, bcrypt, VCRUNTIME140, the CRT api-ms-win-crt-* shims, WS2_32. **No D3D / no Knowhere.** The renderer is statically linked.

**Exports (396 symbols):** every single one is from **AK** (Wwise audio) or **Scaleform** (UI). E.g. `?AddDefaultListener@SoundEngine@AK@@…`, `??0System@Scaleform@@…`. **Zero engine symbols are exported** — no dlsym-style attack surface.

PDB name confirms the original binary is `TAGame-Win64-Shipping`. The `RocketLeague.exe` we ship is a thin renamed wrapper over the TAGame module.

---

## 2. String corpus summary

| Category | Count | Top file |
| --- | ---: | --- |
| `_all_ascii.txt`            | 36 091 | corpus |
| `_all_utf16.txt`            | 28 131 | corpus |
| `material_params`           | 12 237 | `rl_strings/material_params.txt` |
| `decal_skin_cosmetic`       | 3 875  | `rl_strings/decal_skin_cosmetic.txt` |
| `unreal_console_cmds`       | 3 086  | (mostly false-positives — UnrealScript exec names) |
| `shader_compile`            | 1 468  | `rl_strings/shader_compile.txt` |
| `material_classes`          | 928    | `rl_strings/material_classes.txt` |
| `asset_paths`               | 243    | `rl_strings/asset_paths.txt` |
| `paint_team`                | 163    | `rl_strings/paint_team.txt` |
| `override_flags`            | 50     | `rl_strings/override_flags.txt` |
| `tfc_bulkdata`              | 12     | `rl_strings/tfc_bulkdata.txt` |

The `material_params` category over-collects (the regex `^[A-Z][A-Za-z]+(_[DSNMR])?$` is too permissive). The signal-dense findings are in the focused-grep results under `rl_strings/_focus/`.

---

## 3. Findings ranked by exploit potential

### 3.1 `DiffuseOverrideParameter` / `SpecularOverrideParameter` (UE3 debug viewmodes) — **HIGH but gated**

Found in `.rdata` as plain UTF-16LE strings, gated by `bAllowDebugViewmodesOnConsoles`. UE3 ships a globally-bound `FDiffuseOverrideParameter` / `FSpecularOverrideParameter` that **every pixel shader reads** to additively/multiplicatively force diffuse output (used by editor's "lighting only" / "diffuse only" viewmodes). The strings are present in shipping RL.

- **Mechanism:** when viewmode != VMI_Lit, a global uniform is bound at draw time that overrides ANY material's diffuse output with a fixed colour, ignoring the per-material MIC entirely.
- **Why it matters:** if we can enter this viewmode (e.g. via a console command from a `Default*.ini` exec list, or a Kismet sequence), the per-product paint shader is bypassed engine-wide.
- **Gate:** `bAllowDebugViewmodesOnConsoles` and the explicit guard `Debug viewmodes not allowed on consoles by default. See AllowDebugViewmodes().` — Shipping builds nominally disable it, but **the flag is a CVar set in `BaseEngine.ini`**. If RL reads `[Engine.Engine] bAllowDebugViewmodesOnConsoles=True` from a *user-writable* ini, we have a static unlock.
- **Limit:** even if unlocked, viewmode is **global** (every material → uniform colour). Not selective per-decal. Useless for custom decals, useful only for an "unlit override" debug mode. **REJECT for our purpose.**

### 3.2 `ABall_TA::OverrideBallTexture()` — confirmed static API, but ball-only

UnrealScript exec function: `ABall_TAexecOverrideBallTexture`. Error strings reveal flow:
- `ABall_TA::OverrideBallTexture() Failed to find package at %s`
- `ABall_TA::OverrideBallTexture() Failed to create MIC`
- `ABall_TA::HandleOverrideTexture() Failed to find texture at %s%s`

The ball **already has a built-in file-only texture-override mechanism** that takes a package path + texture path, loads the package via UE3's standard package loader, creates a fresh MIC and swaps it in. We could call this from UnrealScript at runtime — but it's body-specific (`Ball_TA` only). **No equivalent `OverrideBodyTexture` / `OverrideDecalTexture` for cars exists** in the symbol table.

### 3.3 `UOverrideMaterialsHitHandler_TA` — engine-level material override per mesh-index

Exec: `UOverrideMaterialsHitHandler_TAexecCacheAndOverrideMaterialsForCarMesh`. Error strings:
- `[OverrideMaterialsHitHandler_TA] Chassis material index (%d) is invalid for mesh %s`
- `[OverrideMaterialsHitHandler_TA] Skin material index (%d) is invalid for mesh %s`
- `[OverrideMaterialsHitHandler_TA] Unhandled override material mode (%d)`
- `[UOverrideMaterialsHitHandler_TA] Failed to cache materials for car mesh component`

**Game-shipped, per-mesh, per-index material override**, identified by class `Chassis` and `Skin` slot indices. Called automatically when the car takes a hit (demo / dent skin). **This is not a static loader** — it's a runtime UnrealScript path — but the fact that the engine has a *built-in selective material-slot-replacement* function for cars proves the mechanism exists at the script layer.

We can't call this from a `.upk` alone. But it would be the **textbook injection target** for a re-implementation that doesn't trip EAC (because the trigger surface is script-driven by gameplay events, not Win32 injection).

### 3.4 `UCanvasTexture_X` / `UScriptedTexture` / `UTeamColorScriptedTexture_TA` — **promising file-only avenue**

Found as live UnrealScript classes:
- `UScriptedTexture`
- `UCanvasTexture_X` + `UCanvasTextureComponent_X`
- `UTeamColorScriptedTexture_TA` with exec `RenderColorArray`
- Render-thread command: `RenderColorArrayToScriptedTextureCommand`

`UScriptedTexture` is a UE3 class whose contents are filled by a `Canvas::DrawTile / DrawString / DrawTexture` script during `OnRender`. **Crucially, ScriptedTextures are referenced like normal `UTexture` objects from materials** — meaning a body MIC could legally reference a `UScriptedTexture` in its `Diffuse` slot.

**Static exploit idea:** in a hijacked car-body `.upk`, replace the `Diffuse` texture parameter target with a `UScriptedTexture` (or `UCanvasTexture_X`) whose `Render()` function (defined in *another* package we hijack or in the same UPK via embedded UnrealScript bytecode) draws a user-provided PNG. Whether shipping RL still cooks ScriptedTexture script bytecode is the open question — needs verification by parsing a stock body UPK.

### 3.5 `UProductAttribute_Painted_TA` + `UProductAttribute_PaintSettings_TA` — **the paint-mask gate**

The engine has a dedicated **`Painted` attribute class** that is attached to a product if and only if it accepts paint. Live exec functions include:
- `UProductDatabase_TAexecAllProductsBySlot`
- `UProductAsset_TAexecGetAssetPackagePath`
- `UProductAttribute_PaintSettings_TA`

**Hypothesis to test:** if a product is **not flagged with `UProductAttribute_Painted_TA`**, the engine may **skip the paint-shader pass** entirely and render only the raw diffuse. Customisation pipeline:
1. In our hijack body UPK, **remove the `Painted` attribute** from the product asset.
2. The engine then renders the body without applying the alpha-mask paint shader.
3. Our user PNG (which we already swap into the Stars slot) shows through as opaque RGBA.

**This is the most exploitable static lever found.** It needs experimental confirmation but is single-flag, file-only, and EAC-invisible (it's a property on a cooked object, not an injected DLL).

### 3.6 `UObjectProvider::Inject` / `InjectDelayed` — internal script-level injection

`UObjectProviderexecInject` and `UObjectProviderexecInjectDelayed` are LIVE exec functions. RL ships its own *script-side* "injection" mechanism — totally distinct from Win32 DLL injection. It lets UnrealScript register objects into a global lookup that other systems consume. EAC almost certainly doesn't track this (it's normal script).

Not directly exploitable from a `.upk` alone (you need a script that calls `Inject(...)`), but it's a **fascinating attack surface** for the day we cook custom UnrealScript bytecode into a hijack `.upk`.

### 3.7 `SeekFreePCPaths` / `SeekFreePCExtensions` + `DIR_ADD` config

```
Specifies whether to load from 'DIR_ADD' directories (0=no,1=yes,2=mandatory)
SeekFreePCPaths
SeekFreePCExtensions
NOSEEKFREELOADING
```

`SeekFreePCPaths` is the ini-driven array (`[Core.System]`) telling UE3 *where to look for cooked packages*. `DIR_ADD` is the per-entry "this directory is additional" marker. **If we can write to `BaseEngine.ini` or `RocketLeague.ini`**, we can add an arbitrary load directory — exactly how AlphaConsole's "mods/" subfolder works.

Already known and used by `rlpeak`/AlphaConsole community. Not novel, but **confirmed at the binary level**.

### 3.8 `Content\Packages\ProductAssets` — hardcoded package root

UTF-16LE literal in `.rdata`. The product loader resolves asset paths relative to this root. We **could not** find an "alt root" or fallback path scanned at startup — RL appears to hard-pin to this. Worth grepping the section for nearby string-table siblings.

### 3.9 Texture parameter API surface (the "gold list")

All `UMaterial[Instance][Constant|TimeVarying]` parameter accessor exec functions are present. The full Get/Set vocabulary the engine recognises:

```
Get/SetScalarParameterValue    Get/SetVectorParameterValue
Get/SetLinearColorParameter    Get/SetTextureParameterValue
Get/SetFontParameterValue      Get/SetMobile{Scalar,Vector,Texture}ParameterValue
SetActorParameter              SetFloatParameter (DecalComponent)
SetNameParameter               SetVectorParameter
CheckForVectorParameterConflicts (MIC time-varying)
```

These confirm RL's MICs are stock UE3 — no proprietary param classes. The names you bind in a `.upk` `TextureParameterValues` array must be **NAMEs (FName)** present in the parent material's parameter list. The engine itself does not embed a hardcoded list of "valid" decal/paint param FNames — they come from the parent `Material`'s `ExpressionTextureParameter` objects. **There is no "engine-aware" magic decal slot name.**

### 3.10 `UProductEquipProfileSlot_Custom_TA` — "Custom" slot class hint

This class name is suggestive of a slot intended for user-provided custom content, but a single-string presence isn't enough to prove a code path. Would need PDB or IDA to follow.

---

## 4. Confirmed material parameter vocabulary

Bare parameter-like FNames that appear in `.rdata` (high-confidence engine-recognised):

```
Diffuse                     Normal              Specular
DiffuseTexture              NormalTexture       FogFactorTexture
DiffuseOverrideParameter    SpecularOverrideParameter
DiffuseGBufferTexture       AOHistoryTexture
RimShader_Color             RimShader_InterpolationDuration   bOverrideRimShaderColor
DetailNormal                DetailHeightScale
BumpOffset                  ColorTex            DepthTex
CurrentMaskTexture          EdgeMaskTexture     AlphaSampleTexture
BlendTexture                FluidDetailNormalTexture
ColorGradingLUT             ColorPalette_X      ColorBias       ColorScale  ColorWeights
TeamColorScriptedTexture_TA pTeamColor          HighContrastLocalTeamColors
PaintFinish_*               (many per finish: MudBug, Scatter, RonB, RonB_Custom, Waves, lawn, toothpicks)
Body_Giftbox_Mystery_*      Body_MuscleCar_Online    Skin_PaintedShut
```

**Not found** (we were hoping for, but they're absent → don't waste time trying these as parameter names):

- `PaintMask`, `DecalRGBA`, `BodyDecal`, `DecalMask`, `DecalDiffuse`, `DecalSlot`
- `ForcedColor`, `ForceColor`, `OverridePaint`, `PaintOverride`
- `PrimaryColor`, `AccentColor`, `TertiaryColor` (as bare names)

The paint colours are clearly passed via **vector parameters** (no fixed FName seen). The decal "stickers" texture is referenced by **whatever FName the parent material defines** — confirmed via stock body UPK inspection, not a hardcoded engine list.

---

## 5. "Kill switch" flags inventory

50 `b*` flags found. The decal-relevant ones:

```
bAllowDebugViewmodesOnConsoles       (gates DiffuseOverrideParameter — see 3.1)
bAllowHighQualityMaterials           (would force fallback shader — too coarse)
bForceCPUSkinning                    (perf, not visual)
bForceDefaultPostProcessChain        (visual, but post-process not pre-shade)
bForceNoMovies                       (irrelevant)
bForceNoPrecomputedLighting          (irrelevant)
bForceRealTimeDecompression          (TFC streaming — see 5.1)
bIgnoreInstanceForTextureStreaming   (we use this de facto when we hijack TFC)
bOverrideLightMapRes                 (irrelevant)
bOverrideRimShaderColor              (single Rim-shader uniform; not decal)
bUseTextureStreaming                 (off would force full-mip loads → useful for our TFC swap)
bUseTranslucentArenaShaders          (translucent shaders, not opaque decals)
```

**`bForceRealTimeDecompression` is worth a deeper look.** If set, it forces realtime decompression of compressed bulk-data textures rather than streaming from TFC. We're already swapping TFC contents directly, so this is a no-op for our pipeline.

`bUseTextureStreaming=False` (in `BaseEngine.ini`'s `[TextureStreaming]`) is a **clean static lever** — it disables the streaming path and forces full-mip resident textures. This makes our TFC-hijack more deterministic (no LOD bias screwing the mip we shipped). Worth recommending as a setup tweak.

### 5.1 No "no-paint" sentinel flag found

We hoped for `bSkipPaintPass`, `bUnpainted`, `bRawDiffuse`, or similar. **None of these exist** in the binary. So there is no single boolean flag we can flip on a cooked product to bypass paint.

The path forward is **§3.5 — strip the `UProductAttribute_Painted_TA` attribute** from the product asset. The attribute's existence acts as the de-facto gate.

---

## 6. Internal API paths exploitable via file-only mechanisms

| Surface | File-only? | Notes |
| --- | --- | --- |
| `Painted` product attribute (3.5) | ✅ | Edit cooked product UPK to drop the attribute |
| `SeekFreePCPaths` ini path (3.7)  | ✅ | Already used (mods/ trick) |
| `UScriptedTexture` in MIC slot (3.4) | ❓ | Requires writing UnrealScript bytecode into a cooked UPK; needs prototype |
| `DiffuseOverrideParameter` viewmode (3.1) | ⚠ | Global; not selective per-decal |
| `OverrideBallTexture` (3.2) | ✅ | Ball-only |
| `OverrideMaterialsHitHandler_TA` (3.3) | ❌ | Script-runtime only |
| `UObjectProvider::Inject` (3.6) | ❌ | Script-runtime only |
| Cooked TFC swap (existing) | ✅ | Already implemented |
| UPK FName rename / rlpeak rename trick | ✅ | Already implemented |

The most novel finding for file-only exploitation is **§3.5** — the `UProductAttribute_Painted_TA` gate.

---

## 7. Recommended next experiments

1. **Build a stock body product UPK in the editor (or extract one) and identify the `Painted` attribute object inside.** It's stored as a UAttribute subobject on the product. Edit it out, re-cook, observe whether the paint shader pass is skipped.
2. **Verify `UScriptedTexture` is cooked-loadable in shipping RL.** Try referencing one from a hijacked body MIC's `Diffuse` slot and observe behaviour (most likely: load works, but `Render()` script bytecode is stripped from shipping cook → texture is black; in which case the attack dies).
3. **Test `[Engine.Engine] bAllowDebugViewmodesOnConsoles=True` in `BaseEngine.ini`** and binding the `VIEWMODE` console command. If accepted, we have a debug-render path; if rejected via signed-ini check, this avenue is dead.
4. **Grep cooked `Engine.upk` for `bUseTextureStreaming` default value** — if RL flips it to false at runtime, our TFC hijack is even more robust.
5. **Re-run this analysis on `Knowhere.dll` once present** — currently absent from the binary tree. (Knowhere appears to be a private RL DLL that some community tools reference; in this install it is not deployed.)

---

## 8. Appendix — raw output index

All under `sandbox/research/rl_strings/`:

- `_all_ascii.txt` (36k strings), `_all_utf16.txt` (28k strings)
- per-category bins: `material_params.txt`, `decal_skin_cosmetic.txt`, `paint_team.txt`, `override_flags.txt`, `material_classes.txt`, `tfc_bulkdata.txt`, `asset_paths.txt`, `shader_compile.txt`, `config_keys.txt`, `unreal_console_cmds.txt`
- focused-grep results: `_focus/*.txt` (45 needle files)
- PE info JSON: `_pe_introspect.json`
