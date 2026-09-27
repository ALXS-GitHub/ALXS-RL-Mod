# AlphaConsole runtime vs file-only — why our R=43 zones render WHITE instead of literal diffuse

> **Date.** 2026-05-15.
> **Context.** Our file-only pipeline (donor `skin_octane_galefire_SF.upk` →
> dual-TFC hijack of `Textures`/`Textures7`) paints the user PNG into the
> donor's two textures correctly, but in-game the R=43 sentinel zones of the
> mask render **white**, not the user's literal diffuse. AlphaConsole's
> `body_mic_override.textures = {"Diffuse": …, "Skin": …}` runtime call
> produces literal diffuse in those same zones. This document explains the
> root cause and the experiment to confirm/fix.

## 1. The MIC AC targets vs the MIC we target — they are NOT the same

This is the single biggest finding, and it matches 100% of the contradictory
evidence we've collected over 3 days.

### What AC hooks at runtime

The BakkesMod SDK contract:

```cpp
struct BodyShaderOverride {
    ShaderOverride body_mic_override;     // ← "body MIC", NOT "decal MIC"
    ShaderOverride chassis_mic_override;
    int body_id, skin_id;
};
```

(`sandbox/research/sdks/BakkesModSDK/include/bakkesmod/core/custom_decals_structs.h`)

Field name is literally `body_mic_override`. In RL's mesh hierarchy
that's the body's **main material slot's** MIC — i.e. `MIC_Body_Octane`
(export 786 in `Startup.upk` → `Vehicle_Parent_Materials`), parented to
`MIC_Body_Paintable_All` (export 832), parented to `Body_Paintable_Mat`
(export 261).

AC's plugin calls `MIC_Body_Octane.SetTextureParameterValue("Diffuse", userTex)`.
The body's main material slot rebuilds with that texture bound to the
`Diffuse` parameter the parent `Body_Paintable_Mat` declares (a
`TextureSampleParameter2D` in group `CarTextures` —
`docs_rl/rl_decal_shader_re.md` §2.2).

`MIC_Body_Paintable_All` already has a default `Diffuse → Object idx 2570`
(some stock body diffuse, `docs_rl/rl_decal_shader_re.md` §2.4). AC
overrides that default at runtime → every fragment shader instance that
samples `Diffuse` reads the user's pixel content. The R=43 sentinel in
the Skin mask then says "ignore the team-color tint, render literal
`Diffuse`" — and Diffuse is now the user's PNG → user art shows in those
zones. 

### What our file-only pipeline currently targets

We hijack `Textures` FName (idx 206) in the **donor skin UPK**
`skin_octane_galefire_SF.upk`. That redirects the `TextureFileCacheName`
lookup of `Force_Body_D` from `Textures.tfc` to `MyDecal_.tfc`. Net
effect: when `Octane_GaleFire_MIC` (the **decal slot MIC**, export 67 of
the donor) is materialized, its `Diffuse → Force_Body_D` binding pulls
user-PNG bytes.

`Octane_GaleFire_MIC` is the MIC bound at the **`SkinMaterialIndex` slot**
of the body mesh — a *separate* material slot from the body's main slot.
The body main slot still uses `MIC_Body_Octane` with its inherited
default `Diffuse` from `MIC_Body_Paintable_All`.

When the shader is evaluated for the body's main-slot fragments, it
samples the *main slot MIC's* `Diffuse` parameter, NOT the decal slot's
texture. **The body's main slot has no idea our user PNG exists.** Where
the mask's R=43 sentinel says "show literal Diffuse here", the shader
shows the **default stock Diffuse** from `MIC_Body_Paintable_All` —
which is a flat white-ish base texture → that's why we see white.

### The two material slots — what does what

| Material slot | MIC bound there | What it renders | Parameters the shader samples |
|---|---|---|---|
| Body main (`int32_t ChassisMaterialIndex` / default slot 0) | `MIC_Body_Octane` (inherits from `MIC_Body_Paintable_All` → `Body_Paintable_Mat`) | The whole car body | `Diffuse` (default = stock body texture in Startup.upk), `Masks`, `Skin` (vector), team palette RT, curvature |
| Skin (`BodyAsset->SkinMaterialIndex`) | `Octane_GaleFire_MIC` (inherits from `MIC_Body_Paintable_All`) | The decal layer ON TOP of the body main slot | `Diffuse → Force_Body_D`, `Skin → Force_Body_BlankSkin`, `CurvaturePack → Body_Force_Curvature` |

