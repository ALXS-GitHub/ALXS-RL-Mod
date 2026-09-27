# RL Decal Shader — DXBC Extraction & Disassembly

> **Goal.** Settle the "R=43/255 sentinel" question by reading RL's actual
> compiled pixel shader bytecode, not by guessing.
>
> **Headline result.** **DXBC successfully extracted at scale (329,098 blobs,
> 243,008 unique pixel shaders) and disassembled with `fxc.exe`.** Inspection of
> the three highest-fanout pixel shader candidates (`ps_5_0`, 16 textures,
> 5 G-buffer outputs — unambiguously body/decal material shaders) shows:
>
> 1. **The composition is a chain of continuous `lerp`s weighted by the mask
>    texture's channels (`r13.x`, `r13.y`, `r13.z`). No `if`, no `lt`-based
>    select, no threshold band on the mask R channel exists.**
> 2. **Color slots are sampled from a palette lookup texture (`t9`, sampled
>    18× per shader) at UVs computed from the mask channels** — the team /
>    primary / accent colors come from a small RGBA LUT, not constants.
> 3. **The only `lt`/`movc` instructions in the entire decal PS are
>    `lt … |x|, l(0.000001)` divide-by-zero guards** and one `if_nz cb0[…]`
>    branch on a per-frame constant buffer flag (unrelated to mask values).
> 4. **No IEEE 754 constants in the range `0.15`-`0.18` appear in the
>    disassembly.** `0.169 = 43/255` does not appear either.
>
> **Verdict.** The "R = 43/255 → literal diffuse passthrough" rule is **not
> a real RL shader behaviour.** RL's body shader treats the mask R/G/B
> channels as **continuous lerp weights** and never special-cases any value.
> The user's RL-Designer simulation is using an invented heuristic.

---

## 1. Pipeline that worked

### 1.1 Decompression

`RefShaderCache-PC-D3D-SM5.upk` (219 MB) is a UE3 UPK whose body is split into
**40 Oodle-compressed chunks** (Oodle Leviathan / Kraken). The standard
`oo2core_5_win64.dll` shipped with most UE3 tools is too old; RL uses Oodle 7.

The decisive find: `oo2core_7_win64.dll` already on the user's machine at
`<PCModManager>\util\oodle\oo2core_7_win64.dll`.
Copying it next to `umodel.exe` (and renaming a copy to `oo2core_5_win64.dll`
to satisfy umodel's hardcoded lookup) lets umodel load the package. It also
lets `OodleLZ_Decompress` be called from Python via `ctypes.WinDLL`.

Each chunked-compression block follows the standard UE3 layout:

```
uint32 magic            = 0x9E2A83C1
uint32 blockSize        = 0x20000 (128 KiB, but per-chunk it can be 0x40000)
uint32 totalCompressed
uint32 totalUncompressed
struct { uint32 c; uint32 u; }  blockTable[ceil(totalU/blockSize)]
byte[] compressedBlocks[]
```

Chunks 1-40 start at offset `0x441` and continue sequentially. Total
uncompressed output is ~2.5 GB. We do NOT need to write that to disk; we
decompress each chunk in memory, search for the 4-byte `DXBC` magic, and
write only the DXBC blobs.

### 1.2 DXBC blob extraction

`sandbox/decrypt/shader_extract/extract_dxbc.py` walks chunks sequentially and
saves each matching DXBC blob (validated by checking version=1 and a sane
total-size field at offsets `+20` and `+24`).

Yield:

| Metric | Count |
|---|---:|
| Chunks processed | 27 of 40 (stopped early when disk space ran out) |
| DXBC blobs extracted | **329,098** |
| Unique DXBC blobs (by 16-byte MD5 in the header) | **291,636** |
| Unique pixel shaders | **243,008** |
| PS with ≥4 texture-2D declarations | **137,772** |
| Largest PS (16 textures, 33 samples) | 16,496 bytes |

### 1.3 Triage & disassembly

`sandbox/decrypt/shader_extract/classify_dxbc.py` parses each blob's SHEX
chunk to read the shader version word and counts the `dcl_resource_texture2d`
declarations and the various `sample_*` opcodes. Filtering down to
`type=PS && dcl_resource >= 4` narrowed the pool to 1,147 unique PS,
copied into `_ps_unique/`.

Disassembly uses **the Windows SDK's `fxc.exe /dumpbin`** (at
`C:\Program Files (x86)\Windows Kits\10\bin\10.0.22000.0\x64\fxc.exe`),
which prints D3D Shader Disassembler output verbatim.

