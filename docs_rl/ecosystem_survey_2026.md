# Rocket League Modding Ecosystem Survey — May 2026

Author: ecosystem-research agent
Context: EAC reintroduced 2026-04-28. BakkesMod officially shut down. ALXS-RL-Mod
team needs every viable **file-only** path for cosmetics, especially **literal-RGB
custom decals** (i.e. arbitrary user-supplied images, not just primary/accent tints).

---

## 1. State of the world after 2026-04-28

### 1.1 EAC rollout, confirmed facts

- EAC went live **April 28, 2026** (Season 22 launch). Mandatory for *all* online
  play (ranked, casual, tournaments, private lobbies).
- Psyonix added a toggle: launch with **EAC off** for offline / LAN / training /
  replays. With EAC off you keep the mod ecosystem but can't queue online.
- Several BakkesMod features were absorbed natively: MMR display, custom training
  randomization, free-play team colors, flip-reset indicator, etc.
- Steam Workshop maps work **with or without EAC** (Psyonix-blessed path).

Sources:
- https://www.rocketleague.com/en/news/easy-anti-cheat-comes-to-rocket-league-on-pc-today
- https://dotesports.com/rocket-league/news/bakkesmod-rocket-league-anti-cheat-update
- https://allthings.how/easy-anti-cheat-in-rocket-league-release-date-and-what-changes/

### 1.2 BakkesMod — officially dead

> "The architecture itself is the problem" — BakkesMod team, closing post,
> April 2026 (quoted in rlpeak.com/blog/is-bakkesmod-still-working.html).

- The process-injection design is fundamentally incompatible with EAC. The team
  declined to rebuild from scratch.
- BakkesMod 2 (a *limited* community fork) ships only as an **offline tool** —
  default plugin repo last updated 2026-04-24 on GitHub. It will only ever work
  with EAC disabled, and therefore drift out of compatibility over time.
- **AlphaConsole** — the BakkesMod plugin powering 99% of custom decal installs
  prior to EAC — depends entirely on BakkesMod injection. No replacement exists
  as of 2026-05-15. AlphaConsole.io continues to host content but the runtime is
  effectively dead for online play.

Sources:
- https://bakk.es/articles/bakkesmod-eac.html
- https://rlpeak.com/blog/is-bakkesmod-still-working.html
- https://www.trophi.ai/post/rocket-league-updates-after-eac-bakkesmod-ballchasing-and-whats-actually-different-now

### 1.3 What custom decals "used to" mean

The pre-EAC stack everyone used:
1. BakkesMod injects into `RocketLeague.exe`.
2. AlphaConsole plugin hooks D3D texture loading.
3. User drops PNG files in `data/acplugin/DecalTextures/<set>/`.
4. AlphaConsole swaps the runtime texture at draw time → user sees full literal
   RGB on their car. Other players see vanilla.

That entire pipeline is dead online post-EAC. Every "tutorial 2024–2026"
indexed below uses this stack. None of them target a file-only path.

---

## 2. Currently-working file-only modding paths

### 2.1 RLPeak — the only named survivor

- Public site: https://rlpeak.com
- Source: https://github.com/rlpeak/rlpeak (Tauri + React + Rust, **same stack
  as us**)
- Wiki: https://rlpeak.com/wiki.html
- Roadmap/SDK: https://rlpeak.com/sdk.html

**How it works (per its docs):**
- Zero runtime injection. "Never attaches to the game process."
- Operates entirely on local files. For each apply: backs up the original to
  `AppData/Backups/originals/`, swaps the file, done.
- V1.1.0 ships: catalog-based **decal / wheel / boost** swap (curated content
  only — no user PNG upload), RocketStats overlay (read-only via log file),
  Workshop Map Loader.
- API: `https://api.rlpeak.com` serves the curated catalog and asset files.
- Asset cache: `AppData/cache/ItemsFiles`. Assets are downloaded only, never
  executed.

**Important limit — catalog-only:**
RLPeak does NOT accept user-supplied PNGs. The wiki and SDK roadmap make
**zero mention** of custom user textures, PNG upload, decal upload, or
literal-RGB pipelines. Future roadmap is: more session stats, rank history,
training utilities, an SDK to plug in *other* tools.