The body main shader has a `Diffuse` parameter. The decal slot MIC also
has a `Diffuse` parameter (both inherited from the same parent). They are
**two distinct texture bindings**, on two distinct material instances.
Editing one does not affect the other.

**The R=43 mask sentinel ("show literal diffuse here") is interpreted
inside the body main shader**. It tells *that* shader to bypass team-tint
and pull from *its own* `Diffuse` parameter (which is bound to the stock
white texture). Our user PNG is bound to the *decal slot MIC's*
`Diffuse`, which is never sampled by the body main shader. Result:
white.

## 2. Why AC's pattern works and ours doesn't — annotated

Step-by-step trace of AC equipping a `slot=-3 item=-1` + `slot=1` pair:

1. Player equips a normal Psyonix decal at `slot=1` (e.g. Octane 306 with
   `Skin → MainBodyOnly.png`). The shader sees the mask, sentinel-detects
   R=43 zones, branches to "show literal Diffuse".
2. AC's plugin hooks `MIC_Body_Octane.SetTextureParameterValue("Diffuse",
   user_png_loaded)`. The body main shader now samples user PNG for its
   `Diffuse` channel.
3. The sentinel-branch instruction in the body main shader returns user
   PNG pixel content. R=43 zones render the user's literal RGB.

Our file-only pipeline:

1. We rename the donor to the user's owned slot (e.g. `Skin_Octane_Stars_SF.upk`).
2. Donor's `Octane_GaleFire_MIC` overrides `Diffuse → Force_Body_D` (now
   redirected via TFC hijack to user PNG). The decal slot shader samples
   user PNG correctly.
3. **But** the body main shader's R=43 sentinel branch still pulls from
   `MIC_Body_Octane`'s `Diffuse`, which is the stock white texture. The
   user PNG is bound to a different MIC's `Diffuse`.

The decal slot is just a *layer on top of* the body's main slot fragments.
Where the decal-slot mask renders opaque it covers the body main, but
the R=43 sentinel zone is precisely the place where the decal slot is
SUPPOSED to be **transparent** so the body main's literal diffuse shows
through. The decal slot's content there is irrelevant; what shows is the
body main shader's output.

## 3. Concrete in-game evidence supporting this hypothesis

- **High-confidence cluster.** R=43 zones rendering white but R=255 zones
  rendering player colors. Player colors come from
  `UTeamColorScriptedTexture_TA` which is sampled by the body main MIC.
  So team-color path through body main shader works → body main shader is
  what's drawing those pixels. The decal slot layer at R=255 is opaque
  red mask → triggers "show team color" in the body main shader.
- **The 2026-05-15 evening Stars test rendered user PNG correctly on
  R=255 primary/accent zones.** Those zones drive the body main shader to
  fetch team palette + render lerp(diffuse, palette, mask.r). The
  diffuse contribution there is mostly hidden by the strong lerp toward
  palette color, so we don't see the "stock white diffuse" leak. R=43
  zones lerp ≈ 0 → diffuse contribution dominates → stock white shows.
- **The donor's decal-slot diffuse was clearly painted with user art**
  but it's only visible at the *edges* of the decal where the decal
  slot's mask is opaque AND non-sentinel — i.e. very few pixels in
  practice. Most of the decal slot fragment is transparent (alpha
  cutout) so the user art there is invisible.

## 4. Are there 2 distinct "Diffuse" texture parameters?

Yes. Two `MaterialExpressionTextureSampleParameter2D` nodes named
`Diffuse` exist in `Body_Paintable_Mat`'s expression list (per
`docs_rl/rl_decal_shader_re.md` §2.2 — "Diffuse (×2)"). One per
material slot variant. The MIC inheritance chain:

```
Body_Paintable_Mat (Material; declares both Diffuse parameters)
   └ MIC_Body_Paintable_All (defaults: Diffuse → idx 2570)
        ├ MIC_Body_Octane (body main, inherits Diffuse from parent)
        └ Octane_GaleFire_MIC (decal slot, OVERRIDES Diffuse → Force_Body_D)