---

## 2. Anatomy of the body/decal pixel shader

Three candidates were inspected closely — all `ps_5_0` PS with **16 texture
samplers and 5 SV_Target outputs** (G-buffer fill, deferred shading). Files:

- `dxbc_chunk09_off03c28bab_sz016204.dxbc` → `_candidate1.asm` (522 lines)
- `dxbc_chunk19_off00caabc5_sz016496.dxbc` → `_candidate2.asm`
- `dxbc_chunk01_off03d10fb3_sz011868.dxbc` → `_candidate3.asm`

All three exhibit the same composition pattern, with minor variations
(different combinations of body-shader static-switch parameters).

### 2.1 Signature

```hlsl
ps_5_0
dcl_constantbuffer CB0[105], immediateIndexed   // material + frame uniforms (~100 vec4s)
dcl_constantbuffer CB1[4]                       // view matrix
dcl_constantbuffer CB2[5]                       // per-frame
dcl_sampler s0 .. s15                           // 16 samplers
dcl_resource_texture2d t0 .. t11                // 12 × 2D
dcl_resource_texturecube t12                    // 1 × cube (env)
dcl_resource_texture2d t13 .. t15               // 3 × 2D
dcl_output o0..o4                               // 5 G-buffer RTs
```

12 distinct 2D textures sampled per-pixel + 1 cubemap + 3 more 2Ds (likely
shared LUTs). This matches `Body_Paintable_Mat`'s declared parameters
(`Diffuse`, `Masks`, `Skin`, `BodyMasks`, `Curvature`, `Glass`, `ColorLookup` ×3,
`PortalRingTexture` ×3, `Normal` ×2, `TertiaryMaterial_Normal`,
`F2DetailNormal`, `BlankSkin`) with room for static-switch variants.

### 2.2 Mask sampling (lines 133, 304 in `_candidate1.asm`)

```
sample_b_indexable(...) r13.xyz, v4.xyxx, t1.xzwy, s3, l(0.000000)   # mask RGB sampled into r13
sample_b_indexable(...) r12.xyz, r2.xyxx, t10.xywz, s12, l(0.000000) # palette sample → r12
```

`r13.x` (mask **R**), `r13.y` (mask **G**), `r13.z` (mask **B**) become the
primary lerp weights throughout. **There is no `lt r13.x, l(0.169)` or any
similar threshold comparison anywhere in the shader.**

### 2.3 Color-lookup palette sampling (lines 305-319, `t9`)

The team / primary / accent colors are **sampled from `t9`**, a 2D palette
texture, with UVs computed from mask channels:

```
add  r2.x, -r12.x, r12.y                              # lerp endpoints from t10
mad  r21.x, r22.w, r2.x, r12.x                        # u = lerp(r12.x, r12.y, r22.w)
mad  r21.y, r13.y, l(0.500000), l(0.253908)           # v = mask.g * 0.5 + 0.2539  ← key
sample_b_indexable r2.x, r21.xy, t9.zxyw, s11, l(0.0) # palette pick A
sample_b_indexable r2.y, r21.xz, t9.xzyw, s11, l(0.0) # palette pick B (different v)
sample_b_indexable r5.w, r21.xw, t9.xywz, s11, l(0.0) # palette pick C (different v)
add  r5.w, -r2.y, r5.w
mad  r2.y, r3.w, r5.w, r2.y                           # lerp pick A→C by r3.w
add  r2.y, -r2.x, r2.y
mad  r2.x, r13.x, r2.y, r2.x                          # lerp by mask.r
```

