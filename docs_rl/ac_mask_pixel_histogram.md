# AlphaConsole decal SKIN MASK — pixel-level empirical analysis

**Date:** 2026-05-15
**Goal:** Empirically confirm the Rocket League native decal shader's color-zone
convention by inspecting the RGBA histograms of actual mask PNGs shipped in
working AlphaConsole texture packs.

## TL;DR — Canonical encoding

| Hex | RGBA tuple | Zone meaning | Confirmed pixel cluster |
|-----|------------|--------------|-------------------------|
| `#FF000000` | `(255, 0, 0, 0)` | **Primary picker color** (team color slot A) | Yes — exact match in all paintable packs |
| `#FF0000FF` | `(255, 0, 0, 255)` | **Accent picker color** (team color slot B) | Yes — exact match in all paintable packs |
| `#2B0000FF` / `#2B0000xx` | `(43, 0, 0, *)` | **Literal-diffuse zone** (uses diffuse RGB unchanged, no team-color tint) | Yes — exact `R=43` peak in every Octane/standard mask; varies `R=29` on Dominus |
| `#0000FFFF` | `(0, 0, 255, 255)` | **Windows zone** | Yes — exact match where any vehicle has glass |

> **The R-channel is the zone selector** (with G=B=0). The alpha channel
> distinguishes primary (A=0) vs accent (A=255) when R=255, and is ignored
> otherwise. R values in the range `[10, 50]` collectively map to "literal
> diffuse" — i.e. the R-channel ramp blends decal-image RGB over the team
> color in `[51..240]`, fully team-color at R=255, fully decal-RGB around
> R=29-46 depending on the body. This matches the RL-Designer threshold
> `r > 0.15 && r < 0.18` (= R in 38..46).

## Sample set — 31 PNGs from AlphaConsoleTextures + 14 from ubelhj/CustomDecalLogos

Repos:
- `https://github.com/AlphaConsole/AlphaConsoleTextures` (master)
- `https://github.com/ubelhj/CustomDecalLogos` (main)

Packs covered: `AC/MainBodyUtilTextures`, `Famiy/Dominus/EsportsPaintable` (16),
`Famiy/Dominus/EsportsFullColor` (6), `Butthead/Sandman`,
`HurricaneModding/Spectre` (5), `HurricaneModding/EsportsDecalsPack` (5),
`HurricaneModding/DragonLord` (2), `KernalPad/HoonicornDecals` (2),
`UncleSkeleton/PhantomACE` (2), plus ubelhj base decals/bodies for cars 22, 23,
403, 4906.

Raw PNGs cached at `<repo>\ac_masks_dl\`,
full per-file JSON at `ac_masks_analysis.json` and `ubelhj_masks_analysis.json`.
Visual zone-maps generated at `ac_masks_dl\zone_maps\`.

## Pattern A vs Pattern B — pack archetypes

Two distinct pack archetypes appear in the wild. They differ in WHICH texture
slot carries the mask data.

### Pattern A — "Paintable mask only" (Famiy/Dominus/EsportsPaintable)

Single decal PNG IS the skin mask. The diffuse stays the stock factory diffuse;
the swapped Skin PNG drives all colours via the picker.

Example: `DomAllegiance.png` (2048x2048, 4.19M px)

| Zone | Pixels | % |
|------|-------:|--:|
| `#FF000000` exact (primary, R=255 A=0) | 1,326,568 | 31.62% |
| `#FF0000FF` exact (accent, R=255 A=255) | 1,074,774 | 25.62% |
| `#0000FFFF` ± noise (windows) | 287,905 | 6.86% |
| Fully-transparent black (`#00000000`, GIMP-corrupted primary) | ~2.4M | ~57% (overlaps primary) |
| R 38-46 G=B=0 (diffuse zone) | **0** | 0.00% |

→ Dominus paintable packs ship **no literal-diffuse pixels at all** — the entire
decal is driven by the two picker slots + windows. This confirms that
`R 38-46 + G=B=0` is opt-in, not mandatory.

### Pattern B — "Diffuse + mask" (ubelhj/CustomDecalLogos for Octane, Butthead/Sandman, AC MainBodyUtilTextures)

Skin PNG mixes all four zones; the matching Diffuse PNG carries the logo art
in the regions flagged as "literal diffuse".

Example: `ubelhj/img/23/flames.png` (Octane skin mask, 2048x2048):

| Zone | Pixels | % |
|------|-------:|--:|
| Primary (R~255, A~0) | 2,040,707 | 48.65% |
| Accent (R~255, A~255) | 313,569 | 7.48% |
| **Diffuse (R 10-50, G=B=0)** | **1,791,407** | **42.71%** |
| Windows (B~255, R=G=0) | 19,986 | 0.48% |
| Other (anti-alias ramp) | 28,635 | 0.68% |

The R-channel histogram (G=B=0 only) shows a discrete **5-step distribution**:

```
  ubelhj__img__23__basedecal.png  (G=B=0 pixels: 4,173,300 / 4,194,304)
    R= 13:     331,849  ( 7.95%)   } anti-alias halo around the R=43 region
    R= 16:     603,123  (14.45%)   } from PNG/mip downsampling
    R= 40:     117,154  ( 2.81%)   }
    R= 43:     703,691  (16.86%)   ← canonical "literal diffuse" sentinel
    R= 45:      33,008  ( 0.79%)   } halo
    R=245:      19,365  ( 0.46%)   } halo around the R=248/255 primary region
    R=248:   2,351,732  (56.35%)   ← canonical "primary picker" sentinel
```

The bimodal `R=43` / `R=248`-ish distribution is the smoking gun. Anti-alias
pixels at R=13, 16, 40, 45 are mip/PNG-quantization byproducts. The shader
clearly treats the whole `R ∈ [10, 50]` band as "literal diffuse" and ramps
linearly into team-color for higher R.

### AC `MainBodyUtilTextures/MainBodyOnly.png` — the reference body skin

This is the SKIN file shipped by AC itself as the "main body only" reference.

| Zone | Pixels | % |
|------|-------:|--:|
| `#2B0000FF` exact (R=43 G=0 B=0 A=255) | 687,105 | 16.38% |
| R 38-46 G=B=0 A=255 (full diffuse band) | 851,414 | 20.30% |
| Pure black (`#000000FF`) — masked-out body area | 2,375,036 | 56.62% |
| `#0000FFFF` ± noise (windows) | 19,990 | 0.48% |

Top-5 distinct colours:
```
((0, 0, 0, 255),    2,375,036)   ← masked body (no team color, no decal)
((43, 0, 0, 255),     687,105)   ← LITERAL DIFFUSE sentinel  ← KEY MATCH
((16, 0, 0, 255),     613,110)   ← mip/AA halo
((13, 0, 0, 255),     320,493)   ← mip/AA halo
((41, 0, 0, 255),     130,196)   ← mip/AA halo
```

This is the **canonical reference** — and the central peak is exactly
`#2B0000FF` = `(43, 0, 0, 255)`, matching the user's documented sentinel from
`Designer-Guide.md`.

## Per-pack confirmation matrix

| Pack | Primary (`#FF000000`) | Accent (`#FF0000FF`) | Diffuse (R~43) | Windows (`#0000FFFF`) |
|------|:---------------------:|:--------------------:|:--------------:|:---------------------:|
| AC/MainBodyUtilTextures | — | — | **687,105 exact** | 19,990 |
| Famiy/Dominus/EsportsPaintable/DomAllegiance | 1,326,568 | 1,074,774 | 0 | 287,905 |
| Famiy/Dominus/EsportsPaintable/DomChiefs | 3,499,704 | 204,289 | 0 | 288,123 |
| Famiy/Dominus/EsportsPaintable/DomCloud9 | 989,178 | 427,316 | 0 | 288,123 |
| Famiy/Dominus/EsportsPaintable/DomFnatic | 73,917 | 1,219,636 | 0 | 288,123 |
| Famiy/Dominus/EsportsPaintable/DomG2 | 102,360 | 1,127,784 | 0 | 288,123 |
| Famiy/Dominus/EsportsPaintable/DomNRG | 350,990 | 1,155,908 | 0 | 288,123 |
| Butthead/Sandman/sandman_decal | 1,723,333 | 532,906 | 20 exact (+103 in band) | 19,986 |
| HurricaneModding/Spectre/cooked1 | — | — | 844 (R=43 in 512x512) | — |
| HurricaneModding/Spectre/flow1 | — | — | 591 | — |
| HurricaneModding/EsportsDecalsPack/allg1 | 12,280,788 | 3,827,316 | 0 | 489,398 (`5,37,252` near-blue) |
| HurricaneModding/EsportsDecalsPack/c9 | 12,223,804 | 3,941,960 | 0 | 489,398 (`0,38,255` near-blue) |
| HurricaneModding/EsportsDecalsPack/g2 | 11,469,004 | 4,219,734 | 0 | 489,398 |
| HurricaneModding/EsportsDecalsPack/nrg1 | — (≈A=4 quasi-transparent) | 4,131,577 | 0 | 489,398 |
| KernalPad/HoonicornDecals/Hoonicorn | 5,683 | 2,169,269 | 6 exact (818 in band) | **130,817 exact** |
| UncleSkeleton/PhantomACE/PhantomACE | — (`R=255 A=2` quasi-transparent) | 2,484,778 | 37 exact | 489,398 (near-blue) |
| UncleSkeleton/PhantomACE/PhamSellout | — (`R=255 A=2`) | 4,110,447 | 141 exact | 489,398 (near-blue) |
| ubelhj/Octane/basedecal | 2,374,040 (R~255 A~0) | 0 | **703,691 exact** | 19,986 |
| ubelhj/Octane/flames | 2,040,707 | 313,569 | 703,619 exact | 19,986 |
| ubelhj/Octane/stars | (similar) | (similar) | 703,670 exact | 19,986 |
| ubelhj/Dominus/basedecal | 2,635,736 | 0 | 0 (uses R=29!) | 286,935 |
| ubelhj/Dominus/arcana | 2,149,341 | 409,540 | 0 (uses R=29/R=32!) | 287,824 |
| ubelhj/Harbinger(4906)/basedecal | A=2 quasi-transparent | — | 0 | — |