> "RLPeak's current scope focuses on cosmetics: 'V1 covers items (decals,
> wheels, boosts).' Custom maps and plugins remain on the roadmap but are
> not yet implemented."
> — paraphrased from rlpeak.com/blog/is-bakkesmod-still-working.html

### 2.2 Custom maps via mods/ subfolder

- This works file-only and is EAC-safe **when launched offline**.
- Stock pattern: drop `<map>.upk` (and optional `<map>.jpg` thumbnail) into
  `CookedPCConsole/mods/<map>/`. Or atomic-swap with `Labs_Underpass_P.upk` /
  `Park_P.upk`.
- Loaders: **rl-map-loader** (github.com/nikosleft/rl-map-loader),
  **RocketLeague-MapManager** (github.com/Yggdrasil128/...), Lethamyr's own
  custom map loader, and the bundled **Workshop Map Loader** in RLPeak V1.1.0.
- Quote from rocketleaguemapmaking.com: "A custom map loader tool only replaces
  a single map file and does not inject anything into the game process, making
  it very unlikely to trigger EAC. However, to be completely safe, always use
  the offline launch option and never load a custom map in an online playlist."

### 2.3 Our own hijack + sparse-TFC approach (project status)

Already proven in this repo (see `docs_rl/custom_decals.md`, commits 4567ee2,
843f447, 47ae078, 59fb130). Working pipeline:
1. PNG → BC3 DDS.
2. Build a sparse `.tfc` with only the new mip data.
3. Patch the target `.upk` FName table / texture offsets so the decal references
   our sparse TFC.
4. Install both into `CookedPCConsole/` (UPK swap, `.tfc` side-by-side).
5. Game install stays pristine; user toggles by removing files.

**Verified byte-identical on the "solid red" test** (e60560d). Game accepts it
with EAC off; online behaviour with EAC on still **needs verification** — the
DLL injection is gone, but EAC may hash the UPK/TFC files. See section 6.

---

## 3. Mod sites & inventory

### 3.1 videogamemods.com / rocketleaguemods.com

- ~2,208 custom decals indexed. Every single one I sampled requires
  BakkesMod + AlphaConsole. Distribution format is the AlphaConsole zip (PNG
  textures + JSON metadata for `data/acplugin/DecalTextures/`).
- Newest 2026 release indexed: "Kinetic Esports 2026 Universal Decal"
  (mod id 1023904). Install method = AlphaConsole. **No EAC-compatible
  release found.**
- RLCS 2025 Universal Esport Decals (Series 1) — same stack.
- "Esport Decals" mod: explicitly uses the in-game tint system on the
  `octane:griffon` slot, so it's NOT literal RGB; it's just a swap of the
  base mask with options.

Source: https://videogamemods.com/rocketleague/

### 3.2 BakkesPlugins.com

- Hosts plugins, cars, and maps. "The community home for Rocket League mods."
- All plugin downloads require BakkesMod. The "Cars" section (custom car
  bodies) is exclusively delivered as BakkesMod plugin packages.
- Active maintenance: BakkesMod 2 default plugin repo updated 2026-04-24.

### 3.3 Creator marketplaces

- **MELOGRAPHICS** (news.melo.graphics) — paid custom decals, BakkesMod +
  AlphaConsole only.
- **Kaizen RL** (kaizenrl.com) — same.
- **Snackosaurus** (snackosaurus.gumroad.com) — same; install = drop folder
  in `data/acplugin/DecalTextures/`.
- **72PinConnector** (72pinconnector.com/rocket-league-decals/) — same.
- **TunersRL** (tunersrl.gumroad.com) — same.
- **ArtStation — Lindsey Gunsallus** ("Linz" Guns) — pro Psyonix decal artist
  (built RLCS esport sets). Her work uses Psyonix's two-channel mask system,
  not literal RGB.

### 3.4 WrapsRL — AI decal generator

- https://wrapsrl.com — Stable-Diffusion-XL-driven Octane/Fennec/Dominus
  decal generator.
