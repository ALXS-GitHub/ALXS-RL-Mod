# RL-Designer mask-color threshold origin

## Where `[0.15, 0.18]` comes from

The range is a **+/- ~0.01 tolerance window around the literal byte value `0x2B/0xFF = 43/255 = 0.1686`**. `#2B0000` is the documented sentinel for "literal diffuse" (use decal color as-is, no team-tint). The threshold bracket is wide enough to absorb sRGB/PNG round-trip noise but tight enough to reject `0x00`, `0xFF`, and similar neighbors.

## Did Alexis derive `#2B0000` empirically?

**No.** The very first version of `docs/Designer-Guide.md` (commit `5d5a545`, 2025-07-19) lists the four sentinel colors but the last line reads:

> `TODO : 2b0000 for what color ???`

He already knew `#2B0000` existed as a convention in RL skin/mask images (likely from observing existing decals, AlphaConsole community tutorials, or the YouTube tutorial linked in `personal.md` — "Blender set up for Rocket League cars | Custom Decals") but didn't yet understand its semantics. The next day (commit `548ee04`, 2025-07-20) he filled in the line: *"What I call decal color here is the color that specify that we should use the exact color of the decal image, and that is not influenced by the team color."* The phrasing "what I call" indicates he coined the name but inherited the value.

`#FF000000`, `#FF0000`, `#0000FF` are well-known AlphaConsole / Psyonix skin-mask conventions (team color, secondary, windows).

## Shader history

- `e787cec` (2025-07-27) — thresholds appear fully formed inside `CarModel.tsx` inline shader. Same commit ships `default_body_skin.png` reference texture.
- `52fa924` (2025-07-27) — extracted to `colorReplacement.frag`, no value change.
- `7d8e234` (2025-07-28) — refactored into Three.js `onBeforeCompile` patch (`shaderSkinPatch.ts`).
- `dfba96b` (2025-08-02) — added curvature texture / car-color branch; literal-diffuse thresholds untouched.

**`[0.15, 0.18]` has never been adjusted since first commit.** No A/B tuning trail.

## In-game validation

Confirmed indirectly. `TODO.md` line 35 says:

> "Find a way in game to make universal decals paint finish and color changeable ... (in fact even decal with full 2B0000 can't change the paint finish...)"

So Alexis tested `#2B0000` in an actual decal in-game and observed it forces literal diffuse (no team tint, no paint finish) — exactly matching the shader behavior.

## Confidence & caveats

Author confidence is **high for the value itself, medium for the exact tolerance**. No comment cites a source. The 0.05 cap on G/B is generous. He noted (TODO) he doesn't fully understand interaction with paint finish (`#FF0000` secondary remains marked "not implemented yet" in `colors.ts`). The shader does not handle bodies whose skin mask uses unconventional values; only `_Body` mesh paths are patched.

## Key files

- `<projects>\RL-Designer\rl-designer\src\components\Model3D\patches\shaderSkinPatch.ts`
- `<projects>\RL-Designer\rl-designer\src\shaders\colorReplacement.frag`
- `<projects>\RL-Designer\docs\Designer-Guide.md` (lines 205-210)
- `<projects>\RL-Designer\TODO.md` (line 35 — in-game test)
