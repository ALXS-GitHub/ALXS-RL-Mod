# UnrealScript & Shader Decompilation Research — Rocket League

> **Scope.** Investigate how to decompile and analyse RL's UnrealScript classes and
> compiled shaders to understand the decal/material loading pipeline at the engine
> level. Goal: identify a static target (no-injection, file-only) that lets us
> ship paintable custom decals on slots the user already owns, ideally bypassing
> the paint shader's R+alpha mask gating in favour of a literal diffuse passthrough.
>
> **Status.** Investigation complete 2026-05-15. All source references below are
> local clones in `sandbox/research/sdks/{RLSDK,BakkesModSDK,ReplayManipulatorOpenSource}`
> (cloned `--depth 1` from `smallest-cock/RLSDK`, `bakkesmodorg/BakkesModSDK`,
> `Martinii89/ReplayManipulatorOpenSource`).

---

## 1. Tools surveyed for UnrealScript decompilation

### 1.1 UE Explorer / Unreal-Library (Eliot VU)

- Repos: `EliotVU/Unreal-Library` (the parser library, .NET), `EliotVU/UE-Explorer`
  (the GUI front-end).
- Supports UE1, UE2, UE2.5, UE3 (~80 UE3 titles incl. Mass Effect, Gears, Borderlands).
- Workflow (vanilla UE3): `UnrealLoader.LoadPackage()` → `InitializePackage()` →
  iterate exports → decompile UScript bytecode into source.
- **Rocket League specifically.** The library README references RL as version
  `867/009` and notes that RL packages "require decryption" via `AltimorTASDK/RLUPKTool`
  before UELib can parse them. The library does **not** ship RL's AES keys —
  the user must decrypt the UPK first, then point UELib at the decrypted file.
- The encrypted header region (`[NameOffset, TotalHeaderSize - GarbageSize)`,
  rounded up to 16 B) is already handled by our own pipeline (`src-tauri/src/commands/decal_swap.rs`
  uses the same AES-256-ECB + `keys.txt` mapping as `RLUPKTool.exe`).
  So the path is : decrypt with our existing code → write the decrypted UPK to a
  temp location → open with UE Explorer.
- **Caveat for shaders.** UE Explorer decompiles UScript bytecode, not the
  compiled HLSL shader bytecode (`FMaterialShaderMap`). Shaders are stored as
  blobs inside `UMaterial` exports and there is no public tool that disassembles
  them back to HLSL.

### 1.2 RLUPKTool / our pipeline

We already have full read access to RL's decrypted UPKs (see `custom_decals.md`
Breakthrough 2026-05-14). The blocker isn't reading the files anymore — it's
understanding the **shader behaviour** they reference.

### 1.3 Has RL's UScript been published?

A GitHub code search for `"TAGame.uc"`, `"Cosmetic_Skin_TA"` UScript files,
or any literal `.uc` text from RL returns **zero hits**. Psyonix has never
released the UScript source, and no community repo of decompiled UScript
sources exists. The closest substitute is the **RLSDK** (generated C++ header
reflection of the UScript classes; see §2).

---

## 2. RLSDK (the C++ SDK)

- Working repo: `smallest-cock/RLSDK` (most actively maintained fork, generated
  with `RLSDK-Generator`).
- Layout : `RLSDK/SDK_HEADERS/{Core,Engine,TAGame,ProjectX,GFxUI,IpDrv,...}_{classes,structs,parameters}.hpp`.
  Eight packages. `TAGame_classes.hpp` is **76 874 lines** — the entire UScript
  class reflection for RL's gameplay package.
- For each UScript class, RLSDK emits :
  - A C++ class with the offset-correct member layout (fields tagged with
    `(CPF_Edit)`, `(CPF_Transient)`, etc. matching the UScript metadata).
  - The list of `UFunction` callable methods (no body — these are runtime
    `ProcessEvent` thunks that call the native UScript implementation).

This is the **canonical reference** for RL's class hierarchy. It tells us
everything except the function bodies (those live in the encrypted UPKs and
the game DLL).

### 2.1 The five most important UScript classes for our decal problem

1. **`UProductAsset_Skin_TA`** (TAGame_classes.hpp:11740) — the data class
   representing one decal asset. Fields :
   - `UMaterialInterface* Skin` — the actual MIC for the decal (the body
     material).
   - `TArray<FSkinBodySettings> BodySettings` — per-body texture/scalar/vector
     parameter overrides. **This is where the FName → texture map lives.**
   - `bTeamFinishDisabled : 1`, `bCustomFinishDisabled : 1` — boolean flags
     that disable the paint-finish picker per axis. **Candidate kill switches.**
   - `FLinearColor ForcedTeamColors[2]`, `FLinearColor ForcedCustomColor[2]` —
     fixed color values that, when present, replace the player's chosen colors.
     **These are the "ForcedColor" properties we already noticed in donor UPK
     name tables.**
   - Methods : `HasForcedCustomColor(TeamIndex)`, `HasForcedTeamColor(TeamIndex)`,
     `HasForcedCustomFinish()`, `HasForcedTeamFinish()`, `GetSkinBodySettings(Body)`,
     `GetSkinParameters(Body)`, `AttemptApplyChassisOverride`.

