# ALXS-RL-Mod — project instructions

Desktop app (Tauri 2 + React 19 + Bun) to customise Rocket League: item swaps,
colour palette, custom decals, presets, map library, match tracker.
**Read `docs/ARCHITECTURE.md` before changing anything** — it is the contract
between modules (IPC command names, write layer, conventions).

## Non-negotiables

- **File-based only.** No DLL injection, no traffic interception, no
  certificates, no hosts file, no system proxy. Features that would need them
  (fake ranks, titles, name spoof…) are out of scope by design.
- **Local first.** The catalog comes from the user's install only (nothing
  bundled, see `catalog::game_db`). Network calls only on explicit user action, only through
  `base::security` (allowlist: map sources, tracker.gg).
- **Reversible.** Game files are written only through `game::writer`
  (manifest + content-addressed backups + hash checks). Prefer the
  `CookedPCConsole/mods/` override over replacing stock files.
- **Survives game updates.** Anything applied records the build
  fingerprint; `integrity` rebuilds from current stock files after an update.
- **Code quality.** Typed errors (`AppResult`), zod at the IPC boundary, no
  `any`, no hard-coded paths, tests for pure logic, i18n EN (default) + FR
  with identical keys.

## Layout

- `src-tauri/src/{base,game}` — foundation (errors, paths, config, security,
  install detection, process watcher, write layer).
- `src-tauri/src/{upk,palette,decals}` — cooked package engine and the
  features built on it.
- `src-tauri/src/{catalog,swap,presets,integrity}` — items side.
- `src-tauri/src/{maps,stats,extras}` — maps, tracker, launch, misc.
- `src/` — React front-end; one folder per feature in `src/features/`.
- `docs_rl/` — reverse-engineering notes on game formats (keep, extend).
- `site/` — GitHub Pages website (`python site/build.py`), `docs/RELEASING.md` — how to ship.

## Commands

- `bun install` then `bun run app` (Tauri dev) · `bun run prod` (installer)
- `bun run typecheck` · `bun run lint` · `cd src-tauri && cargo test`

## Game-format knowledge

- Cooked `.upk`: header tables AES-256-ECB encrypted (keys in
  `src-tauri/resources/keys/keys.txt`, gitignored), body = RL chunked zlib.
  See `docs_rl/palette.md`, `docs_rl/custom_decals*.md`.
- `TAGame.upk` holds the colour palettes (not AES-encrypted).
- Package renames must be length-preserving (null-pad) or re-point to a
  longer FName slot (see `upk::rename`).

## Style

- Git: conventional commits, commit identity from the global git config
  (never set a repo-local `user.email`).
- Keep `docs/ARCHITECTURE.md` in sync when adding a command or a module.