```

Our hijack lands at the second branch (Octane_GaleFire_MIC). AC lands at
the first branch (MIC_Body_Octane).

## 5. Force_Body_D's pixel format and BlankSkin role

- `Force_Body_D` is `PF_DXT5` (BC3) — the user's compressed PNG. Standard.
  No alpha-channel sentinel hidden there.
- `Force_Body_BlankSkin` is sampled by the decal slot MIC's `Skin`
  parameter. In the body main shader, `Skin` is a *Vector* (LinearColor)
  not a Texture (`rl_decal_shader_re.md` §2.3). So BlankSkin is only the
  decal layer's secondary base texture, not the body's literal art.

## 6. AC does NOT replace the entire MIC — it modifies parameters in-place

`UMaterialInstanceConstant::SetTextureParameterValue(FName, UTexture*)`
mutates the `TextureParameterValues` array of an existing MIC.
`SetMaterialColors`, `SetTextureParameterValue` etc. on the *same MIC*
are how AC drives the override (`docs_rl/unrealscript_shader_decompile.md`
§2.3). It doesn't swap the parent or the MIC pointer. We can statically
mimic this exact operation if we target the **right MIC**.

## 7. The fix — concrete plan

The **static equivalent of AC's hook is to edit `MIC_Body_Octane`'s
`TextureParameterValues` array in `Startup.upk` to bind `Diffuse → our
custom Texture2D`**.

But `Startup.upk` is install-wide → editing it would apply our PNG to
**every Octane car globally**, including other players' bodies in online
matches. Unacceptable.

The acceptable alternatives, in order of feasibility:

1. **Path A — body-UPK reparent (preferred).** Most Octane body UPKs
   (e.g. `Body_Octane_SF.upk`) import `MIC_Body_Octane` from
   `Startup.upk` and use it at the main material slot. Cannot easily
   override that import per-decal.

2. **Path B — donor MIC reparent up the chain.** Reparent
   `Octane_GaleFire_MIC` from `MIC_Body_Paintable_All` to a new MIC we
   ship inside the donor UPK whose `Diffuse` defaults to user PNG and
   whose other defaults match `MIC_Body_Paintable_All`. Inheritance
   would propagate user-PNG-as-Diffuse downward. **But** the body main
   slot still uses `MIC_Body_Octane`, not our new MIC.

3. **Path C — make our donor's MIC the body main slot (the AC analog).**
   Find or build a donor that has `MIC_Body_Octane` cooked into the
   donor UPK itself (not imported), with our user PNG bound to its
   `Diffuse`. When the player equips this skin, RL's
   `CarMeshComponentBase_TA::InitMaterials()` reads
   `Skin->BodySettings[bodyIdx].Parameters` (an `FMaterialParams`) and
   pumps the FName→texture map into the **body main slot MIC** via
   `SetMaterialParameters(ElementIdx=0, params)` —
   **not** `ElementIdx=SkinMaterialIndex`.
   
   This is the exact static lever §6.3 of `unrealscript_shader_decompile.md`
   identifies as the unexploited high-value lever.

4. **Path D — empirical sentinel relaxation.** Author a Skin mask with
   `R=43` replaced by another non-sentinel value (e.g. `R=0`). The body
   main shader won't trigger its literal-diffuse branch → falls back to
   normal blend, which renders the decal slot's diffuse content through
   the decal slot mask. Probably loses the "non-tinted user art over the
   whole body" UX that AC achieves.

## 8. Confidence levels

| Claim | Confidence | Basis |
|---|---|---|
| AC hooks `MIC_Body_Octane`, not `Octane_GaleFire_MIC` | **High** | BakkesMod SDK struct field literally named `body_mic_override`; AC slot `-3` = "Main Body Decal" which is the body main slot in plugin terminology |
| The body main MIC's `Diffuse` (default white texture) is what fills R=43 zones | **High** | `MIC_Body_Paintable_All` has explicit `Diffuse → idx 2570` default; literal-diffuse sentinel logic is inside the body main shader since that's where the mask is sampled per-fragment for the body's pixels |
| Our donor's `Octane_GaleFire_MIC` is the decal slot MIC, distinct from `MIC_Body_Octane` | **High** | `unrealscript_shader_decompile.md` §2 + RLSDK class hierarchy |
| Editing `FSkinBodySettings::Parameters` in the donor pumps params to the body main slot | **Medium-high** | RLSDK shows `SetMaterialParameters(ElementIdx, params)` is the entry point; the BodySettings.Parameters source feeds it. ElementIdx 0 is convention for body main. Empirical confirmation needed. |
| Editing `Startup.upk`'s `MIC_Body_Octane` would work but is install-wide | **High** | Confirmed: that MIC is the body main slot's MIC and is loaded once globally |

## 9. The concrete experiment to run

**Experiment.** Modify our existing donor UPK so the body-main MIC's
`Diffuse` is set to the user PNG, via `FSkinBodySettings::Parameters`.
Steps:

1. After decrypt+decompress, walk to `UProductAsset_Skin_TA` export.
   Find its `BodySettings` array property.
2. Find the array entry for the body the donor targets (Octane = body
   asset object idx for the corresponding Body export). Inside this
   `FSkinBodySettings`, find the `Parameters` field (an `FMaterialParams`).
3. Inside `Parameters.TextureParameters` (`TArray<FMaterialTextureParam>`),
   append (or overwrite an existing entry of) `{ Name: "Diffuse",
   Texture: ObjectIdx of Force_Body_D }`. The texture object index is
   already in the donor; we don't need to ship a new Texture2D.
4. Re-encode the body chunks with the new array bytes. The array grows
   by ~24 bytes, but we have headroom in chunk 0's recompression budget.
5. Re-encrypt the header. Save to `mods/`.

**Expected outcome.** When RL equips the donor, the engine's
`SetMaterialParameters(elementIdx=0, params)` pump will write
`MIC_Body_Octane.TextureParameterValues["Diffuse"] = Force_Body_D` →
the body main shader's `Diffuse` parameter binds to our hijacked
texture data → R=43 zones in the mask now render the user PNG → 

If the body main MIC's existing `Diffuse` default (idx 2570) is what's
producing the white, replacing it via this pump should change R=43
output to the user content. If not, the issue is in a different
parameter name or different element slot.

## 10. The exact target

- **UPK.** `<donor>.upk` (e.g. `skin_octane_galefire_SF.upk`).
- **Export.** `UProductAsset_Skin_TA` (export class
  `Class.ProductAsset_Skin_TA`).
- **Tagged property.** `BodySettings` (`TArray<FSkinBodySettings>`).
- **Sub-field.** First entry's `Parameters.TextureParameters[]`.
- **FName to add.** `Diffuse` (parent material declares this as a
  `TextureSampleParameter2D` at the body main MIC inheritance level).
- **Texture object index.** The donor export idx of `Force_Body_D`
  (already in the donor — our TFC hijack already redirects it to user
  PNG). Bind by ObjectIndex int32.

If the donor's `BodySettings[0].Parameters.TextureParameters` is empty
or absent, we need to inject the array. This is the same kind of static
serial edit we already do for FName hijacks and mip arrays. Headroom
budget = chunk 0 recompression slack (typically 1-2 KB free per donor).

## 10b. RLSDK confirms the experiment's target struct

`<RLSDK>/TAGame_structs.hpp:381-386` (verified by inspection):

```cpp
struct FSkinBodySettings {            // size 0x0048
    UProductAssetReferenceBody_TA* Body;                              // 0x0000
    FMaterialParams                Parameters;                        // 0x0008  ← body MAIN params
    TArray<FAssociativeMaterialParams> AdditionalBodyParameters;      // 0x0038  ← per-additional-MIC
};
struct FMaterialParams {              // size 0x0030
    TArray<FMaterialTextureParam>  TextureParameters;
    TArray<FMaterialVectorParam>   VectorParameters;
    TArray<FMaterialScalarParam>   ScalarParameters;
};
struct FAssociativeMaterialParams {   // size 0x0038
    UMaterialInterface*            Material;                          // 0x0000  ← which MIC to override
    FMaterialParams                Params;                            // 0x0008
};
```

So the structure is exactly the AC SDK's structure:

| RLSDK | BakkesMod SDK | What |
|---|---|---|
| `FSkinBodySettings::Parameters` (`FMaterialParams`) | `BodyShaderOverride::body_mic_override` | body **main** MIC override |
| `FSkinBodySettings::AdditionalBodyParameters[i]` (`FAssociativeMaterialParams.Material + Params`) | `BodyShaderOverride::chassis_mic_override` | additional MICs (chassis, brakelights, etc.) |

This confirms the experiment: **inject `{Name:"Diffuse", Texture:Force_Body_D_idx}` into `BodySettings[0].Parameters.TextureParameters`** is the exact static analog of AC's `body_mic_override.textures["Diffuse"] = userTex` call. RL's
`UCarMeshComponentBase_TA::SetMaterialParameters(ElementIdx=0, params)`
pumps those bindings into the body main slot's MIC (`MIC_Body_Octane`)
at equip time — that's the MIC whose `Diffuse` is sampled by the
R=43 sentinel branch of the body shader.

## 11. Bottom line

Our pipeline binds user PNG to the **decal slot MIC**. AC's pipeline
binds it to the **body main slot MIC**. The R=43 sentinel logic samples
the body main slot MIC's `Diffuse` — so until we bind user PNG there,
those zones will keep showing stock-white. The static lever is the
donor's `UProductAsset_Skin_TA::BodySettings.Parameters` (the
`FMaterialParams` block), which RL pumps into the body main slot MIC
at equip time. This is the exact static analog of AC's runtime
`SetTextureParameterValue` call.