2. **`UProductAsset_Body_TA`** (TAGame_classes.hpp:11422) — represents one car
   body :
   - `int32_t SkinMaterialIndex` — which material slot on the mesh is the
     decal/skin.
   - `int32_t BrakelightMaterialIndex`, `ChassisMaterialIndex`, `BoostMaterialIndex`.
   - `FMaterialParams SkinParameters` — body-side default texture parameters.
   - `FLinearColor ForcedTeamColors[2]`, `ForcedCustomColor` — body-level
     forced colors (overrides what the player picks **regardless of decal**).

3. **`UCarMeshComponentBase_TA`** (TAGame_classes.hpp:15533) — runtime
   component on every car ; **this is the renderer of the decal**. Fields:
   - `UProductAsset_Body_TA* BodyAsset`, `UProductAsset_Skin_TA* SkinAsset`.
   - `FLinearColor TeamColorOverride`, `CustomColorOverride`.
   - `UProductAsset_PaintFinish_TA* TeamFinish, CustomFinish` — the paint
     finishes equipped per team / custom axis.
   - Methods (the actual material-update pipeline) :
     - `void ApplyPaintSettings()` — the main entry point.
     - `void SetMaterialColorParams(UMaterialInstanceConstant* MatInst, FLinearColor PaintColor, FName ColorParam, FName FullColorParam)`.
     - `void SetMaterialColors(UMaterialInstanceConstant* MatInst)` — pushes
       the player's team+custom colors into the MIC's vector parameters.
     - `void SetPaintFinishParameters(UMeshComponent* Mesh)`, `SetPaintFinishParametersToAccent(...)`.
     - `void SetMaterialParameters(int32_t ElementIdx, FMaterialParams& outParams)` —
       writes a whole `FMaterialParams` (texture+vector+scalar) onto material
       slot `ElementIdx` of the body's mesh. **This is the static-equivalent
       of AlphaConsole's runtime texture override.**
     - `void InitMaterials()`, `void OnPaintChanged()`.

4. **`UMaterialInstanceConstant`** (Engine_classes.hpp:23458) — UE3 standard
   class for runtime-parameterised materials. The "body main MIC" that
   AlphaConsole's runtime hook overrides is an instance of this class.
   Methods :
   - `SetTextureParameterValue(FName ParameterName, UTexture* Value)` — the
     runtime API AlphaConsole/BakkesMod hook.
   - `SetVectorParameterValue(FName, FLinearColor)`, `SetScalarParameterValue(FName, float)`.
   - The MIC's parent (`MaterialInterface::Parent`) points to the
     `UMaterial` whose compiled shader actually does the blend.
   - **What we can do statically** : set the `TextureParameterValues`
     `TArray<FTextureParameterValue>` directly inside the UPK at the
     `MIC`'s export bytes. That's the equivalent of the runtime call.

5. **`UTexture2D`** (Engine_classes.hpp:14252) — the actual texture asset.
   - `FName TextureFileCacheName` — **the property that determines which `.tfc`
     RL opens**. We already use this empirically (Textures3 / Textures /
     Textures7 hijacks).
   - `FIndirectArray_Mirror Mips` — the mip array we patch (offset + size_disk
     + size_on_disk + dims for each mip).
   - `FGuid TextureFileCacheGuid` — RL doesn't validate this, confirmed
     empirically.

**Honourable mentions (not in the top 5 but relevant)** :

- **`UTeamColorScriptedTexture_TA`** (TAGame_classes.hpp:16284 — inherits from
  `UScriptedTexture`). This is the **runtime-composed team color lookup
  texture** referenced in our skin UPK comments (`TeamColorScriptedTexture_TA`).
  - Fields: `TArray<FLinearColor> PixelColorList`, `bMaxBrightness : 1`.
  - Methods: `void OnRender(UCanvas* Canvas)` (renders the live team palette
    into a render target), `RenderColorArray(...)`, `SetColorsArray(...)`,
    `static FLinearColor GetFullBrightColor(...)`.
  - Each `ATeam_TA` has one (`TeamScriptedTexture` at offset 0x02D8). The
    body shader samples this RT for the team-side palette every frame —
    that's how the lerp picks "Blue or Orange depending on team".

- **`UProductAsset_PaintFinish_TA`** (TAGame_classes.hpp:11635) — the paint
  finish (the metallic / pearlescent / matte parent that wraps a paint
  color). Includes `TextureParameterValues / ScalarParameterValues /
  VectorParameterValues` arrays. The static method
  `SetPaintFinishParametersOnMaterial(MatInst, Finish, Prefix)` is the
  one-shot copy that writes those onto a target MIC. So "applying a paint
  finish" is just bulk-setting parameters with a `Prefix` (e.g. "TeamFinish_"
  vs "CustomFinish_").

- **`UProductAttribute_PaintSettings_TA`** (TAGame_classes.hpp:11931) — per-
  decal config :
  - `FPaintMaterialGroup MaterialGroups[2]` — two groups (team / custom).
  - `bPaintParticles : 1`, `bPaintBody : 1`, `bGammaCorrect : 1`.
  - `FName PaintParameterName` — the FName that gets fed the paint color.
  - `EPaintColorVariant PaintType` (Primary / LightAccent / DarkAccent /
    Emissive / etc.).
  - `TArray<FPaintAttributeParameter> PaintAdditionalParameters`.
  - `TArray<UMaterialInterface*> PaintableMaterials, PaintableMaterialsMetallic`.
  - `TArray<UProductPaint_TA*> IncludePaints / ExcludePaints / UnsupportedPaints`.
  - `TArray<FPaintWithOverride> PaintsToOverride` — paint-specific overrides
    (e.g. Crimson uses a slightly different lookup than Cobalt for this decal).

