# UE3 `SetTextureParameterValue` semantics vs file-only TFC hijack

> **Date** : 2026-05-15. Investigation triggered by empirical finding that the
> dual-TFC `D` + `Skin → (0,0,0,255)` swap does **not** produce literal RGB on
> the car ; "white fallback" appears where literal RGB was expected.
>
> Source basis : `CodeRedModding/UnrealEngine3` (UE3 build 10897, the
> publicly-mirrored 2013 source drop), local `RLSDK` headers, AlphaConsole
> pack forensics (`docs_rl/alphaconsole_packs_forensics.md`), and Yuna Itzy
> in-game test (`docs_rl/custom_decals.md` §"What we learned in-game").

## TL;DR

**`SetTextureParameterValue` has NO shader-side effect that file-only TFC
hijack cannot replicate.** Both paths end at the same place :
`FTexture2DResource::InitRHI()` uploading `Mip.Data` bytes to the GPU sampler
slot resolved by FName lookup. There is no "baked sampler pointer", no
per-MIC shader recompile, no separate texture cache that the AC runtime path
hits and the TFC path bypasses. The "literal RGB still renders white"
symptom does **not** come from missing shader-side propagation. It comes from
a **different layer of the pipeline that AC also doesn't bypass — Psyonix's
native body shader simply has no code path that outputs the `Diffuse` sampler
value un-modified.**

## 1. What `SetTextureParameterValue` actually does (canonical UE3, build 10897)

```cpp
// Engine/Src/MaterialInstanceConstant.cpp:202
void UMaterialInstanceConstant::SetTextureParameterValue(FName ParameterName, UTexture* Value)
{
    FTextureParameterValue* ParameterValue =
        MICTextureParameterMapping::FindParameterByName(this, ParameterName);
    if (!ParameterValue) {
        ParameterValue = new(TextureParameterValues) FTextureParameterValue;
        ParameterValue->ParameterName = ParameterName;
        ...
    }
    if (ParameterValue->ParameterValue != Value) {
        ParameterValue->ParameterValue = Value;
        // Enqueue an update on the rendering thread.
        MICTextureParameterMapping::GameThread_UpdateParameter(this, *ParameterValue);
    }
}
```

`GameThread_UpdateParameter` (defined by the
`DEFINE_MATERIALINSTANCE_PARAMETERTYPE_MAPPING` macro in
`Engine/Inc/MaterialInstance.h`) **does not touch the shader bytecode**. It
enqueues an `ENQUEUE_UNIQUE_RENDER_COMMAND_THREEPARAMETER(SetMIParameterValue, ...)`
that walks the MIC's `FMaterialInstanceConstantResource` (the rendering-thread
mirror) and appends/updates a `TNamedParameter<UTexture*>` entry in the
resource's parameter array. **That's it.**

At draw time the pixel shader runs an FName lookup against this resource
array : `FMaterialInstanceConstantResource::GetTextureValue(ParameterName, &OutFTexture, Context)`
returns `(*Value)->Resource` — i.e. `UTexture2D::Resource`, the
`FTexture2DResource*` — and the renderer binds *that* `FTexture2DResource` to
the sampler slot the shader compiled for `ParameterName`.

**Key consequence** : the runtime hook gives the shader a *different*
`UTexture2D*` (and therefore a different `FTexture2DResource*`, a different
GPU texture, a different mip pyramid). The file-only TFC hijack gives the
shader the **same** `UTexture2D*` (`Force_Body_D`) but with **different bytes**
in its mips (loaded from the same TFC at a "fake" offset we wrote).

## 2. Where the mip bytes come from (the bridge between the two paths)

```cpp
// Engine/Src/Texture2D.cpp:2375
FTexture2DResource::FTexture2DResource(UTexture2D* InOwner, INT InitialMipCount, const FString& InFilename)
{
    ...
    for (INT MipIndex=0; MipIndex<ARRAY_COUNT(MipData); MipIndex++) {
        FTexture2DMipMap& Mip = InOwner->Mips(MipIndex);
        if (Mip.Data.IsStoredInSeparateFile()) {
            // open the TFC, seek to Mip.Data.GetBulkDataOffsetInFile()
            TFCReader->Seek(Mip.Data.GetBulkDataOffsetInFile());
            if (Mip.Data.IsStoredCompressedOnDisk()) {
                TFCReader->SerializeCompressed(MipData[MipIndex], 0, Mip.Data.GetDecompressionFlags(), FALSE);
            } else {
                TFCReader->Serialize(MipData[MipIndex], Mip.Data.GetBulkDataSize());
            }
        } else {
            Mip.Data.GetCopy(&MipData[MipIndex], bDiscardInternalCopy); // inline
        }
    }
    // ... InitRHI() uploads MipData[] to GPU
}
```