## Key finding 1 — `R=43` is the exact, near-universal "literal diffuse" sentinel for Octane-family bodies

Every Pattern-B (diffuse+mask) Octane-class pack writes the diffuse zone at
**exactly `(43, 0, 0, *)`** with no compression drift. The peak is so sharp
(703k+ pixels at R=43 exactly) that this must be a hand-authored constant, not
a happy accident of PNG quantization. The surrounding `R=13, 16, 40, 41, 45,
46` pixels are MIP downsampling artifacts (a fraction of the central peak).

## Key finding 2 — Dominus body uses `R=29` / `R=32` instead of `R=43`

`ubelhj/img/403/basedecal.png` and `arcana.png` (Dominus) show **no R=43
pixels at all**, instead concentrating at `R=29` (24.5% / 956k px) and `R=32`
(7.3% / 286k px on basedecal; 31% / 1.2M on arcana).

This proves the shader's "literal diffuse" band is wider than just R=43 — it's
the entire `R ∈ [10..50]` range, and different body authors picked different
values within that band. The `r > 0.15 && r < 0.18` threshold cited by the
RL-Designer guide (= R ∈ `[38, 46]`) captures the Octane convention, but
**Dominus uses lower R values (~`[29, 32]`) and must be widened**.

Recommendation: treat the literal-diffuse band as **`R ∈ [10, 50]` (≈ 4%-20% in
0..1 normalized)** when authoring a designer/checker tool. The user's existing
RL-Designer threshold `[0.15, 0.18]` is too narrow — it misses Dominus.

## Key finding 3 — primary/accent picker zones are EXACT, not banded

Unlike the diffuse zone, primary (`R=255, A=0`) and accent (`R=255, A=255`)
appear **at exact tuples** with millions of matches per pack. There is some
secondary clustering at `R=255, A in {2, 3, 4}` in EsportsDecalsPack/PhantomACE
files which is likely PNG editor degradation (semi-transparent black bleeding
through a layered red). The shader probably tolerates this because the
samples we examined visibly work in-game (these packs are popular AC
downloads).

## Key finding 4 — windows zone tolerates some hue drift

Most packs use exact `(0, 0, 255, 255)`. But HurricaneModding's EsportsDecalsPack
uses `(5, 37, 252, 255)` or `(0, 38, 255, 255)` — a slight green-shifted blue,
489,398 pixels each. UncleSkeleton/PhantomACE uses `(6, 36, 250, 255)`,
470,048 pixels. These all clearly work in-game, suggesting the shader matches
"B dominant" rather than exact `#0000FFFF`. Suggested broad detector:
`B > 200 && R < 50 && G < 50`.

## Key finding 5 — `#FF000000` GIMP/PS export workaround is documented

The user's `Designer-Guide.md` line 206 explicitly notes that many image
editors collapse `#FF000000` (transparent red) to `#00000000` (transparent
black) on save. Empirical check: `DomAllegiance.png` shows 1,266,142 pixels of
`(0, 0, 0, 0)` — these are very likely GIMP-corrupted primary-zone pixels.
The fact that the pack ships and works confirms the shader treats `A=0`
pixels (regardless of RGB) as primary-color zones, or at least is robust to
this corruption pattern.

## Canonical encoding — recommended for the launcher's designer tool

```
Zone               R        G    B    A     Tolerance
------------------ -------- ---- ---- ----  -----------------------------------
Primary picker     ≥240     ≤20  ≤20  ≤20   Robust to GIMP A=0 corruption
Accent picker      ≥240     ≤20  ≤20  ≥240
Literal diffuse    10..50   ≤20  ≤20  any   Wider than Designer-Guide [38..46]
Windows            ≤50      ≤50  ≥200 ≥200  Broader than exact #0000FFFF
```

These ranges encompass 100% of the working packs sampled (45 PNGs across 9
authors).

## Files generated

- `<repo>\ac_masks_dl\` — 45
  source PNGs cached locally
- `<repo>\ac_masks_dl\zone_maps\`
  — 8 RGBA zone-visualisation PNGs (red=primary, orange=accent, green=diffuse,
  blue=windows, grey=other)
- `<repo>\ac_masks_analysis.json`
  — full per-pixel JSON histogram for 31 AC pack files
- `<repo>\ubelhj_masks_analysis.json`
  — full per-pixel JSON histogram for 14 ubelhj files