- **`UProductAttribute_Painted_TA`** (TAGame_classes.hpp:12393) — the runtime
  attribute. Static methods that compose the final color :
  - `static FLinearColor GetPaintColor(UProductAttribute_PaintSettings_TA*, UProductPaint_TA*, EPaintColorVariant, bool bGammaCorrect)`.
  - `static void ApplyToBody(PaintSettings, Paint, CarMeshComponentBase_TA*)`.
  - `static void ApplyToSkin(PaintSettings, Paint, CarMeshComponentBase_TA*)`.
  - `static void OverrideMeshMaterial(PaintSettings, Paint, MeshComponent*, MaterialIndex, MaterialInterface* InMaterial)` —
    **this swaps an entire MaterialInterface, not just a parameter**. If we
    could call this statically with a "Diffuse-passthrough" material, we'd
    bypass the paint shader entirely. But the static call site is in the
    paint-application path, not the equip path, so we'd need to ship a
    `Painted` attribute on our donor's skin that points at the override
    material. Unclear if RL accepts that without a matching paint asset.

### 2.2 Kill switches found

The brief asked about "kill switches" like `bForceDiffuse`, `bNoPaintShader`,
`bIgnoreTeamColors`. The answer is **none of those exact names exist**, but
adjacent ones do :

| Field | Class | Effect when set |
|---|---|---|
| `bTeamFinishDisabled : 1` | `UProductAsset_Skin_TA` (0x015C bit 0x01) | Forces the team-finish slot off for this decal — the player's team-finish pick is ignored, no parameter is pushed. |
| `bCustomFinishDisabled : 1` | `UProductAsset_Skin_TA` (0x015C bit 0x02) | Same, for the custom-finish slot. |
| `ForcedTeamColors[2]` | `UProductAsset_Skin_TA`, `UProductAsset_Body_TA` | Non-zero values are pushed instead of the player's pick. This is the "promo decal locks your colors to brand colors" mechanism (Baroque / GaleFire_psplus). |
| `ForcedCustomColor[2]` | `UProductAsset_Skin_TA` | Same, for the custom color. |
| `bForceDefaultColors : 1` | `UTeamColorPreferences_TA` (offset 0x0060 bit 0x02), `UPlayerSettings_TA` (offset 0x00D0 bit 0x04) | Player-side colorblind-style preference that forces the team's `DefaultColorList` over `CurrentColorList`. Not useful for our decal problem. |
| `bIsLayerThumbnail : 1` | `ULandscapeMaterialInstanceConstant` | Unrelated. |
| `bPaintBody : 1` | `UProductAttribute_PaintSettings_TA` | Controls whether paint is applied to the body at all. **Could be useful if exposed per-skin** — but it's on the `PaintSettings` attribute attached to a paint, not on the skin itself. |

**Observation about `ForcedCustomColor` / `ForcedTeamColors`** : we already
know empirically (custom_decals.md, "ForcedColor catch") that donors whose
FName table contains the strings `"ForcedCustomColor"` or `"ForcedTeamColors"`
**lock both pickers in-game** (validated 2026-05-15 against `GaleFire_psplus`
and `Baroque` donors). The RLSDK confirms why : these are `CPF_Edit` properties
on `ProductAsset_Skin_TA`, set by Psyonix in the editor when designing the
decal. The presence in the name table is a strong indicator the export's
tagged-property block actually writes them.

**There is no `bForceDiffuse` or `bNoPaintShader`** that would let us tell the
shader "ignore the mask, just sample diffuse". The closest static lever is
`bTeamFinishDisabled` / `bCustomFinishDisabled` — but those don't bypass the
paint mask, they only disable the **paint-finish** layer (specularity /
emissive). The diffuse blend still happens.

### 2.3 The Engine-side material API surface

From `Engine_classes.hpp:23458–23494`, `UMaterialInstanceConstant`'s public
API is :

```cpp
class UMaterialInstanceConstant : public UMaterialInstance {
    TArray<FFontParameterValue>    FontParameterValues;     // 0x0320
    TArray<FScalarParameterValue>  ScalarParameterValues;   // 0x0330
    TArray<FTextureParameterValue> TextureParameterValues;  // 0x0340
    TArray<FVectorParameterValue>  VectorParameterValues;   // 0x0350

    // runtime accessors (called via ProcessEvent):
    void SetActorParameter      (FName, AActor*);
    void SetLinearColorParameter(FName, FLinearColor);
    void SetVectorParameter     (FName, FVector);
    void SetFloatParameter      (FName, float);
    void SetNameParameter       (FName, FName);
    void ClearParameterValues();
    void SetFontParameterValue   (FName, UFont*, FontPage);
    void SetVectorParameterValue (FName, FLinearColor);
    void SetTextureParameterValue(FName, UTexture*);
    void SetScalarParameterValue (FName, float);
    void SetParent               (UMaterialInterface* NewParent);
    bool GetFontParameterValue   (FName, UFont*&, int32_t&);
    bool GetTextureParameterValue(FName, UTexture*&);
    bool GetScalarParameterValue (FName, float&);
    bool GetVectorParameterValue (FName, FLinearColor&);
    // ... + Mobile variants
};
```