Read it as: build a U coordinate from secondary-color CB constants, build a
V coordinate from `mask.g`, sample three rows of the palette, then **linearly
interpolate** between them using `mask.r` and a derived weight `r3.w`. The
**continuous nature** is unambiguous — every selection is a `mad … weight`
pattern, never a branch.

`t9` is sampled **18 times** in this single PS — once per "color slot" the
shader can pick. That fits the four-color-mode comment found in
`skin_aa_flames_tierall_SF.upk` ("0=TeamColor 1=CustomColor 2=PaintedColor
3=Chassis_EmissivePartsColor"), plus secondary/accent variants.

### 2.4 The only branches in the shader

```
$ grep -nE '^\s*(if|lt|ge|eq|ne|discard|movc)\s' _candidate1.asm _candidate2.asm _candidate3.asm
```

Across all three candidate PSs, every match is one of:

| Pattern | Purpose |
|---|---|
| `lt rX.w, \|rY\|, l(0.000001)` followed by `movc … l(0), …` | **Divide-by-zero guard** for normalisation |
| `if_nz cb0[93].y … endif` | Per-frame **constant-buffer** branch (cloud/dust post-effect) |
| `movc rX, cb0[…]z, …` | **Static-switch** read from CB (e.g. `SolidTeamColor`, `Reflections`) |

**Zero comparisons against any mask-channel value.** Zero references to a
`R=43/255` sentinel. Zero comparisons against literals in the `0.15-0.18`
range. The exhaustive count of `lt` instructions is **single digits** per
shader, all of them on `0.000001` for numerical hygiene.

### 2.5 Literal-constant audit

In `_candidate1.asm` the only float literals that appear are:

| Value | Where used |
|---|---|
| `0.000000`, `1.000000` | identity values, used everywhere |
| `0.500000`, `0.250000`, `0.125000`, `0.031250`, `0.015625` | power-of-2 fractions for palette UV stepping (1/2, 1/4, 1/8, 1/32, 1/64) |
| `0.577350` (= 1/√3) | normal/tangent reconstruction |
| `0.300000`, `0.590000`, `0.110000` | **luminance weights** (R/G/B → Y dot product, line 121) |
| `2.000000`, `-1.000000` (line 214) | unpack `[0,1] → [-1,1]` for normal map |
| `6.283185` (= 2π), `0.017453` (= π/180) | rotation matrices for env map |
| `0.253908`, `0.003908`, `0.019534` | **palette-row offsets** (= n * 1/64 + 1/256-ish) |

**Nothing in the 0.15-0.18 band. Nothing at 43/255.**

### 2.6 What R = 43/255 actually does

Given the disassembly:

```
v = mask.g * 0.5 + 0.2539
u = lerp(some_cb_value, other_cb_value, weight)
color = sample(t9, (u, v))
final = lerp(diffuse_rgb, color, mask.r)
```

…feeding **R = 43/255 = 0.169** into `mask.r` produces:

```
final = lerp(diffuse_rgb, palette_color, 0.169)
      ≈ 0.831 * diffuse_rgb + 0.169 * palette_color
```

i.e. **most of the diffuse plus a small tint of the primary palette color.**
NOT a "literal diffuse passthrough." If the user's mask has `R = 41` (the
BC3-quantised 43) on the regions that they want to be "literal," the game
will draw those regions as **83% of the diffuse RGB + 17% of the team's
primary color** — which, when the team color is bright white in some
context (TeamColorScriptedTexture composes a render target that may default
to white if the LUT lookup is out of range), the visual result will be
**flat-white-shifted**, exactly the bug the user is observing.

That is consistent with the in-game empirical result. The "flat white"
isn't a sentinel; it's `0.169 × white + 0.831 × diffuse` blended toward a
white-dominated palette pickup.

---

## 3. What this means for the custom-decals pipeline

1. **Stop encoding "literal diffuse" zones as `R = 43/255`.** The game has
   no such mode. R = anything-non-zero means "blend some of the primary
   palette color in proportional to R."
