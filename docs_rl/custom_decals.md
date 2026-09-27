# Custom Decals — Research Notes

> **Goal** : let the user import their own PNG image and ship it as a swappable RL skin, AlphaConsole-style but without injection. Optionally batch-convert the user's existing AlphaConsole PNG packs (~103 PNGs in 17 packs on disk) into our format.

## Why AlphaConsole's approach doesn't transfer

AlphaConsole's data layout (e.g. `data/acplugin/DecalTextures/Born To Be Itzy/Octane/body_diffuse.png`) was designed around its **runtime injection** :

- PNGs stayed as raw PNG files on disk
- A JSON `Template.json` per pack told the plugin which BodyID to apply to
- AlphaConsole's injected code hooked the body material at render and substituted the texture parameter live in process memory
- Color region encoding (`#FF0000 with 0% opacity = primary slot`, `100% = secondary`) was AlphaConsole's INTERNAL convention so its compositor could apply the player's team colors on top

None of that transfers to a no-injection model. RL natively expects a `.upk` skin asset, not a PNG. We need to ship `.upk` files.

## How RL actually handles skins / decals natively

### The skin file's own self-documentation (gold)

Decrypting `skin_aa_flames_tierall_SF.upk` (the file every rlpeak decal targets) revealed a **developer comment** embedded in the asset by Psyonix :

```
"0 - TeamColor; 1 - CustomColor; 2 - PaintedColor; 3 - Chassis_EmissivePartsColor (LOCKED)"
```

This is the **canonical color-mode legend**. RL composes a body's final look from FOUR color sources, indexed 0–3 :

| Mode | Source |
|---:|---|
| 0 | TeamColor — the team's palette color (Blue / Orange in match) |
| 1 | CustomColor — the player's custom primary / secondary picked in garage |
| 2 | PaintedColor — the "painted" drop variant (Crimson, Sky Blue, Saffron, etc.) |
| 3 | Chassis_EmissivePartsColor — LOCKED, special chassis emissive (rare items) |

And the channel-mask architecture is confirmed by the strings :

```
Emissive Color (BlueMask)
Emissive Color (GreenMask)
Emissive Color (RedMask)
```

→ Each RGB channel of the skin's mask texture marks a region. R / G / B channel each ties to one of the color modes above. The shader does the compose at render time.

### Body-side properties referenced

From decrypted `Body_Octane_SF.upk` :

```
PaintMaskInRGB                     ← THE skin's mask texture parameter
Team1_ColorLookup / Team2_ColorLookup   ← per-team color lookup tables
TeamColorScriptedTexture_TA        ← runtime-composed texture (built by shader)
SkinMaterialIndex / SkinParameters  ← material slots for skin overrides
Skin_Octane_Flames                  ← named skin reference example
Pepe_Body_BlankSkin                 ← the universal "blank skin" base
Body_Octane_PremiumSkin_SK          ← premium skin slot
Import_Body_BlankSkin               ← import shadow of the default
DiffuseColor / TeamColor / SpecularColor
HeadlightColor / TailLightColor / RimColor
ChassisPaintColor / Chassis_EmissivePartsColor / Chassis_EmissivePartsColor_Selector
DefaultColor
```

### Other useful strings inside the native skin file

```
"Body Diffuse for AO, or if you're using a trim sheet, input the DiffuseParts map,
 with AO in the Alpha Channel"

"Default is set to use UV0 (0/Off). Turn on to use UV2 (1/On). --Changes Texture
 Parameters: Diffuse (RGB only), Skin and Skin_M, so if you intend to use UV2, be
 sure all your textures for these are also using UV2"

"Input the RGB color for the plastic parts. If using a tile sheet, make sure the
 AO is in the Alpha of the PlasticColor map."
```

These confirm :

- The skin asset has **multiple texture parameters** : a Diffuse map, a Skin map, a Skin_M (mask) map, plus optional trim/PlasticColor maps
- **Alpha channel of Diffuse = ambient-occlusion (AO)**
- The mask `Skin_M` is what drives the color modes
- There's UV0 vs UV2 selection (likely so the diffuse uses one UV set and the skin uses another)

**The native RL composition recipe at render time** :

1. **Body base texture** — chrome / metal / whatever material the body ships with
2. **PaintMaskInRGB** — a mask texture, RGB channels mark coloring zones
3. **Skin texture (optional)** — the decal pattern overlaid on the body
4. **Team1_ColorLookup / Team2_ColorLookup** — palette lookup table per team
5. **Shader composes them at runtime** :
   - `final_color = lerp(base, primary_palette_RGB, skin_R_channel)`
     `+ lerp(base, accent_palette_RGB, skin_G_channel)`
     `+ skin_diffuse_layered`
   - (the exact lerp formula needs shader inspection, but the principle is RGB-channel masking driving palette pickups)

**Bottom line :** RL's skin file is *not* a PNG. It's a UPK containing :

- A compressed diffuse texture (the decal pattern, DXT/BC compressed)
- A compressed mask texture (the RGB mask defining colorable zones)
- Material parameters
- Mipmaps for each

## What rlpeak does

rlpeak's catalog ships pre-cooked skin UPKs — they already have valid diffuse + mask + material set up. That's why their items "just swap" : we drop the UPK in `CookedPCConsole/<target>` and RL composes correctly.

What rlpeak *doesn't* offer : user-supplied images. Their 6961 skins are all curator-picked / extracted from RL itself or community art that was already shipped as UPKs.

## Implementation plan for custom user-PNG decals

### Phase 1 — Template UPK identification

- Pick a representative skin UPK (e.g. `skin_10x_SF.upk` or one of the universal rlpeak skins)
- Decrypt + inspect with umodel to identify :
  - The texture asset's byte offset inside the UPK
  - The exact pixel dimensions expected (likely 1024×1024 or 2048×2048)
  - The texture format (almost certainly BC3/DXT5 with mipmaps)
  - The mask texture's offset (if there's a separate mask channel or if it's an alpha channel)
- Save the smallest valid skin UPK as a **template** in our app's bundled resources

### Phase 2 — PNG → UPK converter (Rust)

Crates :

- `image` for PNG decoding
- `image_dds` or `texpresso` for BC3 compression with mip generation
- Custom UPK reading/writing — we patch a template, we don't author from scratch

The converter takes :

- `diffuse.png` (the colorful pattern — what the user draws)
- *Optional* `mask.png` (the RGB mask defining where primary / secondary / third color apply — if user wants stockable team-coloring zones)

…and produces a valid skin UPK by :

1. Loading the template UPK bytes
2. Decoding the user PNGs
3. BC3-compressing each PNG with mipmaps
4. Splicing the new compressed bytes into the template at the texture offsets
5. Patching any size / offset metadata in the UPK header so RL accepts it