The arrays at 0x0320–0x0350 are **what we'd statically edit** if we wanted to
mimic AlphaConsole's runtime injection without code injection. Each entry is
an `F*ParameterValue` struct keyed by `FName`. Writing entries to
`TextureParameterValues` with key `Diffuse` and value pointing at our custom
`UTexture2D` would be the static equivalent of
`MIC.SetTextureParameterValue("Diffuse", customTex)`.

**Why we don't already do this** : the body main MIC isn't shipped in the
skin UPK — it's referenced from a parent body UPK (`MIC_Body_Octane`). When
we ship a donor in `mods/`, we override the donor's textures, but the body
MIC keeps using its own parent material's parameters and looks them up by
FName. The donor's `_RGB` mask Texture2D wins because the body MIC's
`PaintMaskInRGB` parameter points at the skin's RGB asset (via the
`SkinTextureParameter` slot configured in `UCarMeshComponentBase_TA::SetMaterialParameters`).

**To statically "win" against the paint shader**, we'd need to either :
1. Find the body main MIC inside the body UPK (e.g. `Body_Octane_SF.upk`),
   edit its `TextureParameterValues` array to set `Diffuse` to our texture —
   but this is install-wide and breaks every other decal on Octane. Not
   acceptable.
2. Ship a fully custom `UMaterialInstanceConstant` inside our donor UPK,
   whose parent is a "passthrough" `UMaterial`, and bind that MIC as the
   skin's material — but the donor doesn't define its own MIC for the body
   skin slot ; it references `MIC_Body_Octane` from the body UPK.
3. Edit the `SkinMaterialIndex` to point at a slot that uses a different,
   simpler material — but that material would have to already exist in the
   body mesh, and we don't ship custom body meshes.

---

## 3. Compiled shaders — can we patch them ?

### 3.1 Where they live

UE3 stores compiled shaders inside `UMaterial` exports, in a struct called
`FMaterialShaderMap`. The `UMaterial` itself contains :

- `bUsedWithSkeletalMesh`, `bUsedAsLightFunction`, ... compile-permutation
  flags.
- `MaterialShaderMaps[]` — one shader map per RHI (Direct3D9 / Direct3D11 in
  RL's case).
- Each shader map has many `FMaterialShader` entries — one per vertex/pixel
  permutation × quality × platform.
- Each `FMaterialShader` wraps a `FShaderCompilerOutput` blob containing
  **compiled DirectX bytecode** (DXBC, the FXC output, not HLSL source).

In Rocket League's cooked builds, shaders are in :

- The owning `UMaterial`'s export bytes inside the UPK.
- Globally in `*.uxx` files (UE3 cached-shader format) shipped at the root of
  `CookedPCConsole/`. RL ships several `RefShaderCache-*.upk` files (e.g.
  `RefShaderCache-PC-D3D-SM3.upk`, `RefShaderCache-PC-D3D-SM5.upk`).

### 3.2 Can we inspect or replace them ?

- **Inspection of DXBC bytecode.** Tools that *can* extract DXBC blobs from a
  raw DX11 shader file : `dxbc-disassembler` (CLI), `RenderDoc`'s shader view,
  the SM5 `D3DDisassemble` API. But to extract from a UE3 UPK you'd need a
  parser that walks the `FMaterialShaderMap` serialisation, which is **not
  publicly documented** for the post-2010 UE3 builds RL uses. UELib does
  **not** parse `FMaterialShaderMap`. The closest open-source parsers are
  Gildor's umodel (extracts textures + meshes only, no shaders) and the
  UE4-targeted `CUE4Parse` (wrong engine).
- **Disassembling RL's DXBC.** Theoretically possible if you can locate
  the blob — DXBC is a well-known format and `fxc /dumpbin` reverses it to
  bytecode listing. But you cannot get back to HLSL source without major
  effort (no open tool decompiles DXBC → HLSL with any quality).
- **Replacing shader bytecode.** Practically infeasible. RL likely validates
  shader maps against the material's hash (UE3's
  `FMaterialShaderMap::Id`); changing the bytecode without recomputing the
  hash makes RL recompile or fall back. Even if RL didn't validate,
  writing valid DXBC that fits the same input/output signature as RL's
  body shader requires us to author HLSL → compile with FXC → splice into
  the shader-map blob with the right RHI-specific header.

**Verdict.** Editing compiled shaders directly is **not** a viable path. The
tooling doesn't exist, the format isn't documented, and even if it did exist,
the security/compatibility risk vs. our current approach (texture-only swap)
is dramatically worse.

---

## 4. The decal slot loading path (reconstructed from RLSDK)

Here's the assumed equip-time flow, traced through RLSDK signatures :

1. **Player equips decal X.** UI triggers `UProductLoader_TA::SetLoadout(loadout)`
   on the local `CarMeshComponent_TA`.
2. `UCarMeshComponent_TA::SetLoadout(loadout)` reads the loadout's
   `SkinSlot` ID, loads the corresponding `UProductAsset_Skin_TA` via
   `UProductDatabase_TA::TLoadAsset<ProductAsset_Skin_TA>(ProductID)`.
3. The skin asset's `UMaterialInterface* Skin` is looked up (a MIC living in
   the skin UPK, e.g. `MIC_Body_Octane_Flames`).
4. `CarMeshComponentBase_TA::InitBodyVisuals()` is called. It walks the
   body's mesh, finds the slot at `BodyAsset->SkinMaterialIndex`, and applies
   the skin MIC there via `MeshComponent::SetMaterial(SkinMaterialIndex, SkinMIC)`.