- Output: PNG diffuse + normal + spec maps + JSON, packaged for
  BakkesMod/AlphaConsole.
- Source: github.com/Lambourne2/wrapsrl — "Planning & Initial Development
  Phase", only 4 commits. **Not file-only**, no post-EAC plan documented.

### 3.5 CustomDecalLogos (github.com/ubelhj/CustomDecalLogos)

- Node.js + Jimp tool. Composites a user PNG over an existing AlphaConsole
  decal base or blank canvas.
- Outputs into `\data\acplugin\DecalTextures` — pure BakkesMod path. Dead
  online post-EAC.

### 3.6 CustomCar (github.com/smallest-cock/CustomCar)

- BakkesMod plugin. Loads arbitrary FBX as car body / topper.
- Last release v1.0.4 on **2026-05-12** (very recent).
- Author explicitly notes: "in online matches, all spawned cars will have
  Octane hitbox" due to "the game's online protections." Authors assume
  EAC-off launch — confirms there is **no file-only custom car body path**
  shipping today.

---

## 4. Communities

### 4.1 Discord servers

- **discord.gg/HBq3T7S** — "Rocket League Mods" (~2,061 members). Primary
  modding hub linked from videogamemods.com. (Not directly fetchable
  without auth.)
- **discord.gg/CYb3Wdd** — "Rocket League Skins Wiki" (decal/skin focus).
- **discord.gg/CgFSe9p** — alt "Rocket League Mods" server (~2,074 members).
- **discord.gg/rocketleague** — official Psyonix server (~700k). Modding is
  not the focus but post-EAC sentiment lives here.
- **discord.gg/rlgarage** — RL Garage server (~182k).

We have **not** joined any of these as the agent. Recommend the user join
HBq3T7S and CYb3Wdd specifically and search for "EAC", "custom decal",
"file-only", "TFC", "UPK" in recent (post-2026-04-28) messages.

### 4.2 Reddit

- /r/RocketLeagueMods — small, modding-focused. Most posts pre-EAC.
- /r/RocketLeague — bigger, general; EAC reaction threads have hundreds of
  comments on 4/28+, mostly mourning BakkesMod.
- No EAC-era literal-RGB success post surfaced in our searches.

### 4.3 Twitter / X

- @RocketYota — pre-EAC speculated BakkesMod might survive. Wrong, but their
  threads collect mod-community sentiment.
- Several pro-scene artists (Linz Guns, MELO, Kaizen) post WIPs but all
  inside the AlphaConsole pipeline.

### 4.4 RL Modding Wiki & RLMM

- https://rocketleaguemoddingwiki.github.io/ — bible for asset extraction,
  Blender import, UDK map-building, UPK decryption.
- https://rocketleaguemapmaking.com/ — focused on custom maps. Confirms:
  custom maps can ship custom materials but the engine "only allows materials
  which exist within packages already inside CookedPCConsole" — i.e. you
  must include the source package or use Material Instances.

---

## 5. Tools & libraries inventory

### 5.1 Rocket-League-specific

| Tool | What | Repo | Status |
|---|---|---|---|
| **RLUPKTool** | AES-256-ECB decrypt of RL UPKs. **Decrypt only — no encrypt.** Header AES + body zlib chunks. | github.com/AltimorTASDK/RLUPKTool | last commit 2018-06; 4 total commits |
| **UModel (AltimorTASDK fork)** | View/extract assets from decrypted RL UPKs. Auto-decrypts now. | github.com/AltimorTASDK/UModel | maintained |
| **rlpeak/rlpeak** | The Tauri/React/Rust launcher itself. | github.com/rlpeak/rlpeak | active |
| **CustomCar** | BakkesMod plugin, custom car bodies (offline). | github.com/smallest-cock/CustomCar | active, v1.0.4 May 2026 |
| **rl-map-loader** | File-swap map loader. | github.com/nikosleft/rl-map-loader | older |
| **RocketLeague-MapManager** | Browser-based map manager. | github.com/Yggdrasil128/RocketLeague-MapManager | maintained |
| **RocketLib** | C++ framework for RL mods. | github.com/h311d1n3r/RocketLib | injection-based; EOL post-EAC |

