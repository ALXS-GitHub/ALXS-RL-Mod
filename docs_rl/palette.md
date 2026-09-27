# Custom Color Palette — Research Notes

> **Goal** : let the user define custom RGB values for RL's 20 primary + 20 accent color slots, swap them via the file-swap engine, restart RL to apply.

## Current understanding of RL's color system

### What "primary" and "secondary" mean in RL

- **Primary** color = the main body paint color (the big slot in the body color picker)
- **Secondary / Accent** color = the body's accent color (the second slot)
- Both are stored in the player profile as **palette indices**, not RGB values directly
- RL ships a fixed palette of ~20 slots for each (primary + accent), Psyonix-defined
- At render time, the shader reads the player's chosen index, looks up the RGB in the palette, applies it through the body's PaintMask channels

This is why our custom-decals work even now without color customization: RL applies the palette colors via shader on top of whatever skin texture is in place.

### Skin file color markers — important correction

AlphaConsole's PNG convention (`#FF0000 0% opacity = primary`, `#FF0000 100% = secondary`) is **AlphaConsole-specific**, not RL-native. AlphaConsole defined that convention so its injected runtime layer could substitute and recompose. We can't reuse it.

RL natively uses a separate mask texture (see [custom_decals.md](./custom_decals.md) `PaintMaskInRGB`).

## What was decrypted (batch 1)

| File | Original | Decrypted | Purpose |
|---|---:|---:|---|
| `TAGame.upk` | 68 MB | 152 MB | Central game logic — palette classes + likely default data |
| `Body_Octane_SF.upk` | ~2 MB | ~4 MB | Body asset reference for PaintMask structure |
| `skin_10x_SF.upk` | small | 78 KB | Skin reference for decal structure |

Decryptor used: `sandbox/decrypt/Game Models decryption/Decryptor/RLUPKTool.exe` (RL UPK Tool).

## Identifiers found in TAGame_decrypted.upk

The following palette-related class and property names appear in the decrypted TAGame.upk strings:

```
ColorPalette_X                    ← palette CLASS
Default__ColorPalette_X            ← DEFAULT INSTANCE (the actual data)
CarColorSet_TA                    ← per-car color set class
CarColorSet_v1 / _v2              ← versions
ClubColorSet / ClubColorSet_v1     ← club-team variant
CustomColor / CustomColorID        ← runtime custom-color support
CustomColorFull / CustomColorSet
CustomColorOverride
CustomColors / CustomTeamColors
AllowCustomTeamColors              ← BOOL flag — RL CODE ANTICIPATES CUSTOM COLORS
BlueTeamColorID                    ← team-specific references
BlueTeamColors / BottomTeamColors
DefaultPaintColor                  ← the default color
TeamColorPreferences_TA            ← player prefs class
TeamColorScriptedTexture_TA        ← runtime composed texture
AccentColor / AccentColorID
AccentColorSet / AccentColors
AccentPalette / DarkAccentColor    ← accent (secondary) slots
ColorSet0 / ColorSet1              ← TWO color sets (likely Primary + Accent)
```

Also found serialized DefaultProperties syntax patterns like `>Pstring(ColorSet0)=(` and `>Pstring(ColorSet1)=(` — these look like the entry points to the actual palette data inside TAGame.upk.

## Open architectural question

Is the actual palette data **stored entirely inside `TAGame.upk`**, or is it stored in a **smaller dedicated UPK** that TAGame.upk only references by class name ?

| Scenario | What it means for us | Where to swap |
|---|---|---|
| **A — All in TAGame.upk** | We must swap the full 71 MB TAGame.upk for ~240 bytes of palette change | `mods/TAGame.upk` (heavy, risky) |
| **B — Dedicated palette UPK** | We swap a small file, much cleaner | `mods/<palette-file>.upk` (light, safe) |
| **C — Split (defaults in TAGame, instances elsewhere)** | Override the instance file only | dedicated instance UPK |

**Status :** Not yet confirmed. Batch 2 will look for a dedicated palette / loadout-colors UPK.

## Batch 2 results

