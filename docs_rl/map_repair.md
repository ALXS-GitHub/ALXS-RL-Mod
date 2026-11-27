# Custom map repair (moved arena objects)

## Symptom

Custom UDK maps (Workshop / bakkesplugins / Lethamyr) loaded on the
2026-11 build (`++Prime+Update60.1`, package licensee 34) show surfaces as a
**blue and grey checkerboard** (walls, skies…), although they worked before
Easy Anti-Cheat (April 2026). `Launch.log` reports nothing.

## Cause (verified 2026-11-27)

- Maps reuse stock arena objects by **import path**, e.g. Minigolf's walls
  use `EuroStadium_P.FutureTech.Materials.ForceField_Mat`, skies use
  `Park_P.Proto_World01.SM.SkySphere01`.
- Psyonix updates **move or rename** such objects. `ForceField_Mat` is no
  longer in `EuroStadium_P` (Mannfield now has `ForceField_HexGage_Mat`, a
  team-coloured hex shield) but still exists, unchanged, in
  `CHN_Stadium_Cage.upk` and `TutorialTest_K.upk`. `SkySphere01` moved to
  `Startup.upk` (top-level package `Proto_World01`, always loaded).
- A missing import resolves to nothing; the surface renders with
  `EngineMaterials.DefaultMaterial`, whose `DefaultDiffuse` is the blue/grey
  checker (extracted from `Startup.upk`). No warning is logged.
- Ruled out: the Workshop Textures pack (present and loaded; removing it
  changes nothing), shader compiling (RL cannot compile at runtime; the
  base materials maps instance — `EngineDebugMaterials.TexturePaint_2Tex_Color`
  etc. — are still cooked in `Startup.upk` with their parameters and
  `bUsedWithStaticLighting`), the licensee 33/34 container changes.
- Not repairable by re-pointing: objects that only exist in the UDK pack
  (uncooked, no shader map in `RefShaderCache`), e.g.
  `Engine_MI_Shaders.M_Shader_SimpleTranslucent` (Yoshi Circuit) and
  `MapTemplates.Sky.M_Procedural_Sky_*`; packs from map makers
  (`LethNeonHeightsPack`, `4v4MapPack`); packages Psyonix removed
  (`grass_textures`, `PostProcess`, `FXActorEvents`).

## Repair (`maps::repair`)

Pure function of (original package, mapping version) → identical bytes for
every player. Uncompressed UDK packages only (licensee 0, no chunks).

- For each rule `{from, to, class}` matching an import (path compared
  case-insensitively, class exact): the import keeps its index; its outer
  chain is rebuilt from reused or appended `Core.Package` imports and its
  object name re-pointed. A target in an always-loaded file (`Startup`,
  `Engine`, `TAGame`, `Core`) is written top-level (`Proto_World01.SM…`);
  otherwise the file name becomes the top package (`CHN_Stadium_Cage.FutureTech…`),
  which the game loads on demand.
- New names and imports are **appended** to their tables. Every absolute
  offset after the insertion points is shifted: summary
  (`TotalHeaderSize`, import/export/depends/guids/thumbnail offsets, name
  and import counts, last generation's name count), each export's
  `SerialOffset`, the thumbnail table, and **inline bulk data**: UE3 bulk
  headers (`flags u32, count i32, size i32, offset i32`) inside export data
  store their own absolute file offset — for inline data `offset ==
  header position + 16`, which is how they are found. Missing this step
  turns every texture white (first prototype).
- Verified on 16 library maps: re-auditing the repaired files resolves all
  45 moved references; every export is byte-identical to the original once
  bulk offsets are neutralised.

Storage: the original package is never modified; each repair is
`repaired/v<N>.upk` + `repaired/v<N>.json` (fixes, source and result
SHA-256) in the map folder, so several mapping versions can coexist and the
user picks the variant (original or vN) per install / host.

## Building a new mapping version

1. Index the game: `cargo run --release --example asset_probe -- <keys.txt> index <CookedPCConsole> game_index.tsv`
   (~10 s, ~300k objects; packages whose AES key is unknown are skipped).
2. Dump each map's imports: `asset_probe <keys.txt> imports <map.upk> > <name>.imports`.
3. `python tools/map_repair_audit.py game_index.tsv <imports_dir> <CookedPCConsole> v`
   lists moved / same / gone references; `… rules` prints the moved ones as
   rules. Review `same` entries by hand (renamed objects).
4. Copy the previous `src-tauri/src/maps/repair_mappings/vN.json` to
   `v(N+1).json`, add the rules, add it to `repair::mappings()`. Never edit a
   released version: friends with that version must produce the same file.