### Phase 3 — Engine integration

- Reuse the existing items engine (`commands::items`)
- New `LocalItem` whose `source_files_dir` contains the **generated** UPK
- Source is "custom" (not rlpeak), so add a `MapSource::CustomDecal` variant

### Phase 4 — UI

- New section in `/items` : **Decal Designer**
- Drop-zone for PNG diffuse + optional PNG mask
- Pack name + which car bodies it applies to (using a body-ID picker)
- Preview thumbnail
- "Generate & add to library" button → runs the converter → adds as a regular item

### Phase 5 — AlphaConsole migration tool

- Scan `%APPDATA%\bakkesmod\bakkesmod\data\acplugin\DecalTextures\<Pack>\<Car>\body_diffuse.png`
- For each pack, batch-run the converter
- Pre-fill the pack name from the folder name
- The user's 103 PNGs become ~17 ready-to-activate decals in our library

## Updated understanding (after deep investigation 2026-05-14)

### rlpeak's actual trick

The community's existing decal "modding" works by **renaming RL's own skin
UPKs**. The 6 961 rlpeak "skins" are literal copies of files already in
`<RL>/.../CookedPCConsole/`, served from rlpeak's CDN, downloaded into
`<RL>/.../CookedPCConsole/` under a *different* name (typically
`skin_aa_flames_tierall_SF.upk`, the universal decal slot). When RL loads
that override, it sees whatever stock skin rlpeak chose to repackage.

Proof: byte-comparing `WHEEL_Vortex_SF.upk` downloaded via rlpeak with the
stock file in the user's RL install → **byte-identical**. They modify
nothing; they only rename.

This means rlpeak **cannot ship user-made art**. Their universe is the
existing RL skin library.

### Texture storage: TFC vs inline

RL ships seven `Textures*.tfc` files (~21 GB total) inside `CookedPCConsole/`.
Most skin UPKs reference texture pixel data **by offset+length into a TFC**.
The UPK itself is ~30–50 KB of metadata.

But **some** skin UPKs have the texture **inlined** (the `FByteBulkData`
flag bit `BULKDATA_StoreInSeparateFile (0x40)` is clear). Example:
`Skin_bartees_SF.upk` is 346 KB on disk; its body decompresses to 1.19 MB,
of which ~352 KB at body offsets `0x008000..0x060000` is high-entropy
texture data (verified via per-32 KB byte-distribution analysis — that
region has <10% zeros and >220 distinct bytes per block).

### Path forward for actual custom decals from user PNGs

A self-contained PNG → UPK pipeline:

1. **Pick a template** — `Skin_bartees_SF.upk` or any other skin UPK whose
   body has a ~350 KB inline texture region. Decompress its body via the
   chunked-zlib format already implemented for palette
   (`commands::palettes`).