So : when the body MIC asks for `Diffuse` and receives `Force_Body_D`, RL
constructs an `FTexture2DResource` that reads bytes from `Textures3.tfc` at
the offset the UPK's `FByteBulkData` records. **Our hijack rewrites that
offset to point at our sparse custom TFC** — the bytes the GPU receives are
ours. Functionally identical to a runtime swap **of the same UTexture2D
identity**.

## 3. The five questions, answered

### Q1. Does `SetTextureParameterValue` update the shader parameter map?

**No** — UE3's compiled shader (DXBC) compiles **only against FNames** ; the
sampler-slot → FName mapping is baked into the shader at cook time. The MIC's
`TextureParameterValues` array is a runtime FName → `UTexture*` lookup table.
Per-frame, the shader runtime walks the MIC's render-thread parameter array,
finds the entry whose `Name == "Diffuse"`, and binds its `UTexture2D->Resource`
to the sampler. No shader recompile happens at any point.

### Q2. Is the render thread's texture pointer cached?

**Yes, in `FMaterialInstanceConstantResource::TextureParameters`** — a
`TArray<TNamedParameter<UTexture*>>` mirror that lives on the render thread.
`GameThread_UpdateParameter` enqueues a render command that mutates this array
in place. **The cache is keyed by FName, not by `UTexture*` identity.** When
the AC plugin replaces the texture, the cached `UTexture*` value at FName
`Diffuse` changes. When our TFC hijack runs, the **same** `UTexture*` (the
pristine `Force_Body_D` `UTexture2D` instance) stays in the cache — only the
GPU bytes inside its `FTexture2DResource` differ.

### Q3. Could the shader sample from a different cache when the pointer changes?

**Possibly relevant** : when AC's new `UTexture2D` becomes the bound texture,
it may have different `LODGroup`, `bIsStreamable`, `Format`, `bSRGB`,
`UnpackMin/Max`, `CompressionSettings`, or `AddressX/Y` — and those settings
do flow into sampler state and color decoding. If AC's new texture is, say,
`PF_A8R8G8B8` linear-space and our hijacked `Force_Body_D` is `PF_DXT5` sRGB,
the renderer applies an sRGB→linear gamma transform on our path that AC
bypasses. **This is a real difference**, but it would skew gamma (over-bright
/ over-dark), **not** wholesale replace RGB with "white". So probably not the
white-pixel root cause, but it is one of the few genuine shader-side
divergences worth knowing about.

### Q4. What does BakkesMod's `DecalUtilities::ApplyDecalToCar` actually do?

From the public SDK headers
(`sandbox/research/sdks/BakkesModSDK/include/bakkesmod/utilities/DecalUtilities.h`
+ `core/custom_decals_structs.h`) and the open-source consumer
(`ReplayManipulatorOpenSource/CustomTextures.cpp:170`), the call is
**exactly** : for each `(FName, ImageWrapper)` pair in `body_mic_override.textures`,
locate the body MIC and call `MIC.SetTextureParameterValue(FName, MakeTextureFromPNG(image))`.
The plugin **does not** modify mesh slot bindings, does not patch shader
uniforms, does not call `OverrideMeshMaterial`. It's a pure
SetTextureParameterValue spray. So our static path **is** the static
equivalent of what AC's runtime does.

### Q5. Does `Force_Body_D`'s `UTexture2D` have special properties that change rendering?

Already inspected via `parse_texture2d_real.py` :
`Format=PF_DXT5, SizeX=2048, SizeY=2048, TextureFileCacheName=Textures3,
MipTailBaseIdx=11, FirstResourceMemMip=5`. Nothing unusual. The `_D` suffix
is a convention, not a sampler flag. The same `UTexture2D` flags would be on
AC's substituted texture if it was generated by a normal authoring pipeline.

## 4. The most likely real reason "literal RGB renders as white"