5. **Parameter pump.** `CarMeshComponentBase_TA::SetMaterialParameters(ElementIdx, FMaterialParams)` is
   called, where `FMaterialParams` came from
   `UProductAsset_Skin_TA::GetSkinParameters(BodyAsset)` (see TAGame_classes.hpp:11776).
   This struct contains :
   - `TArray<FMaterialTextureParam> TextureParameters` — these set the
     `Diffuse`, `Skin`, `Skin_M`, etc. FName-keyed textures on the MIC.
   - `TArray<FMaterialVectorParam> VectorParameters` — color tints.
   - `TArray<FMaterialScalarParam> ScalarParameters` — float knobs.
6. **Color pump.** `CarMeshComponentBase_TA::SetMaterialColors(MatInst)` reads
   the player's team color (from `Team->CurrentColorList`) and custom color
   (from `CarMeshComponentBase_TA::CustomColorOverride`), then calls
   `SetMaterialColorParams(MatInst, ColorVec, ColorParamName, FullColorParamName)`
   to push them as `FLinearColor` vector parameters.
7. **Force-override check.** Before pumping colors, the code likely calls
   `UProductAsset_Skin_TA::HasForcedTeamColor(TeamIdx)`,
   `HasForcedCustomColor(TeamIdx)`. If `true`, it substitutes the skin's
   `ForcedTeamColors[idx]` / `ForcedCustomColor[idx]` for the player's pick.
   This is where the GaleFire_psplus / Baroque lock happens.
