# Personal edition

The app has two editions built from the same sources:

| | Public edition | Personal edition |
|---|---|---|
| Who | Everyone (GitHub releases, auto-update) | The maintainer only, built locally |
| MMR features | **None** | Your MMR per playlist, session delta, MMR tab (any player), lobby ranks, overlay MMR |
| Source of the MMR | — | tracker.gg, through a hidden WebView2 window |
| Updates | Signed auto-updates from GitHub releases | None (rebuild from the sources) |
| Build | `bun run prod` / the release workflow | `bun run prod:personal` |

## Why the MMR is not public

Nothing official exposes other players' MMR, and not even your own after a
match (research of 2026-09-29):

- **tracker.gg** has no Rocket League API: the developer docs list Apex
  Legends and The Division 2 only, and staff refuse Rocket League keys.
  Its [terms](https://thetrackernetwork.com/home/tos) forbid automated
  access: *"You may not use manual or automated software, devices, or other
  processes to 'crawl,' 'scrape,' or 'spider' any page"*, and bypassing the
  robot exclusion headers. The personal edition does exactly that, so it
  must never be distributed.
- **Psyonix/Epic**: no public skill API (`api.rocketleague.com` was never
  opened and no longer resolves). The official Stats API has no rank or
  MMR field. The in-game MMR display (v2.66) shows only your own MMR, only
  to you.
- **Local files**: the save file (`.save`, AES with a key of `keys.txt`)
  holds your tier per playlist but no MMR; `Launch.log` writes the party
  leader's internal MMR (`PartyLeaderMMR`, displayed ≈ ×20 + 100) when a
  queue starts. Replays carry no rank.
- **Proxy (VelocityRL style)**: a trusted root certificate, hosts file
  redirection and a system proxy around the certificate pinning of PsyNet.
  Clear breach of the Epic terms ("circumvent … security"), ban risk, and
  VelocityRL even ships the private keys of its Epic login certificates.
  Out of scope for both editions (`CLAUDE.md`, non-negotiables).

The public edition therefore has no MMR at all. The personal edition keeps
the tracker.gg features for the maintainer's own use, at their own risk.

## How it is gated

Everything MMR is in two folders, compiled into the personal edition only:

| Side | Folder | Gate |
|---|---|---|
| Rust | `src-tauri/src/personal/` (`browser`, `mmr`, `lobby`, `ratings`, `service`, `commands`) | Cargo feature `personal` (`#[cfg(feature = "personal")] pub mod personal;` in `lib.rs`) |
| Front-end | `src/personal/` (`api`, `queries`, `i18n`, `locales/`, `components/`) | Vite constant `__ALXS_PERSONAL__` (`ALXS_EDITION=personal`), lazy imports |

Rules:

1. **Public code never imports `personal` directly.**
   - Rust: the match tracker reaches it only through `stats::hooks`
     (`init`, `on_game_session_start`, `on_live`, `on_match_recorded`,
     `on_session_reset`), which are empty functions without the feature.
   - Front-end: only through
     `__ALXS_PERSONAL__ ? lazy(() => import("@/personal/tracker")…) : null`.
     `__ALXS_PERSONAL__` is replaced by a literal at build time, so the
     public bundle contains neither the branch nor the chunk.
2. **The few gated items outside those folders** carry a
   `#[cfg(feature = "personal")]` or a `__ALXS_PERSONAL__` check:
   - `lib.rs`: the `personal` module, its commands in `generate_handler!`,
     and no updater plugin in the personal edition;
   - `base::security`: `Purpose::Ratings`, `RATING_HOSTS`, tracker.gg in
     `BROWSE_HOSTS`;
   - `base::error`: `AppError::RatingsDisabled`;
   - `base::config`: `tracker.platform`, `tracker.favorites`,
     `tracker.onlineRatings` (the public build drops them when it saves
     `config.json`);
   - front-end: the MMR tab (`TrackerPage`), `SessionRatings`
     (`SessionView`), `OverlayRatings` (`OverlayRoot`), the nav keywords,
     the update check (`UpdateDialog`, Settings → About) and the sidebar
     edition label.
3. **Tauri config**: `tauri.conf.json` is the public one (explicit
   capabilities `default` + `overlay`, CSP without `trackercdn.com`).
   `src-tauri/tauri.personal.conf.json` is merged on top by
   `--config` for the personal edition: personal front-end build commands,
   the `personal` capability (opening tracker.gg links), the tracker CDN in
   `img-src`, and no updater artifacts.
4. **Strings** of the personal edition live in `src/personal/locales/`
   (namespace `personal`, registered by `src/personal/i18n.ts`), not in
   `src/features/*/locales/`, so they are not in the public bundle. The EN/FR
   parity test covers them.
5. **Data**: `%LOCALAPPDATA%\ALXS-RL-Mod\stats\ratings.json` (MMR lines;
   taken over once from the old `session.json`), event
   `personal://ratings`, commands `mmr_lookup` and `ratings_state`.

## Guards (CI)

- `ci.yml`
  - *Public bundle guard*: builds the public front-end and fails if `dist`
    contains `personal://ratings`, `mmr_lookup`, `ratings_state` or a
    tracker.gg host.
  - Clippy and tests run with and without `--features personal`, so the
    personal edition keeps compiling.
- `release.yml`
  - Refuses to run with `ALXS_EDITION` set, runs the bundle guard before
    building.
  - *Public binary guard* after the build: fails if the executable contains
    `api.tracker.gg`. The draft release is already created at that point:
    delete it if this guard ever fails.

Checked locally on 2026-09-29: public bundle and binary contain none of the
markers; the personal build has them in a separate `tracker-*.js` chunk and
in the binary.

## Build and install the personal edition

Requirements: the usual toolchain (`bun install`, Rust stable).

```sh
bun run app:personal     # dev (tauri dev --features personal + personal front-end)
bun run prod:personal    # installer
```

- The installer is written to
  `src-tauri/target/release/bundle/nsis/ALXS-RL-Mod_<version>_x64-setup.exe`,
  the same path as a public build. **Never upload it** (not to a release,
  not to a chat): build it, install it, move on.
- Same app identity as the public edition (`com.alxs.rlmod`, same data
  folder): installing it replaces the public app on this machine, and
  installing a public release replaces it back.
- It never updates itself (no updater plugin, no update check). To update,
  pull and rebuild. Settings → About says so; the sidebar shows
  "Personal edition".
- Then, in the app: Tracker → MMR tab → turn online ratings on, enter your
  platform and id.

## Adding a personal feature

1. Rust: code in `src-tauri/src/personal/`; if the tracker must call it,
   add a hook in `stats::hooks` (both `cfg` variants); new commands go in
   `personal::commands` and in `generate_handler!` with
   `#[cfg(feature = "personal")]`.
2. Front-end: code in `src/personal/`, exported from an entry module
   (e.g. `src/personal/tracker.ts`); public code loads it only behind
   `__ALXS_PERSONAL__`. Strings in `src/personal/locales/`.
3. Run both `cargo clippy --all-targets` and
   `cargo clippy --all-targets --features personal`, then `bun run build`
   and check the guard pattern finds nothing in `dist`.