The hijack mechanically delivers our BC3 bytes to the GPU sampler bound at
the body MIC's `Diffuse` slot. Validated by Yuna-Itzy in-game test for the
**other** Force_Body_RGB hijack (R+α markers do drive the paint shader as
documented). So the bytes arrive. **What's failing is the shader-side
composition logic.**

Three concrete hypotheses, ranked by likelihood :

1. **The "Skin = (0,0,0,255)" neutralization mask is not, in fact, neutral
   for this material.** The decompiled `Body_Paintable_Mat` parameter list
   in `docs_rl/rl_decal_shader_re.md` § 2.2 shows that the parameter named
   `Skin` is a **`VectorParameter`** (an `FLinearColor`), not a
   `TextureSampleParameter2D`. The MIC's TFC-bound `Skin` texture is sampled
   via the `Masks` `TextureSampleParameter2D` (CarTextures group) instead.
   Setting Force_Body_BlankSkin to `(0,0,0,255)` neutralizes a parameter the
   body shader **doesn't read for the team-color mask path**. The actual
   color-zone mask is still `_RGB` (Force_Body_RGB) on the Stars slot, which
   for that slot equals Force_Body_D's RGB channels reinterpreted as a mask.
   So our "literal diffuse" pixels in the D-slot get re-interpreted by the
   shader's RGB+α marker convention as "no zone → fallback color".

2. **Stars slot has no `_D` texture in its native cooked set.** Per
   `docs_rl/custom_decals.md` line 502, the Stars slot is a `[_RGB] only`
   pattern. There is no separate Force_Body_D texture in its native UPK —
   the body MIC inherits diffuse from its parent body's defaults (the
   chrome/metal base texture). Our Textures→MyDecal_ hijack feeds new bytes
   to a texture **slot that may not be sampled at all for Stars's compiled
   shader permutation**, and the rendering reverts to the parent material's
   white fallback texture (`Pepe_Body_BlankSkin` is documented as the
   universal "blank skin" base in `docs_rl/custom_decals.md`:55, default
   color is white).