2. **Locate the texture bytes** precisely by parsing the export table down
   to the `Texture2D` object, reading its `Mips: TArray<FTexture2DMipMap>`,
   and recording each mip's `(BulkDataFlags, ElementCount, SizeOnDisk,
   OffsetInFile)` + the inline data range.
3. **Convert the user PNG** to BC3 (DXT5) with the full mipmap chain at
   the template's exact dimensions (1024² typically). Crates: `image` for
   the PNG decode, `image_dds` or `texpresso` for BC3 encoding.
4. **Splice** — overwrite each mip's inline byte range with our newly
   compressed BC3 bytes. Update the `ElementCount` and `SizeOnDisk` fields
   if the new sizes differ (BC3 of the same dimensions is deterministic in
   size though, so they should match exactly).
5. **Recompress** the modified zlib block, allow growth via the same chunk-
   header patch we use for palette (`patch_chunk_header_for_grown_block`).
6. **Save** as `skin_aa_flames_tierall_SF.upk` (or whichever target slot)
   in `<RL>/.../CookedPCConsole/mods/`. Install via the existing items
   engine.

The `Body` PNG in the user's AlphaConsole pack (`Diffuse` / `1_Diffuse_Skin`)
becomes the main texture. The `Skin` PNG (or `TrimSheet`) maps to the RGB
mask if the template's body has a separate Skin_M texture; otherwise it's
discarded with a UI warning.

### Effort estimate (revised)

- ~~Phase 1 (template ident)~~ — partially done; pick `Skin_bartees_SF.upk`.
- Phase 2 (texture-byte locator): 1 day. Parse FByteBulkData chain in the
  decompressed body, validated against the byte-entropy hint.
- Phase 3 (PNG → BC3 + mipmaps): 1-2 days. Wire `image` + DDS encoder.
- Phase 4 (splice + chunk patch): 0.5 day. Reuse `palettes` patch helpers.
- Phase 5 (UI integration): 0.5 day. Hook the activate button on
  `DiscoveredCustomDecal` cards into the items engine.

**Total: ~3-4 focused days** for a working PNG → custom UPK pipeline.

## Open questions to answer during Phase 1

- [x] Where exactly in a skin UPK are the texture bytes ? **→ Resolved 2026-05-14, see Breakthrough section below**
- [x] What's the canonical pixel resolution ? **→ 2048² for body decals (was 1024² in earlier guess), confirmed BC3/DXT5**
- [ ] How does the mask texture connect to the diffuse — separate asset in the same UPK ? Alpha channel of diffuse ?
- [ ] Are different car bodies' skin UPK formats interchangeable, or per-car ?
- [ ] Does the UPK header reference total file size or per-block sizes that we must patch ?

---

## Breakthrough — 2026-05-14 : full header decryption working

The blocker on every previous attempt was RL's custom encryption of the UE3
package header (`NameTable`, `ImportTable`, `ExportTable`, `DependsTable`).
Without those, mip layout in the body is unparseable — we could only guess
at byte offsets, and every guess collapsed under inspection (see the failed
Octane MVP that crashed RL on GPU upload).

That blocker is now solved.

### Decryption / decompression pipeline

`AltimorTASDK/RLUPKTool` (already shipped in this repo at
`sandbox/decrypt/Game Models decryption/Decryptor/RLUPKTool.exe` and at
`src-tauri/target/{debug,release}/resources/decryptor/RLUPKTool.exe`) does
both stages :

1. **AES-256-ECB** over `[NameOffset, TotalHeaderSize - GarbageSize)` rounded
   up to 16 bytes. RL rotates keys per build; `keys.txt` ships ~120 known
   keys, one match per package. The decryptor reports `KeyN got used 1 time`
   on success.
2. **Chunked zlib** over the body (magic `0x9E2A83C1`, identical structure
   to the body-compression scheme we already implement in
   `commands::palettes`).

Output is a vanilla UE3 1.6 MB UPK that parses cleanly with standard logic.

`sandbox/research/parse_decrypted_upk.py` walks the summary + name table
+ export table.
`sandbox/research/parse_texture2d_real.py` walks one Texture2D export
end-to-end — tagged properties → SourceArt FByteBulkData → mip TArray →
FTextureFileCacheGuid.

### Extracted layout (Skin_Octane_Flames_SF.upk → Pepe_Body_Flames_RGB)

Tagged properties parsed cleanly :

| Property | Value |
|---|---|
| `SizeX`, `SizeY` | 2048, 2048 |
| `OriginalSizeX`, `OriginalSizeY` | 2048, 2048 |
| `Format` | `PF_DXT5` (BC3) |
| `TextureFileCacheName` | `Textures3` |
| `MipTailBaseIdx` | 11 |
| `FirstResourceMemMip` | 5 |

After `None` terminator at body offset `0xfc` :

- 12-byte empty SourceArt FByteBulkData (`flags=0x10000`, elem=0, size=0 —
  RL omits the offset field when size=0; `0x10000` is the
  `BULKDATA_64BitOffset` marker).
- `i32 NumMips = 12`.
- 12 mip entries, 28 bytes each (TFC-stored case) :
  `u32 flags + i32 ElementCount + i32 SizeOnDisk + i64 OffsetInFile + i32 SizeX + i32 SizeY`.

Per-mip data :

| mip | dims | flags | TFC offset | compressed size |
|---:|---:|---:|---:|---:|
| 0 | 2048² | `0x10003` | 64 746 122 | 461 374 |
| 1 | 1024² | `0x10003` | 64 621 340 | 124 782 |
| 2 |  512² | `0x10003` | 64 587 623 |  33 717 |
| 3 |  256² | `0x10003` | 64 576 190 |  11 433 |
| 4 |  128² | `0x10003` | 64 571 438 |   4 752 |
| 5–11 | 64²–1² | inline | (payload in body) | small |

`flags = 0x10003` = `StoreInSeparateFile (0x01)` + `SerializeCompressed (0x02)`
+ `64BitOffset (0x10000)`. The five mips above sit in `Textures3.tfc` as
contiguous **zlib-compressed** blocks. The seven smallest mips inline in
the body (threshold = `FirstResourceMemMip`).

### What this unlocks

Custom decal swap reduces to **a 12-byte patch per mip** at known offsets
inside the decompressed body :

- `SizeOnDisk` field (i32) at `body + 0x114 + mip_idx * 28`
- `OffsetInFile` field (i64) at `body + 0x118 + mip_idx * 28`

…relative to the Texture2D's serial start (`0x196CE` for this skin, read from
its `FObjectExport.SerialOffset`).

The plan, in order :

1. **#53 (gating test)** — copy `Skin_Octane_Flames_SF_decrypted.upk` to
   `CookedPCConsole/mods/Skin_Octane_Flames_SF.upk` as-is. Launch RL, hover
   Octane Flames decal in garage.
   - If the decal renders normally → `mods/` accepts **decrypted** UPKs.
     Implementation path : decrypt → patch mip entries → write decrypted to
     `mods/`. No re-encryption needed.
   - If RL fails / crashes → `mods/` requires the encrypted+compressed
     format. Implementation path : decrypt → patch → re-zlib-compress body
     chunks → re-AES-encrypt header → write to `mods/`. Doable, just adds
     ~1-2 hours of code.
2. **#54** — Python prototype : PNG → BC3 2048² with mip chain (`image_dds`)
   → zlib-compress each mip (matching RL's BULKDATA_SerializeCompressed
   chunked format) → append to `Textures3.tfc` → patch the 5 TFC mip
   entries' `SizeOnDisk` + `OffsetInFile` in the body → save UPK to `mods/`.
   Test in RL.
3. **#55** — Port to Rust : reuse `image_dds` + `flate2 zlib-rs` (already in
   deps for palettes). Wire to the existing `CustomDecalsPage` UI by
   un-stubbing `SUPPORTED_BODY_IDS`. Support all 27 user decals across
   Octane / Fennec / Venom / Roadhog / Dominus / Universal.

### Open question (next gating point after #53)

Does `mods/` also override `Textures3.tfc` ? If yes, the cleanest model is
copy-on-write : ship a `mods/Textures3.tfc` with our appended bytes,
leaving the install pristine. If no, we either :

- Append to the real `Textures3.tfc` (3 GB → grows by ~1 MB per decal pack,
  tracked for clean uninstall), or
- Add a custom `TextureFileCacheName` (e.g. `Textures_ALXS`) and ship our
  own small `.tfc` file — requires growing the name table by one entry,
  which shifts every subsequent header offset and is the harder code path.

---

## SOLUTION — 2026-05-14 : end-to-end working pipeline

Custom decal swap is **solved and validated in-game**. Pipeline confirmed by
swapping Pepe_Body_Flames_RGB to solid red on the Octane Flames decal slot.

### The complete recipe

For each user custom decal :

1. **Create / update a shared `MyDecal01.tfc`** in `CookedPCConsole/` (game
   install root, alongside `Textures3.tfc` — **not** in `mods/`, because
   RL's TFC path resolution does not search `mods/`) :

   - Allocate as a **sparse file** on NTFS (`fsutil sparse setflag`).
     Virtual size = max original TFC offset used by the target UPK ; actual
     disk usage tracks only the bytes we write.
   - Copy the byte ranges needed by the target UPK's 4 textures from real
     `Textures3.tfc` into `MyDecal01.tfc` **at the same offsets** so that
     the 3 auxiliary textures (normal map, mask, specular) we don't replace
     still resolve correctly through their unchanged mip offset_in_file
     fields.
   - Append the user's custom BC3 data at the end (chunked-zlib wrapped,
     same format RL uses inside TFC blocks).

2. **Modify the skin UPK** and save to `mods/`:

   - AES-decrypt the header (key from
     `sandbox/decrypt/.../keys.txt`, region
     `[NameOffset, (TotalHeaderSize - NameOffset) & ~15]`).
   - **Hijack the name table** : rename `Textures3` → `MyDecal01`
     in-place (both 9 chars — no name-table growth required, no other
     header offsets shift). Every NameProperty referencing this name's
     table-index now resolves to "MyDecal01" → RL looks for
     `MyDecal01.tfc` instead of `Textures3.tfc`.
   - In the body's **chunk 0 block 0** (which contains the texture's
     serial data), update the 5 TFC mip entries of the target Texture2D
     (e.g. `Pepe_Body_Flames_RGB`) — three fields per mip :
     `ElementCount` (i32), `BulkDataSizeOnDisk` (i32), `BulkDataOffsetInFile`
     (i64). Update them to point at the appended custom data inside
     `MyDecal01.tfc`.
   - **Recompress** block 0 with zlib level 9. The new compressed size is
     smaller than the original 39 589-byte budget (typically ~38 KB).
   - **Pad** the recompressed block with trailing zero bytes to exactly
     the original c_size (39 589 bytes). RL's zlib accepts trailing data
     after the deflate end-marker, so the padding is invisible to the
     decoder ; but the chunk_info table (which lists per-chunk byte
     ranges) stays consistent, which is essential for RL not to crash
     on a body-size mismatch.
   - **Re-encrypt** the modified header (deterministic ECB — only the AES
     blocks containing our name-table edits and any other touched bytes
     produce new ciphertext, the rest stays byte-identical to original).
   - Save to `mods/<skin>.upk`. RL's `mods/` override mechanism loads our
     UPK instead of the original.

### Architecture properties

- **Game install pristine.** `Textures3.tfc` is never modified.
  `MyDecal01.tfc` is a *new* file we add. Uninstall = delete that file +
  the UPKs in `mods/`.
- **Steam / Epic integrity-safe.** No existing file checksum changes, so
  the launcher's verify-game step never re-downloads anything.
- **Scales to a library.** Multiple custom decals across multiple car
  bodies share the same `MyDecal01.tfc` (each appends its data; the file
  grows incrementally). For 27 decals across 5 body slots the file is
  likely ~10-50 MB virtual / similar real disk via sparse.
- **Single point of failure.** If `MyDecal01.tfc` is corrupted or deleted
  while a UPK is in `mods/`, RL falls back to inline mips gracefully —
  decal appears pixelated, no crash. Recovery = regenerate the file.

### Critical facts confirmed during this work

- `mods/` overrides `.upk` files but **not** `.tfc` files. RL's TFC search
  resolves to `CookedPCConsole/` only.
- RL's zlib accepts trailing bytes after a deflate stream's end-marker.
  This is what makes in-place body modification possible without disturbing
  RL's chunk_info expectations.
- The real `FCompressedChunkInfo` table for this skin UPK is at decrypted
  offset `0x4BB7` (which the summary calls `DependsOffset`, not at the
  value pointed to by the summary field literally named
  `CompressedChunkInfoOffset = 0x49B2` — that location holds an empty
  table with count = 0). Confirmed empirically by matching the table's
  11 entries 1-to-1 with our chunks.
- RL does not validate `TextureFileCacheGuid`, the TFC file's signature,
  or per-byte checksums.
- The 4 Texture2D exports in a skin UPK (diffuse, normal, mask, specular)
  all share the `Textures3` reference. The name hijack therefore redirects
  *all four* to `MyDecal01.tfc` ; copying the auxiliary textures' original
  byte ranges into `MyDecal01.tfc` keeps them rendering correctly.

### Reference implementation

`sandbox/research/decal_swap_final.py` (~280 lines Python) — produces a
working solid-red swap end-to-end. Uses `pycryptodome` for AES,
`fsutil sparse setflag` (subprocess) for NTFS sparse marking, and stdlib
`zlib` / `struct`. To port to Rust : `aes` crate (add dep) +
`flate2 zlib-rs` (already in deps) + `winapi` + `FSCTL_SET_SPARSE` for
the sparse marking.

### Remaining work

- Replace the `make_solid_red_bc3()` synthetic generator with a real
  PNG → BC3-with-mips encoder (`image_dds` crate, already in deps for
  the earlier inline-mip attempt).
- Generalise texture identification : the prototype currently hardcodes
  Pepe_Body_Flames_RGB by matching its mip 0 offset (`64 746 122`). For
  other car bodies + decal slots, parse the export table by name to
  locate the target diffuse Texture2D dynamically.
- Track installed decals in app data : which decal is on which body, where
  each one lives in `MyDecal01.tfc`, plus the original UPK identity so the
  user can revert per-slot.
- Port the Python prototype to Rust and wire to `CustomDecalsPage` UI.

---

## Critical discovery — 2026-05-15 : RL stores Diffuse + Skin as *separate* textures for multi-color decals

The end-to-end pipeline (hijack name → custom `.tfc` → mods/ UPK) is rock-solid, but **picking the right swap-target slot** matters enormously. The slot we ship the user PNG into determines which shader RL applies, which determines whether arbitrary RGB renders at all.

### What we learned in-game (Yuna Itzy test)

Swapping into `Skin_Octane_Flames_SF.upk` (a 1-texture slot, `[_RGB]` only) and feeding it our `merge_diffuse_and_skin` output produced this exact rendering :

| Texture region we wrote | What RL drew on the car |
|---|---|
| `(R=255, G=0, B=0, A=0)`     — team marker        | **Primary color** (correct) |
| `(R=255, G=0, B=0, A=255)`   — secondary marker  | **Secondary color** (correct, the "YUNA" text rendered yellow/green because that's the player's secondary) |
| `(R=255, G=255, B=255, A=255)` — white at α=255   | Rendered as **BLACK** (the "ITZY" text in the merge appeared black on the car) |
| Arbitrary RGB at α=255 (Yuna's photo portrait) | **Mix of white fallback + secondary**, "complètement buggé" |

The conclusion is firm : **the `[_RGB] only` slot's native shader supports a fixed marker palette, not arbitrary diffuse RGB**. AlphaConsole's "1_Diffuse_Skin" convention (the alpha-encoded combined texture) was its own internal format that its **runtime-injected custom shader** could re-interpret. Without injection (post-EAC), RL's native shader treats that texture as a paint mask only.

### How RL ships actual multi-color decals

Scanning every Octane skin UPK in `CookedPCConsole/` (558 files) for Texture2D class exports yielded **160 distinct texture-name fingerprints**. The dominant patterns :

| Pattern (sorted) | Count | Example decals | What the slot supports |
|---|---|---|---|
| `[_RGB]` only | 50 | Flames, Lightning, Stars, Stripes, Tech, Wings, Skulls, Racer, Complexity_Esports | Mask-only; team/secondary/windows markers; **no arbitrary RGB** |
| `[_BlankSkin, _Curvature, _D, _RGB]` | 42 | Mayan, AbstractPanels, AlienThief, AztecSnake, Blammer, Sharkbite, GaleFire | Full "Force" multi-color set : separate diffuse + curvature + base skin + mask |
| `[_D, _RGB]` | 35 | NFL hometown skins (`arz_h`, `atl_h`, `buf_h`, `car_h`, etc.) | Diffuse + mask — minimal multi-color setup |
| `[T_AutoUni_LogoGuide, T_BlackAlpha, _D, _RGB]` | 31 | Baroque, Complexity_024, EndGame, GenG_024 | Esports decals : diffuse + mask + 2 helper textures |
| `[T_AutoUni_LogoGuide, T_BlackAlpha, _D, _RGB, _RGB]` | 13 | BDS, G2, Dignitas, Luminosity (esports `024`) | Multi-color esports + extra Flames-style overlay |
| 12-/14-texture chains | a handful | Pavement, FeralCat | Premium decals with normal/emissive/noise maps |

The naming suffixes follow Psyonix's cooker convention :

- `_RGB` → the **color-region mask** (team / secondary / decal / windows zones)
- `_D` → the **actual diffuse** (visible RGB colors, the art)
- `_BlankSkin` → a base skin layer applied beneath the mask
- `_Curvature` → curvature lighting info
- `_N` → normal map
- `T_BlackAlpha` / `T_AutoUni_LogoGuide` → tiny helper textures shared across many esports skins
- `_M` → material / specular

The shader bound by `MIC_Body_Octane` (imported, not defined in the skin UPK) reads whichever subset of these textures the material expects. So a `[_RGB]`-only slot's shader has **no `_D` to sample from** — that's why arbitrary RGB in the user's PNG can never reach the screen there.

### Strategy pivot — match user pack against the right slot

Two cases for the user's `Template.json` layout :

1. **Universal pack** (e.g. `HeatFire`) — only `body_diffuse.png`, no separate skin. The role in `Template.json` is `1_Diffuse_Skin` and the alpha channel encodes the mask. Target a `[_RGB]`-only slot (e.g. `Skin_Octane_Stripes_SF.upk`) and write the user's combined PNG straight in. Limited to team/secondary/windows colors.

2. **Body-specific pack** (e.g. `Dirty Work`, `Yuna Itzy`, `Chaewon LeSserafim`) — has both `Diffuse` and `Skin` PNG roles. Target a `[_D, _RGB]` slot (one of the 35 NFL hometown decals — least likely to be in active use by a player), then :
   - Write user's `Diffuse` PNG → `<name>_D` Texture2D (encoded as BC3 mip chain at the slot's native dim)
   - Write user's `Skin` PNG → `<name>_RGB` Texture2D
   - No merge needed — RL's multi-color shader composites them itself

Decal packs that ship `_BlankSkin` / `_Curvature` / normal maps don't need to be the swap target ; we can leave those textures untouched and just hijack the `_D` and `_RGB` pair.

### Per-pack victim-slot recommendation (Octane only, MVP)

The 35 NFL hometown decals are the cleanest 2-texture targets. Each is its own UPK, so we can stand up multiple simultaneous user swaps by allocating one slot per active custom decal. A natural mapping for the user's MVP :

| Victim slot (in-game name) | Cosmetic origin | Notes |
|---|---|---|
| `Skin_octane_arz_h_SF.upk` (Arizona Home) | NFL hometown | low-popularity decal, perfect victim |
| `Skin_octane_atl_h_SF.upk` (Atlanta Home) | NFL hometown |  |
| `Skin_octane_buf_h_SF.upk` (Buffalo Home) | NFL hometown |  |
| `Skin_octane_car_h_SF.upk` (Carolina Home) | NFL hometown |  |
| `Skin_octane_chi_h_SF.upk` (Chicago Home) | NFL hometown |  |

The user equips one of these decals in-game (Garage → Octane → Body → Decal → pick "Arizona Home" etc.) to see the custom art rendered.

### Open question : how `_RGB` markers map at runtime

Empirical observations from the Yuna Itzy test confirm the marker palette **for the `[_RGB] only` shader** :

| Pixel value (texture) | Renders as (on body) |
|---|---|
| `(255, 0, 0, A=0)`     | Primary / team color (player's loadout) |
| `(255, 0, 0, A=255)`   | Secondary color (player's loadout) |
| `(255, 255, 255, A=255)` | Black (definite — visible in Yuna's "ITZY" text) |
| `(0, 0, 255, A=0)`     | Windows (decal-zone B-channel marker) |
| Anything else at α=255 | White fallback / undefined |

The `[_D, _RGB]` shader may interpret `_RGB` differently (likely the same markers but blended with `_D` where the mask permits) — that's the next thing to verify by swapping into a `[_D, _RGB]` slot with a known test pattern.

### Reference implementation status

- `src-tauri/src/commands/decal_swap.rs` — runs end-to-end against `Skin_Octane_Flames_SF.upk` (1-texture slot). Includes `merge_diffuse_and_skin` (kept for the universal-pack case) and `png_to_bc3_mips_for_chain`.
- Multi-texture target slot support (`[_D, _RGB]`) — not yet implemented; this is the next step. Needs : (a) parse the skin UPK to identify each Texture2D's role by name suffix, (b) build a per-texture swap entry, (c) extend `DecalSwapPlan` to carry a `Vec<(role, png_bytes)>` instead of just `diffuse + skin`.

### Open question : per-body coverage

This analysis is **Octane-only** (BodyID 23). The 35-file `[_D, _RGB]` pattern should exist for Fennec, Venom, Roadhog, Dominus, etc., but their NFL/sport slots may use different naming. Re-run `categorize_skin_patterns.py` against `Skin_Fennec_*_SF.upk`, `Skin_Venom_*_SF.upk`, … to map equivalents per body before generalising the UI.

---

## Final breakthrough — 2026-05-15 : donor-UPK rename trick + dual-texture swap

The "target the right slot" challenge from the previous section had a critical flaw : **users only own a small subset of decals**. Telling someone "to use this custom decal, you need to also own Mayan / AbstractPanels / NFL Arizona-Home" defeats the point. The user needs custom decals working on decals they ALREADY own — Flames, Stars, Stripes etc. (the basic universal-tier set).

### The donor-UPK substitution trick

When RL loads a skin from `mods/<filename>.upk`, the *internal* package name in the UPK header must match the filename. Otherwise RL refuses to load it (we proved this in the earlier rlpeak research). But beyond that one constraint, **RL doesn't validate that the UPK content matches what the catalog says about that slot**.

So we can take any donor UPK (any decal file on disk, regardless of whether the user owns it as an item), rename its internal package name to match a slot the user *does* own, and ship it as `mods/<owned-slot>.upk`. RL loads our renamed donor when the user equips the owned slot — and the donor's shader / textures / paintable behaviour kick in.

This is rlpeak's rename trick — but cross-tier. rlpeak only renames between same-tier decals (simple → simple). For custom multi-color decals we need cross-tier (multi-color donor → simple target slot).

### Picking the donor — the ForcedColor catch

The obvious first attempt was `skin_octane_galefire_psplus_SF.upk` (a PS+ free multi-color decal). The rename worked — the GaleFire art rendered on the Flames slot — but **both color pickers were disabled in-game**. Same with `Skin_Octane_Baroque_SF.upk` (esports decal).

Looking at their name tables we found two suspicious FName entries that aren't in plain Flames' name table :

- `ForcedCustomColor`
- `ForcedTeamColors`

These are property names that RL uses to lock the player's color picker — the decal "forces" specific colors regardless of what the player picks. Both GaleFire_psplus and Baroque have these names, both lock the pickers.

The fix : **find a donor whose name table does NOT contain these names**. Such donors should leave the color picker fully active.

### The scan

`sandbox/research/find_paintable_multitexture.py` walks every Octane skin UPK, decrypts its header, lists its Texture2D exports + name table, and reports UPKs where :

- Texture2D count ≥ 2 (multi-texture, needed for multi-color shader)
- Neither `ForcedCustomColor` nor `ForcedTeamColors` is in the name table

Result : **76 candidates** out of 558 Octane skins. Notable ones :

| Donor UPK | Texture set | Notes |
|---|---|---|
| `skin_octane_galefire_SF.upk` | `[Body_Force_Curvature, Force_Body_BlankSkin, Force_Body_D, Skin_Octane_GaleFire_RGB]` | Standard 4-texture Force pattern, paintable. **Validated end-to-end 2026-05-15** : rename to Flames slot, multi-color art displays + both color pickers active. |
| `skin_octane_alebrijerocker_SF`, `alientheif_SF`, `aztecsnake_SF`, `blammer_SF`, `cheesy_SF`, `cheetah_SF`, `contemplativerabbit_SF`, `crowscare_SF`, `curve_SF`, `dragoner_SF`, `edgestar_SF`, `elgato_SF`, `escorpio_SF`, `futurerascal_SF`, `GrillerKiller_SF`, … | Same 4-texture Force pattern | Should all behave identically — pick any |
| `skin_octane_bask_SF.upk` | `[_D, _Painted, _RGB, T_BlackAlpha, T_AutoUni_LogoGuide]` | Adds a `_Painted` variant — possibly a different paintable path |
| `skin_octane_curve_SF.upk` | `[…, _RGB, _RGB_Painted]` | Two `_RGB`s, one explicitly `_Painted` |

The `psplus` / `Baroque` / `EvilGeniuses_*` variants are the ones to AVOID — they're the gated tournament / promotional decals where Psyonix forces fixed colors.

### Plan for the full pipeline (current task #68)

1. **Pick the donor** : `skin_octane_galefire_SF.upk` (4-texture Force pattern, paintable).
2. **Rename** its internal package names to match the target slot (Flames for MVP) — length-preserving in-place patch of `skin_octane_galefire_SF` → `Skin_Octane_Flames_SF` and `skin_octane_galefire` → `Skin_Octane_Flames` (both fit since target is shorter).
3. **Identify the four Texture2D exports** by name suffix :
    - `Force_Body_D` → user's `Diffuse` PNG (visible RGB art)
    - `Skin_Octane_GaleFire_RGB` → user's `Skin` PNG (color-region mask)
    - `Force_Body_BlankSkin`, `Body_Force_Curvature` → preserve original byte ranges (these are shared cross-decal textures the player's body material depends on)
4. **Build the sparse `MyDecal01.tfc`** (existing pipeline) — same routine as before, just iterating over four textures instead of one. Copy the two preserved textures' byte ranges from real `Textures3.tfc` ; append BC3-encoded user data for the two swapped textures.
5. **Patch the body's mip arrays** for the two swapped textures (existing `rewrite_block0_multi` helper) ; recompress block 0 with trailing-zero padding to original c_size.
6. **Hijack the name table** `Textures3` → `MyDecal01` so all mip lookups resolve to our custom TFC.
7. **AES-re-encrypt** the header ; save to `mods/Skin_Octane_Flames_SF.upk`.

In-game flow : user equips Octane Flames → RL loads our renamed donor → multi-color shader + their PNGs + paintable pickers all working together.

### Generalising to other slots

The same approach works for any "owned simple slot" → "donor multi-color decal" mapping. The donor names need length-preserving renames per target. The 50 `[_RGB] only` Octane slots (Flames, Stars, Stripes, Skulls, Wings, Lightning, Tech, Racer, etc.) are all viable targets. For other car bodies (Fennec, Venom, Dominus, Roadhog) we need to re-run the scanner against `Skin_<Body>_*_SF.upk` to find the equivalent donor list.

### Reference implementation status

- `sandbox/research/test_galefire_rename.py` — minimal Python that does just step 2 (rename) and ships the result to `mods/`. **End-to-end validated in-game on 2026-05-15** — design + pickers both work.
- `sandbox/research/decal_swap_dual_final.py` — Python reference for the full dual-swap (D + RGB) using the empirical TFC bindings. **Validated 2026-05-15** : Stars slot renders solid red body + solid uniform mask + both pickers active.

---

## 🎯 BREAKTHROUGH 2026-05-15 — Empirical TFC bindings

After many days of crashes when swapping the `_RGB` mask alongside `_D` diffuse, we discovered why : **the tagged property `TextureFileCacheName` lies about which TFC RL actually opens**.

### The lying property

Every Texture2D in a GaleFire donor has a tagged property `TextureFileCacheName` whose value resolves through the package's FName table. Parsed via standard UE3 tagged-property reading :

| Texture | TextureFileCacheName (claimed) | FName idx |
|---|---|---|
| `Force_Body_D` | `Textures` | 206 |
| `Force_Body_BlankSkin` | `Textures` | 206 |
| `Body_Force_Curvature` | `Textures` | 206 |
| `Skin_Octane_GaleFire_RGB` | `Textures7` | 211 |

So the property claims D-side reads from `Textures.tfc` and RGB-side reads from `Textures7.tfc`.

### What RL actually does

**Correction 2026-05-15 (afternoon)** : the earlier "Textures3 hijack worked for D" conclusion was wrong. The end-to-end production install via the app exposed it : the body rendered with **white in the diffuse-passthrough regions** of the mask, because RL was trying to read D's mip from real `Textures.tfc` (matching the tagged property), found garbage at our patched offset, and fell back to a default white texture. The mask side rendered correctly via the `Textures7 → MyDecal02` hijack, which made it LOOK like D was working in earlier solid-red tests — what we thought was D rendering solid red was actually the mask's "primary color everywhere" applying the picker's color over an unchanged GaleFire diffuse.

The diagnosis was confirmed by inspecting the installed UPK :

```
Force_Body_D     TextureFileCacheName='Textures'     mip0 off=39,559,058 (our new MyDecal_ offset)
Skin_*_RGB       TextureFileCacheName='MyDecal02'    mip0 off=423,968,233
```

Note `Force_Body_D`'s tagged TFC name was **NOT** affected by the `Textures3 → MyDecal01` hijack — it remained `Textures` after the rename. Because RL uses this property for TFC lookup, D's data was always being fetched from the wrong file.

**Empirical bindings (corrected)** :

- **Hijacking `Textures` (idx 206) → `MyDecal_`** : redirects `Force_Body_D` + `Force_Body_BlankSkin` + `Body_Force_Curvature` to our custom diffuse-side TFC. (8-char target required by the length-preserving rename rule.)
- **Hijacking `Textures7` (idx 211) → `MyDecal02`** : redirects `Skin_<Donor>_RGB` to the mask-side TFC.
- **Hijacking `Textures3` (idx 208)** : NO observable effect on any Texture2D — `Textures3` doesn't appear in any of the GaleFire donor's tagged properties. (Earlier confusion came from the mask hijack making the body LOOK like the diffuse was swapped.)

So the canonical bindings ARE the per-texture tagged property `TextureFileCacheName` after all. The earlier confusion came from a misread test result.

### The crash that led to the insight

Patching `Skin_*_RGB`'s mip array offsets (changing where in the TFC to read mip data) crashed RL with :

```
Warning: Detected data corruption [header] trying to read 1048576 bytes at offset 423977625
                                          ^^^^^^^^^^^^                  ^^^^^^^^^^^^^^^^^^^
                                          = mip 1 elem (uncompressed)   = mip 1 start in our wrapper
