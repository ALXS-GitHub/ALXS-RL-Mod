# Custom Decals — Final Plan (2026-05-15 synthesis, REVISED)

Synthesis of 9 parallel agent investigations + Python static analysis + actual MIC dump. State of the art was: 0 public file-only literal-RGB solutions post-EAC. This document is the concrete path.

## TL;DR (REVISED — simpler than initially thought)

**The current dual-TFC pipeline IS the correct architecture.** The "literal RGB FullColor mode" requires only a **content change**, not a UPK structure change.

What we discovered from dumping `Octane_GaleFire_MIC` in the donor UPK:
```
TextureParameterValues (3 entries):
  [0] CurvaturePack → Body_Force_Curvature
  [1] Diffuse       → Force_Body_D          ← we control this via Textures→MyDecal_ hijack
  [2] Skin          → Force_Body_BlankSkin  ← we control this via Textures7→MyDecal02 hijack
```

The skin MIC renders at the body's SkinMaterialIndex slot (replaces body MIC there). When we put user's PNG in the diffuse hijack AND a constant `(R=0, G=0, B=0, A=255)` in the mask hijack, the paint shader's primary/accent tint contribution becomes zero, leaving only the literal Diffuse texture visible.

This is exactly AlphaConsole's "FullColor Esports" pattern (validated empirically against Dominus `EsportsFullColor/MainBodyOnly.png` which is literally all (0,0,0,255)).

## Constraint reset — what changed today (2026-04-28 onwards)

- EAC was **reintroduced** in Rocket League on 2026-04-28. BakkesMod is officially discontinued. All BakkesMod-based decal mods (AlphaConsole, CustomDecalLogos, etc.) work offline-only.
- Our file-only architecture is now the **only viable path** for online cosmetic overrides. RLPeak (file-only catalog-only) is the only public competitor and doesn't support user PNGs.
- No public release has solved literal-RGB custom decals file-only. We are ahead of the state of the art.

## The mechanism (validated against RLSDK + binary strings + AC pack forensics)

When a player equips decal X:
1. `UCarMeshComponent_TA::SetLoadout(loadout)` loads `UProductAsset_Skin_TA` for the equipped decal
2. Engine reads `Skin->BodySettings[bodyIdx].Parameters` (an `FMaterialParams` struct)
3. For each entry in `FMaterialParams.TextureParameters`: `BodyMIC.SetTextureParameterValue(FName, Texture2D*)`
4. Same for `VectorParameters` and `ScalarParameters`
5. Color picker values are then pumped via `SetMaterialColorParams` — UNLESS `ForcedTeamColors` / `ForcedCustomColor` override them
6. Final pixel = native paint shader output, sampling whatever textures we bound

**Key insight:** `Diffuse` is sampled **before** the paint blend. If we override `Skin` (the paint mask) with a (0,0,0,255) texture, the paint blend contributes zero, leaving only the literal `Diffuse` visible.

**AlphaConsole does this at runtime.** We replicate it statically in the cooked donor UPK.

## Implementation plan

### Phase 1 — done

Python `inspect_skin_bodysettings.py` analysis confirmed:
- `Octane_GaleFire_MIC` (export[67] in `skin_octane_galefire_SF.upk`) has 3 TextureParameterValues entries
- `Diffuse` binds to `Force_Body_D` (export[96]) — already hijacked by our pipeline
- `Skin` binds to `Force_Body_BlankSkin` (export[95]) — already hijacked by our pipeline
- Parent material = `MIC_Body_Paintable_All` (import[20])

`ProductAsset_Skin_TA` export is 68 bytes — too small to hold `BodySettings.Parameters`. The MIC's own TextureParameterValues ARE the param source (no runtime override pump from skin asset side, since BodySettings is empty/minimal). So **the static MIC is the truth**, and our existing TFC hijack on those textures IS the static AC equivalent.

### Phase 2 — generate neutralization BC3 — done

Script `generate_neutralization_mask.py` confirmed:
- 2048×2048 BC3 of constant `(R=0, G=0, B=0, A=255)` is literally 16-byte block `ffff0000000000000000000000000000` repeated `262144 × num_mips` times
- Full mipchain (10 mips, 2048→4) = 5.59 MB total
- Decoded sample roundtrips: `(0, 0, 0, 255)` ✓

### Phase 3 — add `FullColor` mode to `decal_swap.rs`

In `src-tauri/src/commands/decal_swap.rs`:
1. Add `pub mode: DecalMode` to `DecalSwapPlan` (enum: `Paintable | FullColor`)
2. In `run_swap`, when mode is `FullColor`:
   - For the diffuse sparse TFC: write user's PNG → BC3 (unchanged)
   - For the mask sparse TFC: write the constant `(0,0,0,255)` BC3 (a simple loop emitting the 16-byte block per 4×4 block per mip)
3. Skip the user-provided mask BC3 encoding entirely in FullColor mode

### Phase 4 — UX

Add a toggle in the install dialog: "Paintable (picker colors)" vs "Full-Color (literal RGB)".

When user picks an AlphaConsole pack:
- If pack has only a `Skin` PNG (Pattern B): force Paintable mode
- If pack has a `Diffuse` PNG (Pattern A FullColor): force FullColor mode + use the included MainBodyOnly mask OR our generated constant

When user picks a custom PNG (not AC pack): default FullColor mode (the user typically wants literal RGB).

## Risks + mitigations

