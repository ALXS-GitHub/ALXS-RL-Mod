# Recolouring items

Case study: `Boost_AlphaReward_SF.upk` (Alpha Boost, gold only), 2026-09-28.

Where its gold comes from:

| Export | Colour data | Changeable file-only |
|---|---|---|
| `AlphaReward_MIC` (MaterialInstanceConstant) | `VectorParameterValues`: `Inner_Color` (1.5, 0.8, 0.2, 6), `Outer_Color` (1.5, 0.8, 0.2, 3) | yes: shader uniforms |
| `LiquidGold_02_MAT` (Material) | expressions compiled into the game's shaders | no |
| `ParticleModuleColor` / `…ColorOverLife` / `…ColorScaleOverLife` | `RawDistributionVector.LookupTable` = `[min, max, r, g, b, …]` (baked curve) | yes |
| `DistributionVectorParticleParameter` "CoreColor" | `Constant` (2.5, 1.0, 0.125) — default of a particle parameter; the package's `ParameterDispenser_X` are empty, so nothing overrides it | yes (to confirm in game) |
| Textures `Dust_T`, `Cloud_T`, `Noise_Cones01_D` | grey patterns | not needed |

Notes:
- Distribution objects start their property block 8 bytes into the export
  (4 for most objects).
- Only the distributions referenced by colour modules are colours:
  `ParticleSize` is a `DistributionVectorParticleParameter` too.
- `upk::recolor::colorize` keeps each colour's max and min (value and
  saturation, HDR intensity included) and moves its hue; greys stay grey,
  so a table's `[min, max]` header stays valid.

Tooling: `cargo run --example color_probe -- <package>` dumps the colour
properties; `cargo run --example recolor_probe -- <package> <#rrggbb> <out>`
writes a recoloured copy.

## Colour painted into textures: the exact mode (2026-09-29)

Case study: `WHEEL_AlphaRim_SF.upk` (Alpha wheels). `Wheel_Alpha_MIC`
(parent `Wheel_Master_Mat`) binds `RimDiffuse = Alpha_D`, a **gold** DXT1
2048² texture (mean RGB 101/71/0), tinted by `RimColor` (0.75, 0.67, 0.48)
and `CustomColor` (0.65, 0.49, 0.25). The game renders roughly
`texture × material colour`, so moving the material colours alone
(tint mode) multiplies the target by gold: red turns orange, blue and green
collapse (the gold has no blue), black keeps golden highlights.

Exact mode (`SwapRequest.colorMode = exact`, default in the UI; swaps saved
before stay `tint`):
1. `swap::texture_color::neutralize`: colour textures (bound to a
   `*Diffuse*` parameter or named `*_D`, not `*_N` / `*_RGB`, mean
   saturation ≥ 0.12) become neutral — each pixel's HSV value as a grey,
   alpha kept — and go to our own texture cache through
   `decals::pipeline::build_swap` (cache name `Ax<fnv32 of the owned
   package>` truncated to the stock cache length, so re-swapping the same
   item overwrites it; ~6 MB for the Alpha wheels, written as a `NewFile`
   entry of the swap, deleted with it).
   When the material has no colour to tint the texture (`Hat_AlphaHat_SF`:
   `GoldHat_D`, no `VectorParameterValues`), the target is baked into the
   texture instead.
2. `upk::recolor` exact: material and particle colours take the target's
   hue and saturation **in linear light** (the hex is sRGB), value scaled by
   the target's; grey material colours move too (they tint neutral
   textures), grey particle colours stay.

Checked in memory on 2026-09-29 (Alpha wheels red, Alpha hat blue, Alpha
Boost, Proto wheels): package size unchanged, every mip of the new cache in
bounds. In-game test pending.

Preview (`swap_color_preview`): the main colour texture at ≤ 256 px ×
the main material colour (most saturated one), linear light, no lighting.

Tooling: `item_color_probe <package>` (textures, bindings, material
colours), `texture_dump <package> <out>` (PNG of each texture),
`exact_color_probe <cooked> <package> <#rrggbb> <out>` (whole pipeline in
memory + previews).

## Official paints (swap::paint)

- Paint database: `TAGame.upk`, export `PaintDB` (`PaintDatabase_TA`,
  package `ProductPaint`): `Paints` = object refs, index = the game's
  PaintID (1 Crimson, 2 Lime, 3 Black, 4 Sky Blue, 5 Cobalt, 6 Burnt Sienna,
  7 Forest Green, 8 Purple, 9 Pink, 10 Orange, 11 Grey, 12 Titanium White,
  13 Saffron, 14–18 metals, 19–29 "… Glow"). Each `ProductPaint_TA`:
  `Label`, `Colors[12]` (static array, linear RGBA, indexed by
  `EPaintColorVariant`: Primary, LightAccent, DarkAccent, Emissive,
  DeEmissive, Complementary, Balanced, Tertiary, Additive, Unused3…).
- An item: one or more `ProductAttribute_PaintSettings_TA` (defaults:
  `PaintParameterName=CustomColor`, `PaintType=Primary`,
  `PaintEmissiveMultiplier=1`, `bPaintParticles=false`), with
  `MaterialGroups[].Materials` (the MICs it paints), `IncludePaintIDs`,
  `UnsupportedPaints`, `PaintAdditionalParameters[]` (`ParameterName`,
  `PaintVariant`, `bEnabled`) and `PaintsToOverride[]` → per paint,
  `ProductOverride_ParticleSystemColorParameter_TA.ParameterOverrides[]`
  (`PaintParameterName`, `PaintType`, `Paint` or `CustomColor`).
- The game sets `Colors[PaintType] × multiplier` on the named parameters
  at runtime (decoded from `ProductAttribute_Painted_TA`). We bake the same
  values into the defaults: MIC `VectorParameterValues`,
  `MaterialExpressionVectorParameter.DefaultValue` and particle
  `DistributionVectorParticleParameter.Constant`. Items whose parameter is
  only inherited from another package's material (e.g. `WHEEL_Brink`)
  cannot be painted file-only.
- Tooling: `cargo run --release --example paint_apply_probe -- <CookedPCConsole> <PaintID> <item.upk>…`,
  `examples/paint_probe.rs` (generic dumper).