```

RL was reading `elem` bytes (uncompressed BC3 size = 1 MB for mip 1), not `size_disk` bytes (compressed wrapper ≈ 2 KB). With our wrapped data only 2 KB at that offset, RL read 1 MB into the next mip and past EOF → "data corruption".

This happened *only* for the RGB mask, never for the diffuse — confirming RL has a distinct loading path for masks (or all textures in `Textures7.tfc`) that reads `elem` bytes raw. The diffuse loading path uses chunked-zlib decompression with `size_disk` bytes.

Initially we thought the fix was to write raw uncompressed BC3 at `elem`-sized offsets, but that rendered as garbage (RL interpreted the bytes as a different pixel format). The real fix was : **redirect to the correct TFC**, and the diffuse-path code in RL apparently handles `MyDecal02.tfc` correctly with chunked-zlib data.

### The working dual-swap

```
FName hijacks (length-preserving, null-padded) :
  Textures  (8) → MyDecal_ (8)   ← D + aux (idx 206)
  Textures7 (9) → MyDecal02 (9)  ← mask (idx 211)

Custom TFCs in CookedPCConsole/ :
  MyDecal_.tfc — sparse ~40 MB, contains :
      [Body_Force_Curvature original bytes copied from real Textures.tfc]
      [Force_Body_BlankSkin original bytes copied from real Textures.tfc]
      [Force_Body_D wrapped user PNG → BC3 chunked-zlib appended]
  MyDecal02.tfc — sparse ~424 MB, contains :
      [Skin_<Donor>_RGB wrapped user mask PNG → BC3 chunked-zlib appended at original max-offset]