### 5.2 Generic UE3 tools (potentially applicable)

| Tool | What | Source | Applicability to RL |
|---|---|---|---|
| **UPKUtils** (wghost) | Hex-patch + UnrealScript decompile + Texture DDS export/import. Includes `PatcherGUI` to install/remove mods cleanly. Built for XCOM:EU/EW Long War; now also Batman AA/AC. | github.com/wghost/UPKUtils | **Locked to specific engine versions, will crash on unsupported games.** Could be ported — the patching technique (hex diffs + clean install/uninstall) is exactly the abstraction we want. |
| **UPK Explorer for UE2-UE3** | Universal: extract textures (DDS), create texture packs, swap meshes/materials, FBX I/O. | nexusmods.com/site/mods/587 | Worth testing against decrypted RL UPK. Pipeline: edit DDS, build pack, install with **TFC Installer**. |
| **TFC Installer** | Installs UPK Explorer texture packs by patching the package + injecting into TFC. **Auto-disables SHA checks in some game .exe** (red flag for EAC). | nexusmods.com/site/mods/588 | The technique is what we already do manually; we should study its delta logic. |
| **UPKManager (stricq)** | C# UE3 package extractor + rebuilder. DDS I/O. Built for Blade & Soul. | github.com/stricq/UPKManager | Closest to a clean reference implementation of a round-trip texture editor. **High value reference.** |
| **UE Explorer** | UE1/2/3 browser + UnrealScript decompiler (EliotVU). | eliotvu.com | Read-only but very mature on UE3 internals. |
| **UPKmodder** | Java tool, hex-diff UPK mods, used by XCOM Long War. | github.com/AmineriRevisited/UPKmodder | Mostly UnrealScript-focused, less texture. |
| **Buckminsterfullerene02/UE-Modding-Tools** | Databank of UE tools across games. | github.com/Buckminsterfullerene02/UE-Modding-Tools | Index page; cross-references most of the above. |

### 5.3 What the UE3-community techniques tell us

The XCOM Long War / Blade & Soul / Borderlands 2 communities have **all**
solved file-only UE3 texture replacement. The pattern is universal:

1. Decrypt the package (game-specific key).
2. Decompress zlib chunks.
3. Edit texture mip data (DDS or raw BC3/BC5).
4. Re-pack the package with original compression preserved.
5. Optionally inject into TFC instead of growing the UPK.
6. Either re-encrypt OR rely on the engine accepting unencrypted packages
   (most UE3 games accept both; RL specifically may not — needs testing).

**Crucial insight**: TFC Installer "automatically disables SHA checks in some
game exe files when a texture package is installed." This is a clue that some
UE3 games do hash-check packages, and the community has been patching the exe
to bypass it. For RL with EAC, **patching the exe is a non-starter** — EAC
will trip. We need the engine to load modified packages without exe patching.
Our hijack approach (rename + swap) sidesteps this by keeping the *original*
UPK intact and routing through a different package name.

---

## 6. Direct answers to the seven specific questions

### Q1. Anyone solved literal-RGB custom decals file-only post-EAC?

**No public solution found.** Every indexed 2024–2026 decal release uses
BakkesMod + AlphaConsole. RLPeak — the only file-only player — explicitly
ships catalog-only swaps and does not (yet) accept user PNGs. **The dual-TFC
hijack technique in this repo (commits 4567ee2 → 59fb130) appears to be ahead
of the public ecosystem.** This is a real differentiator for ALXS-RL-Mod.

### Q2. EsportsFullColor / RGB-pack equivalents without runtime injection?

The "EsportsFullColor" name does not appear anywhere in public sources.
Closest match: the **Esport Decals pack** on videogamemods.com — but that
mod ships color-adjustable decals using **in-game tint parameters** (primary
+ accent), not literal RGB diffuse. So even the name "FullColor" in the
community refers to the *tint* system, not arbitrary RGB. The literal-RGB
problem is therefore still unsolved publicly.

### Q3. Custom car bodies file-only post-EAC?