3. **AlphaConsole's "FullColor" trick relies on the runtime hook setting
   the texture on the body MIC at a *different* FName than what RL's cooked
   material instance exposes.** AC's manifest uses literal FName `Diffuse` ;
   maybe AC's plugin internally re-maps it to the body's actual sampler
   parameter name (e.g. `BodyDiffuseTexture` or the slot-specific FName from
   `Body_Paintable_Mat`'s `Diffuse` `VectorParameter`). If true, the file
   hijack of "Force_Body_D" reaches a UTexture2D the body shader never
   samples, while the runtime AC call reaches one it does.

Hypothesis **1** is the most likely, with hypothesis **2** as a strong
secondary on the specific Stars slot we tested. The two compound : on a
slot that has no `_D` texture, neutralizing `_Skin` is doubly irrelevant.

## 5. Concrete experiment to prove/disprove

**The ground-truth probe (single experiment that decides everything)** :

1. Switch the donor from Stars (1-texture `[_RGB] only`) to **GaleFire**
   (4-texture `[_BlankSkin, _Curvature, _D, _RGB]`). GaleFire is documented
   in `docs_rl/custom_decals_FINAL_PLAN.md` § Phase 1 and is the slot whose
   MIC we already dumped — it definitely has both `_D` and `_RGB` textures
   bound.
2. Generate a **solid pure red 2048×2048 BC3** (8 mips) via our existing
   `image_dds` pipeline. The block bytes for `(R=255, G=0, B=0)` are
   `0x00, 0xf8, 0x00, 0xf8, 0x00, 0x00, 0x00, 0x00` for the color block + 8
   zero alpha bytes ; trivially generable.
3. Hijack **only the `_D` texture** (Force_Body_D → MyDecal_.tfc). Leave
   `_RGB` and `_BlankSkin` pristine.
4. Equip the donor decal with a player loadout that has primary=blue,
   secondary=yellow. The body shader composites :
   - Where the original Stars `_RGB` mask was `(R=255, α=0)` (primary zone):
     **blue** (team color overrides red diffuse).
   - Where mask is `(R=255, α=255)` (secondary zone): **yellow**.
   - Where mask is `(R=0, B=0)` (everywhere else — diffuse passthrough zone
     on a paintable shader): **pure red** if the shader has a literal-diffuse
     fallback ; **white** if it does not.
5. Run in-game.

**Outcome interpretation** :

- **Body regions render solid red where mask is "no zone"** → shader DOES
  have a literal-diffuse passthrough path. Our pipeline can deliver literal
  RGB. The current Stars failure is hypothesis 2 (Stars slot has no `_D`
  binding). Fix : switch UI default donor to GaleFire-class slots.
- **Body regions render white where mask is "no zone"** → shader does NOT
  have a literal-diffuse fallback. The native body shader always composites
  through the paint pipeline ; "no zone" = "default color = white". **File-only
  literal RGB is fundamentally impossible** without a DXBC patch, and the
  documented AlphaConsole "FullColor" pattern only works because AC's
  runtime plugin is installing a *different shader permutation*, not just
  swapping textures. AC's `Diffuse` FName must therefore target a
  parameter the runtime hook re-binds with custom shader state we cannot
  reproduce statically.

## 6. If file-only literal RGB is fundamentally impossible

If the experiment in §5 yields "white where mask is no-zone", the static
ceiling is :

1. **Paintable mode** — user PNG is treated as an RGB mask using the
   documented marker palette (R=primary, R+α=secondary, B=windows). User
   picks colors in garage. This is what RL natively supports without any
   shader bypass. We already ship this.
2. **No FullColor mode statically.** Mark the FullColor toggle as
   "AlphaConsole-online-only" in the UI. Document that post-EAC literal
   RGB requires a runtime hook (i.e. would re-introduce process-injection
   risk).
3. **The only other static lever** is `FSkinBodySettings::Parameters`
   (`docs_rl/unrealscript_shader_decompile.md` §6.3). Pushing FName
   bindings into the donor skin UPK's `BodySettings` may discover a
   shader-exposed scalar parameter that triggers an unblended-diffuse
   permutation. This is reconnaissance, not a guarantee — and it requires
   header growth under AES-ECB which we already do for `upk_renamer`.

The most honest framing : **our TFC hijack reaches the GPU sampler
identically to AC's runtime call. What we cannot reach is AC's *plugin
state* — specifically the additional render-thread state AC pushes into
the body shader that switches its permutation away from team-color paint
and toward literal diffuse passthrough.** That state isn't in the UPK ;
it's in the running process. File-only cannot install it.

## 7. Source citations

- `CodeRedModding/UnrealEngine3 : Development/Src/Engine/Src/MaterialInstanceConstant.cpp:202-238` — `SetTextureParameterValue` implementation.
- `CodeRedModding/UnrealEngine3 : Development/Src/Engine/Inc/MaterialInstance.h:46-145` — `DEFINE_MATERIALINSTANCE_PARAMETERTYPE_MAPPING` macro defining `GameThread_UpdateParameter` and `RenderThread_UpdateParameter`.
- `CodeRedModding/UnrealEngine3 : Development/Src/Engine/Src/MaterialInstanceConstant.cpp:75-93` — `FMaterialInstanceConstantResource::GetTextureValue` (the render-thread sampler resolver).
- `CodeRedModding/UnrealEngine3 : Development/Src/Engine/Src/Texture2D.cpp:2375-2495` — `FTexture2DResource::FTexture2DResource` (the TFC reader path that turns our hijacked bytes into the GPU sampler input).
- `sandbox/research/sdks/RLSDK/RLSDK/SDK_HEADERS/Engine_classes.hpp:23458-23494` — RL's `UMaterialInstanceConstant` class layout (offsets match generic UE3).
- `sandbox/research/sdks/BakkesModSDK/include/bakkesmod/utilities/DecalUtilities.h` + `core/custom_decals_structs.h` — BakkesMod public API confirms the runtime hook is a pure `SetTextureParameterValue` spray.
- `docs_rl/rl_decal_shader_re.md` § 2.2-2.3 — `Body_Paintable_Mat` parameter list (Skin is a Vector, not a Texture ; the real mask is `Masks`).
- `docs_rl/custom_decals.md` § "What we learned in-game" — Yuna Itzy empirical test : `[_RGB] only` slot rejects literal RGB.
- `docs_rl/alphaconsole_packs_forensics.md` § 2 — AC's two FullColor patterns (only Pattern A relies on the virtual `slot=-3` AC-plugin construct).
