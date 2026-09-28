# Recolouring items (experimental)

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
