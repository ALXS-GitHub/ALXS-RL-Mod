# RL decal/paint shader convention — cross-tool comparison

Survey of every public Rocket League preview/designer tool that actually implements (not just installs) the paint+decal shader.

## TL;DR

Only **two** independent open-source projects implement the RL paint shader. They use **fundamentally different conventions** for the mask texture and **no shared threshold values**. There is no single agreed-upon "[0.15, 0.18] literal-diffuse" range outside RL-Designer itself.

| Tool | Source | Primary R range | Accent R/A range | "Diffuse" R range | Windows B range | Author's source |
|------|--------|-----------------|------------------|-------------------|-----------------|-----------------|
| **RL-Designer** (ALXS) | [`shaderSkinPatch.ts`](https://github.com/ALXS-GitHub/RL-Designer/blob/main/rl-designer/src/components/Model3D/patches/shaderSkinPatch.ts) | `r>0.9 && g<0.1 && b<0.1 && a<0.1` (FF0000 transparent) | `r>0.9 && g<0.1 && b<0.1 && a>0.9` (FF0000 opaque) — used as "exact decal color" | `r in [0.15,0.18] && g<0.05 && b<0.05` (#2B0000) | `b>0.9 && r<0.1 && g<0.1` (#0000FF) | Community/AlphaConsole convention (`#FF0000`, `#0000FF` well-known); `#2B0000` value inherited from RL decal community, name coined by author. No commit explanation. |
| **rl-loadout-lib** (Longi94, three.js) | [`static-decal-material.ts`](https://github.com/Longi94/rl-loadout-lib/blob/master/src/webgl/static-decal-material.ts) | `rgbaMap.r > 0.58823529411` (150/255) — continuous blend (not hard zone) | `rgbaMap.g` (continuous), and `decalMap.a` (continuous) once primary mask is on | **No "literal diffuse" zone** — diffuse is what falls outside primary | Not handled in decal material (handled by per-body materials, e.g. glass shader elsewhere) | Reverse-engineered from extracted RL game UPK files (companion `rl-loadout` repo has `extract_static_skins.py` umodel extractor) |
| **rl-loadout-lib** GreyCarMaterial (K.I.T.T.) | [`grey-car-material.ts`](https://github.com/Longi94/rl-loadout-lib/blob/master/src/3d/body/grey-car-material.ts) | `rgbaMap.r > 0.16470588235` (42/255) — continuous blend | `rgbaMap.a` blends fixed grey | n/a | n/a | Same |
| **rl-loadout-lib** FelineMaterial | [`feline-material.ts`](https://github.com/Longi94/rl-loadout-lib/blob/master/src/3d/body/feline-material.ts) | `rgbaMap.r > 0.16470588235` (42/255) | `1.0 - rgbaMap.a` blends primary | n/a | n/a | Same |
| **rl-loadout-lib** Berry/Dark/Eggplant | various | n/a (no R threshold) | uses `rgbaMap.a` or `rgbaMap.g` for blending | n/a | n/a | Same |
| **rlviewer.joudcazeaux.fr** | minified JS bundle | does NOT implement paint shader — just calls `material.map = customTexture` | — | — | — | n/a (textures are uploaded as flat albedo) |
| **AlphaConsole Electron UI** | [github.com/AlphaConsole/AlphaConsoleElectron](https://github.com/AlphaConsole/AlphaConsoleElectron) | n/a — no preview, UI-only; rendering done by closed-source `.dll` injected into RL | — | — | — | Native RL UberSkin shader (closed-source) |
| **BakkesMod plugin ecosystem** | [bakkesplugins.com](https://bakkesplugins.com/plugins?search=preview) | n/a — no preview plugins found that re-implement the paint shader | — | — | — | — |
| **CustomDecalLogos** (ubelhj) | [github](https://github.com/ubelhj/CustomDecalLogos) | image-compositing only (Jimp PNG overlay); no shader, no zone interpretation | — | — | — | — |
| **CustomCar** (smallest-cock) | github | C++ BakkesMod plugin; relies on in-game shader, no re-implementation | — | — | — | — |

## Detailed source dump

### RL-Designer (the user's tool — baseline)

```glsl
if (skinColor.r > 0.9 && skinColor.g < 0.1 && skinColor.b < 0.1 && skinColor.a < 0.1) {
    finalColor = mainTeamColor;                        // #FF0000 transparent → primary team color
}
else if (skinColor.r > 0.9 && skinColor.g < 0.1 && skinColor.b < 0.1 && skinColor.a > 0.9) {
    finalColor = decalColor.rgb;                       // #FF0000 opaque → decal RGB as-is
}
else if (skinColor.r > 0.15 && skinColor.r < 0.18 && skinColor.g < 0.05 && skinColor.b < 0.05) {
    finalColor = decalColor.rgb;                       // #2B0000 → "literal diffuse" zone
}
else if (skinColor.b > 0.9 && skinColor.r < 0.1 && skinColor.g < 0.1) {
    finalColor = windowsColor;                         // #0000FF → window glass
    isWindow = true;
}
```

- Hard branches (`if/else`), no continuous blending.
- `[0.15, 0.18]` brackets the byte value `0x2B = 43/255 = 0.1686` with ±0.01 sRGB round-trip tolerance.
- See [`rl_designer_threshold_origin.md`](./rl_designer_threshold_origin.md) for evolution.

### rl-loadout-lib StaticDecalMaterial (the most thorough OSS implementation)

```glsl
vec4 texelColor    = texture2D(map, vUv);          // diffuse texture
vec4 rgbaMapColor  = texture2D(rgbaMap, vUv);      // skin/mask texture
vec4 decalMapColor = texture2D(decalMap, vUv);     // decal RGBA

if (bodyPainted == 1) {
    texelColor.rgb = blendNormal(texelColor.rgb, bodyPaintColor.rgb, 1.0 - rgbaMapColor.r);
}

float primaryMask = rgbaMapColor.r;
if (isBlank == 0) {
    primaryMask = decalMapColor.r;                 // for non-blank decals, the decal's own R is the primary mask
}

if (primaryMask > 0.58823529411) {                 // 150/255 — gating threshold
    texelColor.rgb = blendNormal(texelColor.rgb, primaryColor.rgb, primaryMask);
}

if (isBlank == 0) {
    if (primaryMask > 0.58823529411) {
        texelColor.rgb = blendNormal(texelColor.rgb, accentColor.rgb, decalMapColor.a);
    }
    if (painted == 1) {
        texelColor.rgb = blendNormal(texelColor.rgb, paintColor.rgb, decalMapColor.g);  // titanium-white-style paint
    }
}

// body-side accent
texelColor.rgb = blendNormal(texelColor.rgb, accentColor.rgb, rgbaMapColor.g);
```

**Channel semantics (Longi94 convention):**
- `rgbaMap.r` → primary team color mask (continuous, gated at >0.588)
- `rgbaMap.g` → accent color mask on body
- `rgbaMap.a` → (used differently per body)
- `decalMap.r` → overrides primary mask when a decal is applied
- `decalMap.g` → paint finish mask (titanium white, etc.)
- `decalMap.a` → accent color zone within decal

**No "literal diffuse" zone exists** — anything that doesn't trip the >0.588 primary mask retains its base texel color, which IS the literal diffuse.

## Comparison: RL-Designer vs rl-loadout-lib

| Aspect | RL-Designer | rl-loadout-lib |
|---|---|---|
| Architecture | hard if/else branches | continuous `blendNormal()` accumulation |
| Primary trigger | `r>0.9 && a<0.1` (very specific #FF0000 transparent) | `r > 0.588` (any reasonably-red pixel) |
| Literal diffuse | dedicated zone at `r ≈ 0x2B/255` | implicit (whatever is below the primary gate) |
| Windows | dedicated `b>0.9` branch with glass shader | not handled (windows are a separate body submesh) |
| Source of truth | Community/tutorial convention + in-game trial | umodel-extracted UPK assets + RL community wiki |
| Per-body variants | one universal shader | 6+ specialized materials (Dark Car, Berry, Feline, K.I.T.T., Eggplant…) |

## Consensus on the [0.15, 0.18] literal-diffuse range

**There is no consensus.** RL-Designer is the ONLY public tool that implements a dedicated literal-diffuse zone at `r ≈ 0x2B`.

Independent corroboration FOR the value:
1. The hex `#2B0000` is documented in RL-Designer's own `Designer-Guide.md` as community-inherited.
2. The author tested it in-game and confirmed it forces literal diffuse (no team tint, no paint finish) — see TODO.md line 35.
3. The byte value `0x2B = 43` is suspiciously specific (not 0x20, 0x30, 0x40) — strongly suggesting it's a sentinel value chosen by Psyonix or an RL artist convention.

Independent corroboration AGAINST a hard-bracketed range:
1. rl-loadout-lib's shader has NO equivalent zone — primary masking is continuous, not bracketed.
2. The native game `UberSkin_V1_Mat` exposes parameters `Decal1`, `Decal2`, `BodyMasks`, `TeamColor`, `CustomColor`, `PaintColor`, `TrimColor` but the actual decoding is baked into compiled HLSL (closed source). The material props show `Decal1Threshold = 0.5`, `Decal2Threshold = 0.5`, and `DecalBlendHeightThreshold = 0.5` — none of these are 0.16/0.17.
3. The rl-loadout-lib K.I.T.T. material uses `r > 0.16470588235` (42/255 = `0x2A/255`) which is ALMOST identical to RL-Designer's `0x2B` — but as a primary-color *gate*, not a literal-diffuse zone.

## Most reliable source

**rl-loadout-lib** is the most reliable because:
- It is built directly from umodel-extracted RL UPK assets (companion `rl-loadout` backend has the extraction tools).
- It maintains per-body specialized materials reflecting the actual UDK material instance graph.
- Its primary-color gate at `0.165` (42/255) aligns with the byte value Psyonix appears to use as a sentinel — supporting the idea that `0x2B` is real but not specifically a "literal diffuse" sentinel.

**RL-Designer's interpretation may be incorrect or oversimplified:**
- The `[0.15, 0.18]` bracket does mask the byte `0x2B`, but in rl-loadout-lib's continuous-blend model, `r = 0x2B/255 = 0.169 < 0.588` would simply mean "no primary tint applied" — i.e. the pixel keeps its literal diffuse color. This is the SAME observable outcome RL-Designer hard-codes as a special branch.
- In other words: **the user's `[0.15, 0.18]` zone is functionally redundant** with rl-loadout-lib's "fall through the >0.588 gate" behavior. The user could replace the third `else if` with no zone at all and let the diffuse fall through, matching rl-loadout-lib semantics.

## Recommendation

If the user wants a defensible shader, port from **rl-loadout-lib StaticDecalMaterial**. It is:
- continuous (no brittle hard ranges)
- derived from actual game files (extracted UPK)
- handles per-body variants (Berry, Dark Car, Eggplant, K.I.T.T., Feline)
- supports paint finish (titanium white) which RL-Designer does not implement
- MIT-licensed three.js library, drop-in ready

## Sources

- RL-Designer: https://github.com/ALXS-GitHub/RL-Designer
- rl-loadout-lib: https://github.com/Longi94/rl-loadout-lib (npm: `rl-loadout-lib`)
- rl-loadout (companion backend + extractor): https://github.com/Longi94/rl-loadout
- rlviewer (Joud Cazeaux): https://rlviewer.joudcazeaux.fr/ — confirmed via JS bundle inspection that it does NOT implement the paint shader
- AlphaConsole repos: https://github.com/AlphaConsole — no shader source (renderer is closed `.dll`)
- BakkesPlugins decal preview search: https://bakkesplugins.com/plugins?search=preview — zero preview plugins implement the shader
- GitHub code search for `mainTeamColor windowsColor`: only matches RL-Designer (the user's own repo)
- GitHub code search for `"skinColor.r > 0.15"`: only matches RL-Designer