- [x] Filename grep on `CookedPCConsole/` for `Color | Palette | Loadout | PaintColor | Tints | Swatch | CarColorSet | UIColor` → **NO dedicated palette UPK exists** at the top level. Closest matches were `GFx_GarageMenu_SF.upk` (UI shell), `MENU_GarageComplex.upk` (UI shell), `MENU_PremiumGarage.upk`.
- [x] Decrypted `GFx_GarageMenu_SF.upk` (2.5 MB) → contains UI strings and ActionScript labels (`bluePrimaryLabelTextField`, `previewClubColors() - primary:`, etc.) but **no embedded palette RGB data**. The picker UI references the palette by index, doesn't store the RGBs.
- [x] Decrypted `MENU_GarageComplex.upk` (9 MB) → has material parameter names (`Color1`, `Color2`, `Color3`, `CustomColor`, `DefaultColor_Team1`) — again references, not data.
- [x] Decrypted `ESportsTeam_Cloud9_SF.upk` (1 MB) — surprisingly **does NOT contain its team colors** at the asset level. Just `TeamName`, `TeamLogo`, `TeamLogoBanner_Cloud9`, and class reference `ProductAsset_ESportsTeam_TA`. The Cloud9 team colors must be defined elsewhere (probably in TAGame.upk's `Default__CarColorSet_TA` per team).

**Conclusion :** The palette data is **most likely entirely inside `TAGame.upk`** as the `Default__ColorPalette_X` instance, since :

1. No dedicated palette UPK was found
2. UI files only reference the palette by index
3. Even esports team data files don't carry their own colors

→ **Architecture A confirmed** (see open question above). We'll need to swap a modified `TAGame.upk` to override the palette.

### Mitigations for swapping a 71 MB file

To keep the swap safe :

- We change *only* the palette byte range, leaving the other 71 MB byte-identical
- We compute SHA-256 of the source `TAGame.upk` and store it as a "expected pristine hash"
- We refuse to operate if the user's `TAGame.upk` doesn't match (different RL version → palette may be at a different offset)
- The `.original` backup mechanism in our existing items engine handles restore cleanly

## Located byte offsets (validated 2026-05-14, revised after first in-game test)

By scanning the decrypted `TAGame.upk` (152 MB) for any `int32 count` followed by valid LinearColor floats (R/G/B in [0,1], A ≈ 1.0), we found palette arrays of various sizes. **The first attempt targeted the 28/18 arrays which turned out to be a legacy / secondary set — RL's actual in-garage picker uses the bigger 70 / 105 arrays below.**

| Palette | Count | Start offset (decrypted) | First color | Role |
|---|---:|---:|---|---|
| Accent (shared) | 105 | `0x00FDF438` | #E5E5E5 → #FF7F7F | The full accent picker (**15 rows × 7 cols**), shared by Blue + Orange teams |
| Primary (Blue) | 70  | `0x00FE24B4` | #507F39 → #397F3F | Blue-team primary picker (**10 rows × 7 cols**) |
| Primary (Orange) | 70 | `0x00FE8556` | #7F7F39 → #7F7039 | Orange-team primary picker (**10 rows × 7 cols**) |

Each entry is `int32 count + count*16 bytes (f32 RGBA)`. Data starts at `offset + 4`.

The user's defined primary RGBs are written to **both** the Blue and Orange primary blocks, so the picker is consistent regardless of the team preview. Accent has a single block shared by both teams.

### Legacy / unused arrays (kept here for reference; we do NOT splice these)

The first scan also surfaced these smaller arrays — they may be older / unused / used by a different subsystem. We learned the hard way that splicing into them had no visible effect in-game; the real palette grid is the 70/105 set above.

| Palette | Count | Start offset | Notes |
|---|---:|---:|---|
| Blue accent (legacy) | 18 | `0x00FE0142` | Cool-tone gradient |
| Blue primary (legacy) | 28 | `0x00FE0C06` | Cool-tone gradient |
| Orange accent (legacy) | 18 | `0x00FE61E4` | Warm-tone gradient |
| Orange primary (legacy) | 28 | `0x00FE6CA8` | Warm-tone gradient |

## "Encryption" check — revised understanding

The TAGame.upk that Epic ships is **not actually AES-encrypted** despite RLUPKTool's "keys.txt" file suggesting otherwise. Direct float-pattern search in the shipped file finds zero palette colors only because the body uses a **custom chunked zlib compression** that obfuscates byte positions — not because of cryptographic encryption. Inspection of the body bytes (see `sandbox/research/parse_chunks.py`) reveals:

- Bytes `0..0xE08`: plain `FPackageFileSummary` fixed fields (magic, version, header size, folder name, package flags, table offsets/counts, GUID, generations, engine/cooker versions, `CompressionFlags`, empty `CompressedChunks` array, package source). All readable directly.
- Bytes `0xE09..TotalHeaderSize`: name + import + export + depends tables — also stored in chunked-zlib form, just like the body.
- Bytes `TotalHeaderSize..EOF`: body, formatted as a sequence of RL custom chunks.

### RL chunk format

Each chunk in the body (and inside the header range past `0xE09`) has this layout:

```
uint32 Magic = 0x9E2A83C1     // chunk sentinel (same as UPK magic)
uint32 BlockSize              // 131072 (128 KiB)
uint32 CompressedTotal        // sum of all block compressed sizes in this chunk
uint32 UncompressedTotal      // sum of all block uncompressed sizes
N pairs of (uint32 CompressedSize, uint32 UncompressedSize)  // N = ceil(UncompressedTotal / BlockSize)
N concatenated zlib-compressed blocks
```

Each compressed block starts with the standard zlib header `78 9C` and decompresses cleanly with `zlib.decompress()`. **There is no AES layer.** RLUPKTool's "keys" file is a red herring for RL — those keys are likely leftovers from a Borderlands-2-era UPK tool the project was forked from.

### Surgical patching

Because the body is just chunked zlib (no real encryption), we can patch the palette **without** producing a decrypted template at all:

1. Locate which `(chunk, block)` pair contains the palette bytes when decompressed.
2. Read that block's compressed bytes, `zlib.decompress` to get 128 KiB.
3. Overwrite the palette RGBs at known in-block offsets.
4. `zlib.compress` with max level (9) — for our palette modifications this produces output smaller than the original block (saves ~900 B).
5. Right-pad the recompressed bytes with `0x00` back to the original block size so all downstream chunks stay at the same file offset.
6. Write the patched block in place. The file remains **byte-identical to the original except for the ~19 KB block** that holds the palette.

This is the approach implemented in `commands::palettes::splice_palette_into` (replacing the older decrypt-template approach).

### Located block for the palette

After a single brute-force pass (`sandbox/research/parse_chunks.py` + a sig-search), all 3 palette arrays are in the **same** block:

| Field | Value |
|---|---|
| Encrypted file path | `<rl>/TAGame/CookedPCConsole/TAGame.upk` |
| Chunk index | 3 |
| Block index | 10 |
| File offset of compressed block | `0x00AD33D8` |
| Compressed size | 19 781 bytes |
| Uncompressed size | 131 072 bytes |

In-block offsets of the `int32 count` for each palette (data follows at `+4`):

| Palette | In-block offset | Count |
|---|---|---:|
| Accent shared | `0x0F483` | 105 |
| Primary Blue  | `0x124FF` | 70 |
| Primary Orange| `0x185A1` | 70 |

These three offsets + the block coordinates are the only RL-build-specific magic numbers we need to maintain across RL updates.

## Reference SHA-256

Pristine encrypted TAGame.upk at the time of research:
```
daf9def22011734d3d3e056a54c800adb565d21cac5af50aa8e260d17b32ea59
```
If the player's file doesn't match, the offsets may differ (different RL build). We'll guard against this.

## Remaining work

- [x] Locate palette byte offsets — see table above.
- [x] **Bundle the decryptor** — RLUPKTool.exe + keys.txt + ICSharpCode.SharpZipLib.dll + RLUPKT.Core.dll live at `src-tauri/resources/decryptor/`, registered in `tauri.conf.json#bundle.resources`. The `prepare_palette_engine` Tauri command invokes it via `tokio::process` to decrypt the user's pristine TAGame.upk into `<app_data>/palette_engine/tagame_decrypted_template.upk` (~152 MB cached). Guarded by SHA-256 == `TAGAME_PRISTINE_SHA256`.
- [x] **Implement byte-level splicer** — `commands::palettes::splice_palette_into` reads the cached decrypted template, writes 4 × `f32` RGBA values per slot at each of the 4 known offsets, refuses if the count int32 has drifted (RL update detector). Covered by unit tests `splicer_writes_user_rgbs_at_known_offsets` and `splicer_refuses_when_count_does_not_match`.
- [x] **Activation uses items-style root swap** — pristine `TAGame.upk` is moved to `<backups>/TAGame.upk.original` (or matched by SHA-256 if a prior backup exists), the spliced template is atomically renamed into place. Deactivation restores the backup with SHA-256 verification before write. The decrypted file is ~2.1× the size of the encrypted one (152 MB vs 71 MB), so the user's install grows by ~80 MB while a palette is active.

### Crash on first launch — root cause + fix

First in-game test crashed RL silently a moment after the window opened. Diagnosis:

The decrypted UPK still carried `PKG_StoreCompressed (0x02000000)` in its `PackageFlags` field (RLUPKTool decompresses the content but does NOT clear the flag). At engine startup, RL's loader honoured the flag and tried to re-decompress our already-flat content → crash.

**Fix** (`commands::palettes::clear_compression_flags`): before writing the spliced output, parse the UE3 `FPackageFileSummary` header up to `PackageFlags` and clear `PKG_StoreCompressed | PKG_StoreFullyCompressed`. Other flags are preserved. Covered by `splicer_clears_compression_flags_but_keeps_others` unit test.

### Failed attempts (recorded so we don't repeat them)

1. **Root atomic-swap of the decrypted 152 MB template** — RL crashes silently a few seconds after launch. The custom-decompressed file is rejected by RL's StartupPackage loader even with `PKG_StoreCompressed` and `CompressionFlags` cleared.

2. **Drop the decrypted 152 MB file in `mods/`** — same outcome. The "Ambiguous package name" dialog appears (informational), the user clicks OK, and RL fatal-errors during `LoadScriptPackages` (`Launch.log` shows the crash at `[0012.37]` between `Startup_LoadGlobalShaders` and the next breadcrumb). The StartupPackage loader does NOT accept the flat decompressed format that RLUPKTool produces.

3. **Manual sanity check: copy the pristine encrypted file into `mods/`** — RL launches normally (with the same informational warning), confirming the `mods/` override mechanism itself works for `TAGame.upk`. The crash is purely about the file content not the location.

### Working approach — surgical patch of the encrypted file

Modify only the ~19 KB block that contains the palette, keep the rest of the 71 MB file byte-identical to the original. Result is structurally indistinguishable from the original except for our palette RGBs.

- [ ] Port the Python prototype (`sandbox/research/parse_chunks.py` and the inline roundtrip in conversation history) to Rust in `commands::palettes::splice_palette_into`.
- [ ] Remove the decrypt-engine prep step from the UI flow (no longer needed — there's no template to cache).
- [ ] Drop the patched file in `<rl>/.../CookedPCConsole/mods/TAGame.upk`. The "Ambiguous package name" dialog will still appear once per launch; treating it as a known cosmetic limitation for now, or trying to suppress later via `DefaultEngine.ini` overrides.
- [ ] Verify in-game palettes show the user's RGBs after RL restart.

## Implementation plan (once data file is confirmed)

1. **Decrypt-and-locate** the palette byte range (one-time research task)
   - Find the RGB float/byte sequence inside the decrypted UPK
   - Confirm by cross-referencing one known RL color (Sky Blue, Crimson, etc.) found via the in-game picker
2. **Rust generator**
   - Input: 20 primary RGB + 20 accent RGB chosen by user
   - Output: a modified UPK with our RGBs at the right offsets, all other bytes byte-identical to the decrypted original
3. **Storage**
   - User's palettes live in `<app_data>/palettes/<id>/` (palette.json with the 20+20 RGB + name)
   - Active palette flag, same pattern as items
4. **Engine integration**
   - New ItemKind::Palette in the items engine, OR a parallel module — TBD based on implementation tidiness
   - Activate = generate UPK + place in `mods/` (or as a swap, depending on what RL accepts)
   - Deactivate = remove our file → RL falls back to its baked-in palette
5. **UI**
   - New `/palettes` route in the launcher
   - 20+20 color pickers per palette, name, save
   - Library of palettes, activate one at a time
6. **Restart hint** — same as items