UPK body patches (after AES-decrypting + decompressing) :
  - Force_Body_D mip array → 5 TFC mips redirected to new offsets in MyDecal_.tfc
  - Skin_<Donor>_RGB mip array → 5 TFC mips redirected to new offsets in MyDecal02.tfc
  - FName table renames: galefire→stars, GaleFire→Stars (texture/material/thumbnail/template),
                          Textures→MyDecal_, Textures7→MyDecal02
  - Re-encrypt header (AES-256-ECB)
  - Recompress body block 0 to fit original c_size budget
```

**Critical post-rename gotcha** : when the rename target is shorter than the source (e.g. `Skin_Octane_GaleFire_RGB` 24 chars → `Skin_Octane_Stars_RGB` 21 chars), we null-pad the FName slot. The FName length prefix stays at the original size, so naïve `parse_name_table` returns the string with trailing `\0` bytes. The `_RGB` suffix detector then sees `"_rgb\0\0\0"`, fails the `ends_with("_rgb")` check, and the mask falls into the "preserve" bucket instead of being recognised as Mask. The fix is to trim trailing nulls in `parse_name_table` (see `decal_swap.rs` `parse_name_table`).

**In-game result on Stars slot (2026-05-15 evening)** : custom diffuse design visible on body, user's primary color in mask's primary regions, accent in mask's accent regions, both pickers active.

### Generalising the dual-swap

Any donor we've previously identified as paintable (76 candidates from `find_paintable_multitexture.py`) should work with the same two-hijack scheme as long as it has both a body-diffuse Texture2D and a `_RGB` mask Texture2D. The hijack FName replacements are donor-agnostic (always `Textures` and `Textures7`). What varies per donor : the auxiliary textures' offsets in real `Textures.tfc` (we copy them byte-for-byte into `MyDecal_.tfc` at original positions so unmodified mip references still resolve).

For other car bodies (Fennec / Venom / Dominus / Roadhog), each has its own `Body_<Body>_Curvature` / `Force_Body_BlankSkin` analogues but the same `Textures` / `Textures7` TFC convention should apply.

## Reparent path investigation 2026-05-15

Question : instead of swapping the body diffuse texture under the existing paint-system shader, could we **reparent the MIC's `Parent` ObjectIndex to a literal-diffuse base material** (e.g. `EngineDebugMaterials.TexturePaint_2Tex_Color`) so the user's PNG renders as-is, no paint compositing?

Three scripts written under `sandbox/research/`:

1. `inspect_engine_debug_materials.py` — decrypt + dump `EngineDebugMaterials.upk`
2. `galefire_imports.py` — list every import row in `Skin_Octane_GaleFire_SF.upk`
3. `parent_swap_dry_run.py` — locate the MIC's `Parent` ObjectIndex byte and round-trip a re-encoded copy

Catalog : `catalog_material_parents.py` → [`material_parent_candidates.md`](material_parent_candidates.md) (sampled scan of 53 representative UPKs).

### EngineDebugMaterials.upk — what's inside

The file IS plaintext (no AES on stock UE3 engine UPKs — only RL-cooked skin/body UPKs are encrypted). It uses the **stock UE3 export layout** (68-byte flat entries with i32 SerialOffset), distinct from RL's skin layout (variable-size entries with i64 SerialOffset and a NetObject section).

26 Material/MIC exports total. The relevant ones :

| Export | Class | Notes |
|---|---|---|
| `BlackUnlitMaterial` | Material | smallest, no params |
| `TexturePaint_2Tex_Color` | Material | name refs `Normal`, `Mask` in body |
| `VertexPaint_2Tex_Color` | Material | name refs `ParameterName`, `Normal`, `Mask` |
| `VertexPaint_4Tex` | Material | name refs `Normal`, `Mask` |
| `WireframeMaterial` | Material | minimal |
| `MI_Sand_Master_VertPaint` | MaterialInstanceConstant | references `Diffuse` parameter |

**Key obstacle :** Materials use a **custom binary serialiser** (the compiled-shader expression graph) — they are NOT pure tagged-property blobs like MICs. We cannot read the parameter list as easily; we only see name-table references via byte scan. Full parameter introspection would require porting UE3's `UMaterial::Serialize`.

### Skin_Octane_GaleFire_SF.upk import table

47 import rows. **Material-class imports : exactly ONE.**

```
row 20  MaterialInstanceConstant  MIC_Body_Paintable_All  ref=Vehicle_Parent_Materials.MIC_Body_Paintable_All
```

No `EngineDebugMaterials` package row. No literal-diffuse Material row. No way to point the MIC's `Parent` at `TexturePaint_2Tex_Color` without **injecting two new import rows + one new package row** :

- Package row for `EngineDebugMaterials` (cls_pkg=Core, cls=Package)
- Material row for `TexturePaint_2Tex_Color` (cls_pkg=Engine, cls=Material, outer=-newPkgRow)
- Probably an `Engine.Material` class import too (the skin already has `Engine.MaterialInstanceConstant` at row 7, not `Engine.Material`)

Injecting import rows means **header re-layout** : import count++, every header-offset (export_offset, depends_offset, ...) shifts, every export's relative serial_offset stays the same but the post-header file content gets shoved down by N*28 bytes. **The header is AES-ECB encrypted in 16-byte blocks** — adding 28 bytes (not a multiple of 16) breaks alignment.

### Parent ObjectIndex offset — provable, byte-level precision

`parent_swap_dry_run.py` walks tagged props of the MIC exports and locates the `Parent` ObjectProperty. The skin has two MICs :

| Export | Name | Parent.ObjectIndex | Parent resolves to | Byte offset (decompressed body) |
|---|---|---|---|---|
| 66 | `Body_Force_MIC` | (preamble finder mismatched — needs body-pos heuristic improvement; the `-1` NetIndex scan picked up the wrong export's preamble for the second-in-file MIC) | n/a | n/a |
| **67** | **`Octane_GaleFire_MIC`** | **`-21`** | **`import[20] : MIC_Body_Paintable_All`** | **`0x128959` (1 214 809)** |

So Octane_GaleFire_MIC — the actual user-facing one — has its `Parent` pointing at `Vehicle_Parent_Materials.MIC_Body_Paintable_All` (an MIC, not even a plain Material). Its tagged-prop structure is :

```
TextureParameterValues   ArrayProperty
VectorParameterValues    ArrayProperty
Parent                   ObjectProperty  size=4   value=-21
```

To re-target this MIC's Parent to a different material, you write 4 little-endian bytes at decompressed-body offset `0x128959`.

The **good news** : the round-trip works. We re-zlib'd all 11 body chunks (using zlib level 9 — actually shaves 3156 bytes vs Psyonix's compression) and re-parsed the resulting test file. The flipped Parent value (`-35` = Texture2D `Import_Body_BlankSkin`, just an arbitrary target to prove writeback) was read back correctly. Header offsets remained valid because they only reference :

- The decompressed body offsets (stable as long as we don't alter the body's logical content, only its compressed representation)
- The compressed chunks live after `total_header` — we appended chunks sequentially so the file is well-formed even though its compressed tail is shorter

Test file : `sandbox/research/test_output/Skin_Octane_GaleFire_SF_parentswap_TEST.upk` (~320 KB, original ~324 KB).

### Verdict

**Reparenting to `EngineDebugMaterials.TexturePaint_2Tex_Color` is NOT viable as a clean swap** for a stock vehicle decal :

1. **Required new imports** : the EngineDebugMaterials package isn't in GaleFire's import table. Adding it means growing the header — but the header is encrypted as fixed-size AES-ECB blocks. Header growth would force a full header re-layout : import_count++, import_offset/export_offset/depends_offset etc. all shift, every byte after the modification within the same encrypted block changes, the file's total size changes (probably by 28 bytes for one import row), and the cooked-package CRC (if any) needs recomputing. Doable but engineering-heavy.

2. **Material expression graph is "stock UE3", not RL-cooked** : the engine UPK uses the OLD export layout (68-byte flat). At runtime, RL's shader runtime expects MICs to point at materials compiled for its specific shader cache. `EngineDebugMaterials` materials may or may not have shader bytecode that RL's renderer can dispatch — there's no proof they do.

3. **The actual fully-paintable Octane_GaleFire_MIC Parent is suspected to be `MIC_Body_Paintable_All` (vehicle parent material)** — that's the master paintable shader. Reparenting to `TexturePaint_2Tex_Color` would lose ALL the body shading (envmap, fresnel, the chrome look, the AO, the team color, …) — the result would be a flat textured cube, not a car.

4. **Easier path remaining valid** : the texture-hijack approach already shipped (`feat(custom-decals): dual-TFC hijack`) preserves the paint shader, accepts user PNG → BC3, and produces correct in-game output. Reparenting would be a regression in quality even if it worked.

### Simplest reparent target available **inside** GaleFire's imports

If we wanted to test in-place reparenting **without injecting new imports**, the only Material/MIC import available is **`MIC_Body_Paintable_All` (row 20)** which is almost certainly what the MIC already points at. Reparenting to itself is a no-op. There is **no other Material reachable** without growing the import table.

### Risks summary

- **Header re-layout under AES-ECB encryption** : technically feasible (we already do AES roundtrips for the FName-table rename trick), but adding N rows where N*28 isn't a multiple of 16 requires repacking the encrypted region carefully, and any miscount of dependent header offsets corrupts the file.
- **Shader-runtime incompatibility** : engine-debug materials are not built against RL's deferred-renderer shader cache. Even if the import resolves, the material may fail to render or fall back to "compilation failed" pink.
- **Visual regression** : team color, painted variants, AO, plastic/trim shading, headlights all live inside `MIC_Body_Paintable_All`'s expression chain. Going to a 2-texture debug material loses all of that.
- **Reversal already exists** : we have a working PNG → decal pipeline that preserves paint shading. Reparent would be net negative.

### Scripts produced

| Script | Purpose |
|---|---|
| `sandbox/research/inspect_engine_debug_materials.py` | Decrypt & dump EngineDebugMaterials.upk Material/MIC exports |
| `sandbox/research/galefire_imports.py` | Dump every import row of Skin_Octane_GaleFire_SF.upk |
| `sandbox/research/parent_swap_dry_run.py` | Pinpoint Parent ObjectIndex byte + round-trip encode |
| `sandbox/research/catalog_material_parents.py` | Sample 53 UPKs, list Materials, flag paint-system membership |
| `docs_rl/material_parent_candidates.md` | Generated catalog |

