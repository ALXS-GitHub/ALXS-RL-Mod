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
properties; `cargo run --example recolor_probe -- <package> <hue> <out>`
writes a recoloured copy.