**No.** CustomCar (github.com/smallest-cock/CustomCar) is the gold standard
for custom car bodies and it is a BakkesMod plugin. Last release 2026-05-12,
which means the author knows about EAC and shipped anyway — they've
accepted offline-only. **No file-only custom car body path exists publicly.**
RLPeak's roadmap mentions cars only as future work with no technical detail.

### Q4. Workshop maps and EAC

**Yes, they work online with EAC enabled** — *when* delivered through Steam
Workshop's blessed channel (Psyonix added Workshop support to the Epic build
and made it EAC-compatible). Custom map loaders (file-swap to
`Labs_Underpass_P.upk`) **only work offline** and the community consensus
(rocketleaguemapmaking.com) is: "always use the offline launch option and
never load a custom map in an online playlist." So workshop-as-a-vector for
custom materials is technically real, but Psyonix curates Workshop content.

### Q5. UE3 modding scenes we've missed

| Game | Engine | Relevant | Notes |
|---|---|---|---|
| GTA 5 | RAGE (not UE) | No | Irrelevant. |
| Fortnite | UE4/5 | Partial | UE4+ uses .pak / AES; different format. Not transferable. |
| Saints Row IV | UE? Volition CEngine | No | Not UE3. |
| XCOM:EU/EW | UE3 | **Yes** — see UPKUtils, UPKmodder. **Highest transfer value.** |
| XCOM 2 | UE3.5 | Partial | "Highlander" pattern (replace XComGame.upk wholesale) won't fly with EAC. |
| Borderlands 1/2/PreSequel | UE3 | **Yes** — texmod + UPK Explorer well documented. |
| Batman AA / AC | UE3 | **Yes** — UPKUtils supports both. |
| Blade & Soul (UE3 era) | UE3 | **Yes** — UPKManager is a clean C# round-trip implementation. |
| Bioshock Infinite | UE3 | Yes (texmod-era). |
| Dishonored | UE3 | Yes; UPK Explorer support. |

Top three to study deeper: **UPKManager (Blade & Soul)** for clean C# code,
**UPKUtils + PatcherGUI** for the install/uninstall abstraction, **Borderlands
2 texmod community** for documentation of the texture-edit user flow.

### Q6. Adapting UPKmodder for RL?

UPKmodder is Java, focused on **UnrealScript hex-patching** (it converts
hex into pseudo-UnrealScript). Texture work is not its strength. The more
applicable tools are **UPKUtils** (same author family) which has
`ExportTexturesToDDS` / `ImportTexturesFromDDS`, and **UPKManager** which
is C# and explicitly designed for round-trip texture editing.

No record of anyone trying UPKmodder against Rocket League. Engine version
mismatch (RL uses an unusually old custom UE3 build, engine 868, licensee
20/22) means most tools crash on RL UPKs without decryption first. Once
decrypted (via RLUPKTool or UModel-fork), the layout is standard enough
that **UPKManager would likely be the best starting reference** for a
clean round-trip editor port.

### Q7. Post-EAC custom decal releases to download and RE

None found that are file-only. The single closest target is **RLPeak's own
catalog files** — download a curated RLPeak decal, diff against the original
asset RLPeak backed up, and see exactly what bytes get swapped. RLPeak being
open source means we can also just read `src-tauri/` in their repo.

Suggested RE list (in priority order):
1. **RLPeak `src-tauri/`** — clone, read the Rust file-swap logic, see if
   it patches FName tables (like our `upk_renamer`) or does pure byte swap.
2. **CustomCar's data files** (the FBX bridge into BM) — to understand the
   in-memory car-body schema we'd need for a file-only port.
3. **TFC Installer source** — read the SHA-check disabler logic to learn
   *which* RL exe locations check package hashes (if any).
4. **AlphaConsole's old plugin DLL** — symbol-strip & RE the D3D texture
   hook. Even if dead online, the offset of the decal texture binding in
   the shader is gold for understanding what slot we hijack.

---

## 7. Most promising unexplored paths (the "what to try next" list)

### Path A — Port UPKManager's round-trip code to Rust, target RL

