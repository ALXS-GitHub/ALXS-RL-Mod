# Custom decals — checkpoint 2026-05-16 (stable dual-TFC, mask=`_RGB`)

> User-validated: with the install at commit **`74da7c1`** the user's custom
> AC pack (Yuna Itzy on Octane Stars slot) renders correctly in-game.
> Primary picker tint and accent picker tint apply to the mask's R=255 / G=255
> zones respectively. The only remaining issue is the "literal-diffuse" R≈43
> zones — see §3.

## 1. Architecture (current, working)

Pipeline (`src-tauri/src/commands/decal_swap.rs` + `custom_decals.rs`):

```
Donor:   CookedPCConsole/skin_octane_galefire_SF.upk   (read-only)
         CookedPCConsole/Textures.tfc                   (read-only, ~3.7 GB)
         CookedPCConsole/Textures7.tfc                  (read-only, ~2.6 GB)

Output:  CookedPCConsole/mods/Skin_Octane_Stars_SF.upk  (modded UPK override)
         CookedPCConsole/MyDecal_.tfc                   (sparse, diffuse + preserved aux)
         CookedPCConsole/MyDecal02.tfc                  (sparse, mask only — when user ships a Skin PNG)
```

### Hijack pairs

| FName before rename | FName after rename | Purpose |
|---|---|---|
| `Textures`  | `MyDecal_`  | Diffuse TFC (binds `Force_Body_D`) |
| `Textures7` | `MyDecal02` | Mask TFC (binds `Skin_Octane_<Donor>_RGB`) |

Plus length-preserving renames of `skin_octane_galefire*` → `skin_octane_Stars*`,
`Skin_Octane_GaleFire_RGB` → `Skin_Octane_Stars_RGB`, `Octane_GaleFire_MIC` →
`Octane_Stars_MIC`, thumbnail / template names.

### Texture role classification (`classify` in `decal_swap.rs`)

```rust
if name ends with "_d" or "_diffuse"  → Diffuse
else if name ends with "_rgb"         → Mask
else                                  → Preserve (sparse-copied at original offsets)
```

So in the GaleFire donor: `Force_Body_D` is the diffuse target, `Skin_Octane_GaleFire_RGB`
is the mask target, `Body_Force_Curvature` + `Force_Body_BlankSkin` are preserved.

### Why `_RGB` and NOT `BlankSkin` — the key insight

The donor's `Octane_GaleFire_MIC` carries three `TextureParameterValues`
entries (verified by `sandbox/research/inspect_skin_bodysettings.py`):

```
[0] CurvaturePack → Body_Force_Curvature
[1] Diffuse       → Force_Body_D
[2] Skin          → Force_Body_BlankSkin
```

This is **superficially misleading**: it looks like binding our mask to
`Force_Body_BlankSkin` would be correct. It is not. From
`docs_rl/startup_upk_body_mic.md` §2.3 + `ac_pack_ground_truth.md` §6.1:

- The parent material `Body_Paintable_Mat` declares **`Skin` as a
  `MaterialExpressionVectorParameter`** (a flat RGBA tint), NOT a
  `TextureSampleParameter2D`.
- The actual texture-mask parameter is named **`Masks`**
  (TextureSampleParameter2D), and at the skin-MIC level it inherits to
  `Skin_Octane_<Donor>_RGB` despite the suffix.
- Binding a Texture2D to a VectorParameter slot in a MIC is a **no-op** —
  the engine accepts it without error but the shader never samples it.

Three commits between f9981d4 and d0759bc each retargeted the mask onto
`BlankSkin` thinking it would fix things. None of them did. Hijacking
`BlankSkin` writes the user's mask to a slot the shader never reads, so
in-game the player sees pure donor textures (GaleFire). The revert
(commit 74da7c1) restored 59fb130's mask=`_RGB` targeting and the
in-game render came back.

## 2. Body block 0 patch — mip array rewrites