8. **Paint finish layer.** If the player has a `PaintFinish` equipped (e.g.
   "Striker"), `CarMeshComponentBase_TA::SetPaintFinishParameters(Mesh)`
   walks each element and either applies the finish's params or, if
   `bTeamFinishDisabled` or `bCustomFinishDisabled` is set on the skin asset,
   calls `SetPaintFinishParametersToAccent` with a disabled flag (which
   nulls out the finish's parameter contributions).
9. **Shader render.** The MIC's parent `UMaterial` (e.g. `M_Octane_Decal`)
   compiled DXBC shader runs every frame, sampling the bound textures and
   blending using the formulas `lerp(base, TeamColor, R_channel) +
   lerp(base, CustomColor, G_channel) + B_channel * windows + diffuse`.

**Where in this chain we have static leverage** :

| Step | Static-editable ? | How |
|---|---|---|
| 1 (equip ID) | Yes | UPK filename rename (we already use this) |
| 2 (load asset) | Yes | UPK content (mip patch + name hijack — we already do this) |
| 3 (MIC ref) | Partially | The donor's MIC lives in `Body_<Body>_SF.upk`, shared. We can't change it per-decal without per-body forking. |
| 4 (slot assignment) | Partially | `BodyAsset->SkinMaterialIndex` lives in the body UPK — install-wide. |
| 5 (param pump) | **Yes** — the params come from `UProductAsset_Skin_TA::GetSkinParameters` whose source is the skin UPK's `BodySettings.Parameters` field. **By editing the donor skin UPK's `FMaterialParams` we can push our own FName→texture/vector/scalar bindings into the body MIC at equip time, statically.** |
| 6 (color pump) | Yes | If we set `ForcedTeamColors / ForcedCustomColor` to `FLinearColor::White` (1,1,1,1), the multiplier becomes a no-op — the diffuse texture would render at its literal RGB. |
| 7 (force-override) | Yes | Setting `ForcedTeamColors[0]=(1,1,1)` and `ForcedCustomColor[0]=(1,1,1)` *with HasForcedTeamColor returning true* would push white as the team and custom colors → mask's R+G regions render with no tint. **But this likely also locks the pickers** (the standard ForcedColor side effect we already saw). |
| 8 (paint finish) | Yes | Setting `bTeamFinishDisabled=1` and `bCustomFinishDisabled=1` disables the finish layer (specularity / emissive overlay). Doesn't bypass the diffuse blend. |
| 9 (shader) | No | DXBC. Off-limits. |

**The most promising static lever we haven't tried yet** : **step 5**.
`FMaterialParams` is editable inside the skin UPK's `FSkinBodySettings`
serial data. By manipulating the texture-parameter entries, we could
override which FName-keyed textures the body MIC samples. If `M_Octane_Decal`
has any parameter named `Diffuse` (or even better, an unused parameter that
the shader still samples), we could bind our custom texture there directly.
This sidesteps the entire `_RGB`+team-color blend path **if the shader
respects that parameter unconditionally**. Worth a test.

---

## 5. BakkesMod SDK / ReplayManipulator — runtime reference

### 5.1 What BakkesMod's public SDK exposes for decals

`sandbox/research/sdks/BakkesModSDK/include/bakkesmod/utilities/DecalUtilities.h` :

```cpp
class BAKKESMOD_PLUGIN_IMPORT DecalUtilities {
public:
    _NODISCARD static tl::expected<ApplyDecalToCarResult, std::string>
        ApplyDecalToCar(const CarWrapper& car,
                        const pluginsdk::BodyShaderOverride& custom_body_decal);
    _NODISCARD static tl::expected<BodyAssetIdsCheckResult, std::string>
        GetBodyAssetIds(const CarWrapper& car);
    _NODISCARD static tl::expected<bool, std::string>
        ApplyDecalToBall(const BallWrapper& ball,
                         const pluginsdk::ShaderOverride& override);
};
```

`sandbox/research/sdks/BakkesModSDK/include/bakkesmod/core/custom_decals_structs.h` :

```cpp
namespace pluginsdk {
    using Tex             = std::shared_ptr<ImageWrapper>;
    using TextureOverride = std::map<std::string, Tex>;        // FName → texture
    using ColorOverride   = std::map<std::string, LinearColor>;// FName → color
    using ScalarOverride  = std::map<std::string, float>;      // FName → scalar

    struct ShaderOverride {
        TextureOverride textures;
        ColorOverride   colors;
        ScalarOverride  scalar;
    };
    struct BodyShaderOverride {
        ShaderOverride body_mic_override;
        ShaderOverride chassis_mic_override;
        int body_id = -1;
        int skin_id = -1;
    };
}
```

So BakkesMod's contract is identical to our hypothesis : at runtime it
**finds the body MIC for car X**, and for each `(FName, Texture)` pair in
`body_mic_override.textures` it calls `MIC.SetTextureParameterValue(FName, Texture)`.
The implementation is closed-source (lives inside `pluginsdk.lib` /
`bakkesmod.exe`). The FName strings are **caller-defined** — they come
from the plugin's JSON config, not from BakkesMod hard-coding them.

### 5.2 ReplayManipulator — confirms the FName keys are from JSON

`sandbox/research/sdks/ReplayManipulatorOpenSource/ReplayManipulatorOpenSource/Features/CustomTextures/CustomTextures.cpp:170` :

```cpp
void CustomTextures::ApplyDecalToCar(const CustomDecal& decal, const CarWrapper& car) {
    if (!CarHasRightBodyAndSkin(decal, car)) return;
    const auto sdk_decal = pluginsdk::BodyShaderOverride{
        .body_mic_override    = {.textures = decal.body},
        .chassis_mic_override = {.textures = decal.chassis},
        .body_id              = decal.BodyID,
        .skin_id              = decal.SkinID
    };
    DecalUtilities::ApplyDecalToCar(car, sdk_decal);
}
```

…and the keys for `decal.body` come from the AlphaConsole JSON (see
`CustomTextures.cpp:204-219` — `from_json(DecalConfig)` reads `Body` and
`Chassis` as `DecalPathMap = std::map<std::string, std::filesystem::path>`).

So the AlphaConsole pack's `Template.json` looks like :

```json
{
  "PackName": {
    "BodyID": 23,
    "SkinID": 0,
    "Body": {
      "Diffuse":   "body_diffuse.png",
      "Skin":      "body_skin.png"
    },
    "Chassis": { "Diffuse": "..." }
  }
}
```

The keys `Diffuse`, `Skin` (or `Body_Diffuse`, `1_Diffuse_Skin`, etc. in
different AlphaConsole-era packs) are **FName parameter names on the body
MIC**. Different packs use different keys depending on how the author
named them and which body their decal targets.

**Going beyond `Diffuse` and `Skin`.** Based on RL's UPK strings we've
already decrypted (custom_decals.md §"Body-side properties referenced"),
the body MIC's actual FName parameter list includes :

- `Diffuse` — the visible-art texture
- `Skin` — the colorable pattern
- `Skin_M` — the secondary mask (might be normal/spec/AO)
- `PaintMaskInRGB` — the explicit RGB-region mask
- `Team1_ColorLookup`, `Team2_ColorLookup` — palette lookup textures
- `DiffuseColor`, `TeamColor`, `SpecularColor` — vector params
- `HeadlightColor`, `TailLightColor`, `RimColor` — vector params
- `ChassisPaintColor`, `Chassis_EmissivePartsColor` — vector params
- `DefaultColor` — vector param

Any of these are valid FName parameter keys we could write into a static
`FMaterialParams` block in the donor skin UPK.

### 5.3 BakkesMod wrapper hierarchy (relevant subset)

`include/bakkesmod/wrappers/GameObject/MeshComponents/`:
- `MeshComponentWrapper.h`
- `SkeletalMeshComponentWrapper.h`
- `CarMeshComponentBaseWrapper.h` → exposes `GetCar()`. Most of the heavy
  lifting (the actual `ApplyPaintSettings` call) is wrapped in
  `DecalUtilities::ApplyDecalToCar`, not exposed as a public wrapper
  method — keeping the implementation closed.

`include/bakkesmod/wrappers/items/assets/`:
- `ProductAssetWrapper.h`
- `ProductAssetBodyWrapper.h` → `GetEquipProfile()`, `CanEquip(product)`.
- `ProductEquipProfileWrapper.h`.

There is **no `ProductAssetSkinWrapper`** in the public SDK — Bakkes hides
that class. So we can't even query the player's currently-equipped skin
asset's `ForcedTeamColors` from a plugin without raw UObject access.

---

## 6. Practical takeaways for our project

### 6.1 Tools that actually work on RL UPKs

| Tool | RL support | Use case |
|---|---|---|
| **Our own pipeline** (`src-tauri/src/commands/decal_swap.rs` + `RLUPKTool.exe`) | Full | Decrypt header, decompress body, parse + edit + re-pack. Already shipping. |
| **RLUPKTool** (`AltimorTASDK/RLUPKTool`) | Full | The standalone decryptor we already vendor. |
| **UE Explorer** (Eliot VU) | Partial — needs decrypted UPK | Browse decrypted UPK exports + decompile UScript bytecode of native helper classes (not our use case but useful for verifying class layouts beyond what RLSDK shows). |
| **UModel** (Gildor) | Partial | Texture/mesh extraction, no UScript or shaders. |
| **RLSDK headers** (`smallest-cock/RLSDK`) | Reference only | Definitive class/struct/offset reference, ~77 k LOC for TAGame alone. |

**Tools that DON'T work** :

- UE Explorer can't decrypt RL UPKs by itself (needs the decryptor pre-pass).
- No public tool inspects `FMaterialShaderMap` blobs in RL.
- No public RL UScript source-code dump exists.

### 6.2 Top 5 UScript classes for our problem (recap)

1. `UProductAsset_Skin_TA` (TAGame_classes.hpp:11740) — the skin definition. Holds `BodySettings`, `bTeamFinishDisabled`, `bCustomFinishDisabled`, `ForcedTeamColors[2]`, `ForcedCustomColor[2]`.
2. `UCarMeshComponentBase_TA` (TAGame_classes.hpp:15533) — the runtime renderer. Owns `ApplyPaintSettings()`, `SetMaterialParameters(ElementIdx, FMaterialParams)`, `SetMaterialColorParams`, `SetPaintFinishParameters`.
3. `UMaterialInstanceConstant` (Engine_classes.hpp:23458) — the parameterised material. Has `TextureParameterValues / VectorParameterValues / ScalarParameterValues` arrays directly editable in UPK bytes.
4. `UTexture2D` (Engine_classes.hpp:14252) — the texture asset. `TextureFileCacheName` field drives our TFC hijack ; `Mips` array drives the mip-redirect patch.
5. `UProductAsset_Body_TA` (TAGame_classes.hpp:11422) — the body. `SkinMaterialIndex`, `SkinParameters`, plus its own `ForcedTeamColors / ForcedCustomColor` (body-level forced colors).

Bonus class : `UTeamColorScriptedTexture_TA` (TAGame_classes.hpp:16284) — the runtime-composed team palette render-target. Confirms that team colors are sampled from a live texture, not a fixed parameter — explaining why our `_RGB` mask's R channel always renders with the player's team color.

### 6.3 Kill-switch summary

- **No literal `bForceDiffuse` / `bNoPaintShader` exists.**
- The closest legitimate static levers on the skin asset are :
  - `bTeamFinishDisabled` / `bCustomFinishDisabled` — disables paint-finish overlay (specularity/emissive), NOT the diffuse/mask blend.
  - `ForcedTeamColors[2]` set to `(1,1,1,1)` and `ForcedCustomColor[2]` set to `(1,1,1,1)` — would make the `lerp(base, TeamColor, R)` formula degenerate to `lerp(base, white, R)` which still tints by R, but the tint becomes `(1,1,1)` so the **mask R region renders as white** (not as the player's color). **But** setting these properties typically also disables both color pickers (the empirical "ForcedColor lock" we already saw). So we'd be trading "user-controllable colors" for "untinted diffuse passthrough on masked regions" — likely the wrong trade.

- The **highest-value static lever we haven't fully exploited** is editing the donor's `FSkinBodySettings::Parameters` (an `FMaterialParams` struct serialised inside the skin UPK's `BodySettings` array). Pushing our own FName-keyed texture overrides there is the static equivalent of `MIC.SetTextureParameterValue(FName, Tex)`. If the body shader respects an FName parameter that overrides the `_RGB` mask path (e.g. a `bUseLiteralDiffuse` scalar, or a `Diffuse` parameter that bypasses the lerp), we could trigger that from a pure data edit.

### 6.4 Compiled-shader editing

**Not viable.** No public tool parses RL's `FMaterialShaderMap`. DXBC bytecode disassembly is technically possible but reauthoring a working shader is infeasible without HLSL source. RL's likely hash validation on the shader map would also reject a modified blob.

---

## 7. Next-action proposal (informational, not a commit plan)

If the team wants to try bypassing the paint shader without runtime injection :

1. **Pick the most permissive donor.** Re-scan `find_paintable_multitexture.py` results, keeping only donors that ALSO lack `bTeamFinishDisabled` / `bCustomFinishDisabled` in their tagged properties (parse with the SDK-confirmed offset 0x015C bit 0x01/0x02).
2. **Dump and patch the donor's `FSkinBodySettings::Parameters`.** This is the `FMaterialParams` struct at offset 0x0008 inside each `FSkinBodySettings` array entry inside `ProductAsset_Skin_TA::BodySettings`. Try adding entries that bind `Diffuse` and `Skin` (or `PaintMaskInRGB`) to our custom textures via FName. The MIC will pick them up at equip time, possibly overriding the parent material's defaults.
3. **Probe for shader-exposed unblended parameters.** A black-box experiment : ship a donor where we push a known-invalid FName (e.g. `bUseLiteralDiffuse=1` as a scalar param) and see if any RL behaviour changes. If the shader has a discarded permutation-flag parameter, it would manifest as no-op in-game but might give visual hints if it's actually a real input.
4. **Inspect `RefShaderCache-PC-D3D-SM5.upk`** by hand for any DXBC permutation tagged with `M_Body_Octane_Decal` or similar. Even without disassembly, the file's serialized parameter signature lists the FName parameter inputs — which would definitively answer "does the shader read a `bUseLiteralDiffuse` style parameter or not".

None of these are guaranteed to work, but step 2 is cheap to try and step 3/4 are reconnaissance that informs the long-term plan.

---

## 8. Source references (all local clones)

```
sandbox/research/sdks/RLSDK/RLSDK/SDK_HEADERS/
    Core_classes.hpp        Engine_classes.hpp     Engine_structs.hpp
    TAGame_classes.hpp      TAGame_structs.hpp     TAGame_parameters.hpp
    ProjectX_classes.hpp    GFxUI_classes.hpp      AkAudio_classes.hpp

sandbox/research/sdks/BakkesModSDK/include/bakkesmod/
    utilities/DecalUtilities.h
    core/custom_decals_structs.h
    wrappers/GameObject/MeshComponents/CarMeshComponentBaseWrapper.h
    wrappers/items/assets/{ProductAssetWrapper.h,ProductAssetBodyWrapper.h}

sandbox/research/sdks/ReplayManipulatorOpenSource/ReplayManipulatorOpenSource/
    Features/CustomTextures/{CustomTextures.h,CustomTextures.cpp}
    Features/BallHide/BallHiderAndDecals.{h,cpp}
    Features/TextureCache.{h,cpp}
```

Key line citations (RLSDK paths abbreviated to `<RLSDK>/SDK_HEADERS/`) :

- `<RLSDK>/TAGame_classes.hpp:11740` — `UProductAsset_Skin_TA` class
- `<RLSDK>/TAGame_classes.hpp:11751-11752` — `bTeamFinishDisabled` + `bCustomFinishDisabled` bits
- `<RLSDK>/TAGame_classes.hpp:11755-11756` — `ForcedTeamColors[2]` + `ForcedCustomColor[2]` arrays
- `<RLSDK>/TAGame_classes.hpp:11768-11772` — `IsAnimatedSkinType`, `HasForcedCustomColor`, `HasForcedTeamColor`, `HasForcedCustomFinish`, `HasForcedTeamFinish`
- `<RLSDK>/TAGame_classes.hpp:11775-11776` — `GetSkinBodySettings`, `GetSkinParameters`
- `<RLSDK>/TAGame_classes.hpp:11453-11460` — `UProductAsset_Body_TA::SkinMaterialIndex` + ForcedColors
- `<RLSDK>/TAGame_classes.hpp:15533-15642` — `UCarMeshComponentBase_TA` full surface
- `<RLSDK>/TAGame_classes.hpp:15583-15603` — `ApplyPaintSettingsToObject`, `SetMaterialColorParams`, `SetMaterialColors`, `SetPaintFinishParameters`, `ApplyPaintSettings`
- `<RLSDK>/TAGame_classes.hpp:16284-16303` — `UTeamColorScriptedTexture_TA` (the runtime team-palette RT)
- `<RLSDK>/TAGame_classes.hpp:11635-11663` — `UProductAsset_PaintFinish_TA::SetPaintFinishParametersOnMaterial`
- `<RLSDK>/TAGame_classes.hpp:11931-11951` — `UProductAttribute_PaintSettings_TA` (PaintParameterName, PaintType, PaintableMaterials)
- `<RLSDK>/TAGame_classes.hpp:12393-12422` — `UProductAttribute_Painted_TA::ApplyToBody/ApplyToSkin/OverrideMeshMaterial`
- `<RLSDK>/Engine_classes.hpp:23458-23494` — `UMaterialInstanceConstant` full surface
- `<RLSDK>/Engine_classes.hpp:14252-14306` — `UTexture2D` (TextureFileCacheName, Mips)
- `<RLSDK>/TAGame_structs.hpp:364-369` — `FMaterialParams { Texture/Vector/Scalar parameters }`
- `<RLSDK>/TAGame_structs.hpp:381-386` — `FSkinBodySettings { Body, Parameters, AdditionalBodyParameters }`
- `<RLSDK>/TAGame_structs.hpp:2720-2726` — `FParameterInformation`
- `<RLSDK>/TAGame_classes.hpp:50-55, 484-507` — `EPaintColorVariant` enum (Primary / LightAccent / DarkAccent / Emissive / DeEmissive / Complementary / Balanced / Tertiary / Additive)

---

## 9. Conclusion (one-paragraph)

RL's decal pipeline is fully decompilable at the **data layer** : we have
the AES keys, RLUPKTool to decrypt headers, and RLSDK as a reflection of
every UScript class with offset-correct member layouts. UE Explorer
decompiles native UScript bodies if we want them. **What we cannot
decompile** is the compiled DXBC shader bytecode inside `UMaterial`
exports — no public tool parses `FMaterialShaderMap` and even if we
extracted DXBC, recompiling/rewriting it is not feasible. The most
promising **static** lever we haven't yet exploited is editing the donor's
`FSkinBodySettings::Parameters` (`FMaterialParams` struct, three TArrays of
FName-keyed texture/vector/scalar overrides) to push our own FName bindings
at the body MIC — the file-only equivalent of AlphaConsole's runtime
`MIC.SetTextureParameterValue(...)` hook. There is **no `bForceDiffuse`**
or `bNoPaintShader` kill switch ; the only adjacent flags are
`bTeamFinishDisabled` / `bCustomFinishDisabled` (which only disable the
paint-finish overlay, not the diffuse/mask blend), and the `ForcedTeamColors`
/ `ForcedCustomColor` arrays (which lock the picker as a side-effect — the
wrong trade).