Highest leverage. UPKManager (https://github.com/stricq/UPKManager) is a
public, MIT-friendly, **proven** round-trip texture editor for UE3 cooked
packages (Blade & Soul). It exports a texture object to DDS, lets you edit
it, rebuilds the UPK preserving compression. The format differences with
RL are bounded:
- Decryption layer (AES-256-ECB, we already have the key in RLUPKTool).
- Engine version 868 licensee 20/22 quirks (probably ~50 lines of header
  parsing).
- TFC offset table fix-up (we already do this manually).

Estimated effort: 2–4 weeks. Pay-off: native PNG-to-installed-decal pipeline,
no Python, no external tooling. Eliminates the "BC3 conversion through Python
prototype" historical debt.

### Path B — Workshop-as-CDN for cosmetics

Currently underused. Steam Workshop maps are EAC-compatible online. A custom
map .upk **can ship arbitrary textures, materials, and meshes**. We could
publish a Workshop "map" that's really a cosmetics container — but it would
only apply in that map (Psyonix's design). Not viable for "see my decal in
ranked", but viable for "see my decal in training". For a user who wants
private custom training with skins, this is the only EAC-online-legal path
known today.

### Q3-related Path C — Mesh swap via existing slot hijack

We've done texture hijack via Stars-slot D + RGB swap. The same trick on a
**static mesh** in a low-traffic vehicle slot would prove the file-only
custom car body path. No public solution exists, so this is a real frontier.
Risk: mesh slots are referenced from many .upk packages; finding a
"sacrificeable" slot is harder than texture slots.

### Honorable mentions

- **RE the AlphaConsole DLL** for hook offsets even though it's dead online.
  Knowing *where* AlphaConsole hooks the decal binding tells us which
  package/material to target in the file-only path.
- **Join HBq3T7S and ping the Linz / MELO / Kaizen artists**. They have
  pro-grade decal-channel knowledge (alpha mask conventions, normal/spec
  channel packing). Even if they don't know modding internals, their PSD
  templates are the ground truth for what an in-game decal expects.
- **Ask the wghost / UPKUtils maintainer** about porting to RL. wghost has
  publicly said they're open to additional games. RL UPK structure is close
  enough that contributing RL support to UPKUtils could give us a free
  community-maintained patcher GUI.

---

## 8. People worth reaching out to

| Name / handle | Why | Where |
|---|---|---|
| **AltimorTASDK** | Wrote RLUPKTool and the RL-specific UModel fork. Knows the encryption + format intimately. | github.com/AltimorTASDK |
| **wghost** | UPKUtils author. Could potentially add RL support. | github.com/wghost |
| **stricq / ashllay** | UPKManager — closest existing round-trip implementation. | github.com/stricq/UPKManager |
| **smallest-cock** | CustomCar author, shipped 2026-05-12 (post-EAC aware). | github.com/smallest-cock |
| **Lambourne2** | WrapsRL author, has the AI-decal generation pipeline that we could pair with a file-only installer. | github.com/Lambourne2/wrapsrl |
| **RLPeak team** | Same stack as us. Friendly competition / possible collab. Open-source so we can contribute upstream. | github.com/rlpeak/rlpeak |
| **Lindsey "Linz" Gunsallus** | Psyonix decal artist, knows internal mask conventions. | linzguns.artstation.com |
| **MELOGRAPHICS** | Largest decal creator, has internal knowledge of the AlphaConsole format. | news.melo.graphics |

---

## 9. Bottom line for ALXS-RL-Mod

1. The public ecosystem has **not solved literal-RGB file-only decals
   post-EAC**. Our dual-TFC hijack approach (this repo, commits 4567ee2 →
   59fb130) is genuinely novel relative to public state-of-the-art.
2. RLPeak is the only direct competitor in the file-only space, and they
   ship catalog-only. Custom-PNG support is **wide open**.
3. UPKManager (Blade & Soul, MIT-ish, C#) is the best reference for a clean
   round-trip texture editor port — strongly recommend studying it.
4. Workshop maps are the only EAC-online-legal path for *any* custom assets
   today, and Psyonix curates the channel.
5. Custom car bodies remain unsolved file-only — meaningful greenfield if
   we want it after decals are stable.
