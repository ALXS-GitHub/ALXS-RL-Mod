# MIC `Diffuse` binds to `Pepe_Body_D` (import), NOT `Force_Body_D` (local)

> **Date:** 2026-05-16 evening
> **Finding from `docs_rl/diag/diag_mic_params.py`** on the modded
> `mods/Skin_Octane_Stars_SF.upk`. The donor MIC's stored
> `TextureParameterValues` carries:
>
> ```
> [0] CurvaturePack → import[36] = Body_Octane_Curvature_New / Texture2D
> [1] Diffuse       → import[38] = Pepe_Body_D / Texture2D          ← KEY
> [2] Skin          → export[107] = Skin_Octane_Stars_RGB           (local, hijacked OK)
> ```
>
> Plus Parent → `Body_Force_MIC` (local export[66]).

## Why our diffuse hijack was a no-op

Every previous architecture (`Textures → MyDecal_`, single-TFC, dual-TFC, etc.)
patched `Force_Body_D`'s mip array to point at `MyDecal_.tfc`. But the MIC
**never references** `Force_Body_D` — its `Diffuse` parameter binds to
`Pepe_Body_D`, an externally-imported Texture2D whose actual bytes live in
its source UPK's TFC (probably `Body_Octane_SF.upk` → `Textures3.tfc`). The
shader samples Pepe_Body_D when it does `tex2D(Diffuse, uv)`. Our hijack
on `Force_Body_D` writes the user PNG into a TFC slot **no shader sampler
ever reads**.

The mask hijack on `Skin_Octane_*_RGB` worked because that texture **is**
referenced by the MIC's `Skin` parameter (export[107], a positive
ObjectIndex). Hijacking its TFC binding reached the shader's mask sampler.

## Visual reconciliation

This explains every prior in-game observation:

- **"Primary/accent picker zones work."** Mask hijack delivers user mask
  bytes → shader's `Masks`/`Skin` sampler reads them → R=255 zones tint
  primary, etc.
- **"Literal-diffuse zones (R≈43) look mostly white/light."** Shader does
  `lerp(diffuse, primary, mask.r)`. At R=43 the output is ~83% diffuse +
  17% primary. The diffuse is sampled from `Pepe_Body_D` = stock Octane
  body texture (mostly off-white base coat), not the user's PNG. So the
  user sees the stock Octane diffuse with slight tint, not their art.
- **"I see GaleFire when nothing else works."** When BlankSkin was the
  mask target (commits f9981d4 → d0759bc), no hijack reached the shader
  at all, so both Diffuse AND Mask samplers used their stock (Pepe_Body_D
  diffuse, stock RGB-equivalent mask through the parent material chain).

## The fix (proposed, not yet implemented)

Modify the MIC's `Diffuse` `ParameterValue` ObjectIndex from `-39`
(import Pepe_Body_D) to `+<idx of Force_Body_D export>`. The MIC then
binds Diffuse to our local Force_Body_D, whose mip array is already
patched to read from `MyDecal_.tfc` (user diffuse PNG).

Implementation:

1. Parse the MIC export in the modded UPK body.
2. Walk the `TextureParameterValues` array to find the entry with
   `ParameterName = "Diffuse"`.
3. Locate the `ParameterValue` ObjectIndex field byte position
   (entry_start + 56 within the array, after the ParameterName tag +
   value + ParameterValue tag header).
4. Rewrite those 4 bytes from `-39` to the Force_Body_D export index + 1.
5. Re-flow through `rewrite_chunk0_block0` (the MIC export lands inside
   block 0; same recompression budget rule applies).

The change is length-preserving (4 bytes replacing 4 bytes). No header
growth, no AES region size change, no chunked-zlib c_size delta.

## How to verify after the fix

Run `python docs_rl/diag/diag_mic_params.py` after activation. Expected:

```
TextureParameterValues: count=3
  [1] ParameterName = 'Diffuse'  ParameterValue = <positive>
      → export[<N>] = Force_Body_D
```

Then in-game, the literal-diffuse zones (mask R≈43) should show ~83%
user diffuse + 17% primary tint instead of the stock Octane body.

## Open follow-ups

- `CurvaturePack` also binds to an import (Body_Octane_Curvature_New).
  This means our preserved-copy of `Body_Force_Curvature` is also
  unused. Doesn't matter for correctness but explains why removing it
  wouldn't change anything visible.
- The `Skin` parameter binding to `Skin_Octane_*_RGB` proves there IS
  a TextureSampleParameter2D named `Skin` somewhere in the material
  inheritance chain (the parent material parse in
  `docs_rl/startup_upk_body_mic.md` §2 only found `Skin` as a Vector
  parameter, but evidently the actual binding works). Re-investigate
  the parent material parameter graph — possibly `Body_Paintable_Mat`
  has a sub-expression with a TextureSampleParameter2D also named
  `Skin` that the earlier dump missed.