For each swapped Texture2D, `rewrite_chunk0_block0` patches three i32/u32
fields per mip entry:

- `ElementCount` ← raw BC3 byte count for the mip
- `BulkDataSizeOnDisk` ← chunked-zlib-wrapped byte count
- `BulkDataOffsetInFile` ← offset into the corresponding `MyDecal*.tfc`

Then the modified 128 KB block is recompressed with `flate2` level 9 and
zero-padded to the original `c_size` (RL refuses size deltas — empirically
confirmed in `palettes.rs` for the analogous TAGame.upk case).

## 3. Outstanding issue — literal-diffuse zones (task #99)

When the user authors an AlphaConsole-style Skin PNG with R≈43 (`#2B0000`)
pixels to mark "show literal diffuse here", those zones in-game **do not**
show pure literal RGB. They show ~83% diffuse + ~17% primary-picker tint.

Per the reconstructed shader in `docs_rl/rl_decal_shader_re.md` §6:

```hlsl
primaryWeight   = mask.r * (1 - mask.a)
secondaryWeight = mask.r * mask.a
windowsWeight   = mask.b
bodyTint = lerp(diffuse, primary,   primaryWeight)
bodyTint = lerp(bodyTint, secondary, secondaryWeight)
bodyTint = lerp(bodyTint, windows,   windowsWeight)
```

At `mask.r = 43/255 = 0.169` → primaryWeight = 0.169 → 17% palette tint.
The R=43 sentinel is **author convention** from RL-Designer
(`docs_rl/rl_designer_threshold_origin.md`), not a real RL shader branch.
True "literal diffuse" needs `R ≈ 0 AND B ≈ 0`.

Recommended fix (task #99): inside `png_to_bc3_mips` for the mask path,
remap input pixels with `R ∈ [25, 60]` and low G/B to `(R=0, G=0, B=0,
preserve_alpha)` before BC3 encoding. AC convention packs continue to
work as authored; native shader behaves correctly.

## 4. FullColor mode (preserved, not in-game-tested)

`DecalMode::FullColor` (commit dede7d5) synthesizes a constant `(R≈41, A=255)`
BC3 mask regardless of user Skin PNG. Should produce literal RGB across the
whole body modulo small tint at R=41. Not yet validated in-game — the user's
test path is Auto/Paintable mode for now.

## 5. Sources / cross-references

- `docs_rl/startup_upk_body_mic.md` — parent-material parameter graph,
  proves `Skin` is a VectorParameter and `Masks` is the real texture sampler.
- `docs_rl/ac_pack_ground_truth.md` — BakkesMod SDK contract,
  AC Pattern A vs B translation rules, MainBodyOnly mask byte analysis.
- `docs_rl/rl_decal_shader_re.md` — DXBC reconstruction,
  IEEE-754 byte-scan ruling out a 0.169 / 0.18 literal-diffuse sentinel.
- `docs_rl/rl_designer_threshold_origin.md` — provenance of the AC `#2B0000` convention.
- `docs_rl/diag/diag_*.py` — empirical TFC binding / mip layout dumps.

## 6. How to verify the current state (Python diag)

```bash
python docs_rl/diag/diag_modded.py
```

Expected for the modded UPK:

| Texture | TFC | mip 0 offset | size_disk |
|---|---|---:|---:|
| `Body_Force_Curvature` | `MyDecal_` | `0x9ea2b0` (original, sparse-preserved) | 408516 |
| `Force_Body_BlankSkin` | `MyDecal_` | `0x25b2c15` (original, sparse-preserved) | 29565 |
| `Force_Body_D` | `MyDecal_` | `max_orig` (= user diffuse base) | per encoding |
| `Skin_Octane_Stars_RGB` | `MyDecal02` | `max_orig` in MyDecal02 (= user mask base) | per encoding |

If `BlankSkin`'s TFC is anything other than `MyDecal_` (i.e. an unexpected
name like `MyDecal_3` or `MyDecal02`), the regression has returned.