| Risk | Likelihood | Mitigation |
|---|---|---|
| `BodySettings.Parameters` is empty in GaleFire (forces array growth) | Medium | Use a different donor that has populated params; or implement array growth (we already track body offsets) |
| Body MIC has no `Diffuse` parameter (only `Skin` etc.) | Low | RL.exe binary strings confirm `Diffuse` is a recognized FName param. AC's FullColor packs use it explicitly. |
| Mask=(0,0,0,255) doesn't fully neutralize the shader | Medium | Validated by AC pack forensics: Dominus `MainBodyOnly.png` IS exactly (0,0,0,255). Will validate via Python sentinel grid before in-game test. |
| EAC false-positive on file edits | Low | RLPeak ships exactly this style of edit, no reports of EAC issues. Our previous EAC alert was on `psapi.dll` (process scanning) — unrelated to file edits. |

## Strategies B & C (kept as fallbacks)

**Strategy B — Startup.upk override** (hijack `Octane_Body_D` TFC binding directly)
- Confirmed: `MIC_Body_Octane`, `Octane_Body_D`, `Body_Paintable_Mat` all live in `Startup.upk` (export[786], [2501], [261])
- Place modified Startup.upk in `mods/` with diffuse texture's TFC name hijacked → custom RGB BC3 in sparse TFC
- Plus equip any decal with mask=(0,0,0,255)
- **Drawback:** affects body globally when decal equipped (becomes "full skin" not just decal slot UX)

**Strategy C — `UProductAttribute_Painted_TA` removal** (speculative, cheap to test)
- Binary analysis found this discrete attribute marks products as "paintable"
- Hypothesis: removing the attribute → engine skips paint-shader pass → raw diffuse
- Single-flag, file-only, EAC-invisible
- Needs experimental confirmation in-game; high reward if it works

## Painted attribute removal — investigated, dead end (2026-05-15)

Binary analysis hypothesis: `UProductAttribute_Painted_TA` marks paintable
products; removing it might skip the paint pass. **Disproven by Python scan**
(`inspect_painted_attribute.py`): the FName `ProductAttribute_Painted_TA`
appears in exactly 1 UPK across the entire 17,301-file CookedPCConsole tree:
`TAGame.upk` (the class definition itself). No skin/body UPK references it
as an attribute instance. RL stores paintability dynamically via runtime
class instantiation or central product DB lookup, not as a per-UPK flag.

The flags `bPaintBody` and `bPaintChassis` exist in ~148 UPKs (topper/antenna/
boost) but live inside `UProductAttribute_PaintSettings_TA`, not on the skin
asset. They control whether paint applies to the body slot — not whether the
paint *shader* runs. Removing or flipping them would deactivate paint
application, but the shader still produces white-base fallback if it has no
tint to apply — not literal RGB. Confirmed not a viable shortcut.

## DiffuseOverrideParameter — investigated, not exploitable file-only

Binary string `DiffuseOverrideParameter` is a UE3 debug-viewmode global
uniform that globally overrides every pixel shader's diffuse output.
Gated by `bAllowDebugViewmodesOnConsoles`. The flag would need to be set
at process boot via UnrealScript or DLL injection — both impossible
file-only post-EAC. Documented for completeness; not actionable.

## Dead ends — confirmed and documented

- **Material reparent** (e.g. → `TexturePaint_2Tex_Color`): GaleFire has only ONE Material import (`MIC_Body_Paintable_All`, its current parent). Any reparent requires header growth under AES-ECB. Engine-debug materials aren't built against RL's deferred renderer shader cache — they may not render at all. Reparent would lose all paint shading.
- **Compiled shader edit**: DXBC bytecode, undocumented `FMaterialShaderMap` serialization, hash-validated on load. Off-limits.
- **`bForceDiffuse` / `bNoPaintShader` etc.**: do not exist anywhere in RL.exe strings. There is no global bypass flag.
- **Custom map asset injection**: maps can't ship custom shaders, only ship MICs parented to engine-cooked materials. Vehicle decal pipeline is initialized at game start independent of the level — map asset injection doesn't reach the vehicle render path.

## Sources

- `docs_rl/alphaconsole_packs_forensics.md` — AC pack JSON schema + MainBodyOnly mask pixel analysis
- `docs_rl/lethamyr_maps_approach.md` — why map asset path doesn't help
- `docs_rl/material_parent_candidates.md` — exhaustive Material catalog + reparent dry-run
- `docs_rl/rl_binary_analysis.md` — RL.exe strings + `UProductAttribute_Painted_TA` finding
- `docs_rl/unrealscript_shader_decompile.md` — RLSDK class layouts + the FSkinBodySettings::Parameters lever
- `docs_rl/ecosystem_survey_2026.md` — confirms no public file-only post-EAC solution exists
- `sandbox/research/sdks/RLSDK/` — full RLSDK headers cloned for reference

## Next concrete actions (sequential)

1. ✅ Done: agent + analysis sweep
2. Write `sandbox/research/inspect_skin_bodysettings.py` → find FMaterialParams in GaleFire UPK, report current bindings + byte offsets
3. Write `sandbox/research/test_neutralization_mask.py` → generate the (0,0,0,255) BC3, verify pixel values
4. Prototype `sandbox/research/strategy_a_dry_run.py` → produce patched GaleFire UPK in `sandbox/research/test_output/` with retargeted ObjectIndex bytes (no game test)
5. If Phase 4 dry-run validates roundtrip, port to Rust as `commands/decal_swap_fullcolor.rs`
6. UI mode toggle in `custom_decals` Tauri command