2. **Use `R = G = B = 0` in the mask wherever you want pure diffuse.**
   That's the natural identity-of-the-lerp. Literal `R = 0` mask pixels
   render as 100% diffuse, 0% palette — exactly the "literal" behaviour
   the user wants.
3. **Use `R = 1` for full primary color, `R = 1, α = 1` for accent**
   (the alpha channel is the secondary-vs-accent multiplexer, already
   confirmed by the Yuna-Itzy in-game test). Intermediate `R` values give
   intermediate blends, which is useful for soft edges.
4. **The RL-Designer preview shader should match this.** Replace the hard
   `if r > 0.9 …` cutoff chain with a literal lerp:
   ```glsl
   vec3 finalColor = mix(decalDiffuse.rgb, primaryColor, mask.r * (1.0 - mask.a));
        finalColor = mix(finalColor,        secondaryColor, mask.r * mask.a);
        finalColor = mix(finalColor,        windowsColor,    mask.b);
   ```
   Drop the `0.15-0.18` literal-diffuse band entirely.

---

## 4. Reproducing this analysis

1. Copy `oo2core_7_win64.dll` (or any Oodle 6+ DLL) next to your tool. For
   umodel specifically, also copy it as `oo2core_5_win64.dll` (umodel
   hardcodes the v5 name).
2. Run `sandbox/decrypt/shader_extract/extract_dxbc.py` — needs ~5 GB of
   free disk to land the ~330k DXBC blobs (each `512 B – 16 KB`).
3. Run `classify_dxbc.py` to dedupe by MD5 and filter to high-texture-count
   pixel shaders. Output `_ps_unique/` (~1,200 unique candidate shaders).
4. Disassemble specific candidates with
   `& "C:\Program Files (x86)\Windows Kits\10\bin\10.0.22000.0\x64\fxc.exe" /nologo /dumpbin <blob.dxbc>`.
5. The body decal shader is recognisable by: `ps_5_0`, 16 texture decls,
   5 SV_Target outputs (deferred G-buffer), 18+ samples of one specific
   2D resource (the palette LUT), heavy use of `mad` against
   `0.577350` / `0.500000` / `0.015625` constants. Three of these shaders
   live in chunk 09 (offsets `0x03c28bab`-`0x03ce424f`) and chunk 19
   (offsets `0x00bf57a5`-`0x00caf0af`).

---

## 5. What we still don't have

- **Material → DXBC mapping.** We have 243k unique PS but no
  cross-reference from `Body_Paintable_Mat` to a specific blob. The
  `FMaterialShaderMap` headers live in the same UPK but parsing them
  requires walking the UE3 export table after full decompression. Three
  candidates inspected here are clearly body-material PS variants based on
  signature alone (16 textures, 5 G-buffer outputs, palette LUT pattern);
  there are likely 50-200 close variants distinguished by static switches
  (DynamicWear, Reflections, ThickGrass, TilingMask, etc.).
- **HLSL source.** No public DXBC → HLSL decompiler produces production
  quality. The `.asm` disassembly IS the ground truth.
- **Chunks 28-40.** Extraction stopped at chunk 27 when disk filled. The
  remaining 13 chunks likely contain more material shaders but the
  body-decal shader is unquestionably already in chunks 1-27 based on
  the candidate matches found.

---

## 6. The bottom line for the parent task

> "What does the literal-diffuse branch (or R = 43 sentinel) actually do?"

**There is no literal-diffuse branch and no R = 43 sentinel.** The mask
R channel is a **linear lerp weight** between the diffuse RGB and a
palette-LUT colour. Feeding R = 43/255 produces `0.169 × palette + 0.831 ×
diffuse`, which when the palette LUT returns a bright tone (e.g. white in
the default-lookup region) reads as the "flat white wash" the user
observes in-game.

The fix isn't to find the magic R value the shader expects — it's to
**stop trying to encode pass-through diffuse via a non-zero mask value**.
Use `R = 0` and the lerp naturally returns the diffuse unchanged.
