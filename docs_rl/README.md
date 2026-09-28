# Reverse-engineering notes

Working notes on Rocket League's cooked package formats (`.upk`, `.tfc`, coalesced
localisation), materials and textures, written while building the app. They are kept for
reference and are **not** documentation of the current code — see
[`docs/ARCHITECTURE.md`](../docs/ARCHITECTURE.md) for that.

- `package_encryption.md` — AES-ECB vs the "fully encrypted" AES-CTR packages (2026-08+).
- `item_recolor.md` — which colours of an item are data (particles, material instances) and can be recoloured.
- `palette.md` — `TAGame.upk` colour palettes (current approach).
- `lethamyr_maps_approach.md` — how community maps replace an arena.
- `custom_decals*.md`, `mic_diffuse_binding_finding.md`, `startup_upk_body_mic.md`,
  `rl_decal_shader_*.md` — **historical** (May 2026). Several conclusions in them are wrong:
  full-colour decals *are* possible file-only. The working approach uses the stock
  `Body_Paintable_Diffuse_Mat` material (`1_Diffuse_Skin` art + `2_Diffuse_Skin_Mask` zones),
  see `src-tauri/src/decals/`.
- `diag/` — throwaway Python probes used during that research (paths are examples).

No game file, extracted texture or encryption key is stored here.
