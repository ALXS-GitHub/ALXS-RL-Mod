# Lethamyr custom maps + UE3 map asset workflow

Date: 2026-05-15.
Question: how do custom maps ship literal RGB textures/materials that don't get the team-color paint shader, and can that be reused for vehicle decals?

---

## TL;DR

* **Maps cannot ship truly-custom materials.** Every "custom material" in a workshop map is actually a **Material Instance Constant (MIC)** parented to a pre-existing engine material that's already cooked into `CookedPCConsole`. The most common base is `TexturePaint_2Tex_Color` from `Startup.upk` (`Engine > Content > EngineDebugMaterials`). What the map ships is the MIC + the new `Texture2D` data; the **base shader is the one already inside RL**.
* **Maps can ship arbitrary `Texture2D` data** — those *are* cooked into the level `.upk` and load at runtime.
* **The "no team paint" effect on map signs/billboards** is not a separate shader — it's just a different base material (e.g. `LevelColorationUnlitMaterial`, `TexturePaint_2Tex_Color`) that has no team-paint nodes. The base material is selected by the modder when they create their MIC.
* **Vehicle decals can't reuse this path** because: vehicle parts are loaded by a different content-loading code path (the *Products* / item-equip pipeline), and the body's decal material slot is fixed to a paint-shader-aware material (`TextureSkin`-style) that is bound by the body's `ProductID`. The map-asset trick doesn't give you a route into that slot.
* **`CookedPCConsole/mods/`** loads `.upk` files for **both** maps (via the Underpass-swap method or RLMapLoader-style swaps) *and* vehicle parts (via the CustomCar BakkesMod plugin) — but the **plugin is what changes the loading semantics**, not the directory.

---

## 1. Background: UE3/UDK material model in RL

UE3 cooks materials by compiling the HLSL shader code derived from the material's expression graph into a binary blob inside the `.upk`. **RL refuses to compile new material shaders at runtime**:

> "Any custom materials cannot compile, as they have not been cooked into the game files and are only referenced by our uncooked maps."
> — rocketleaguemoddingwiki.github.io, `udk_tour/materials.html`

> "Can't compile testy_D_Mat with seekfree loading path on console, will attempt to use default material instead."
> — observed log error, Epic Dev forum [link below]

The only materials that work are those already in `CookedPCConsole/*.upk` (chiefly `Startup.upk`, `TAGame.upk`, `Engine.upk`). UDK ships a copy of those engine materials, so you create a **Material Instance Constant** *parented* to one of them, change its texture/scalar/vector parameters, and the cooked map references the base by name. At runtime, RL resolves the parent by name from its own copy of the material, then applies your overrides.

This is identical to the AlphaConsole approach for decals: AC also doesn't compile a new shader; it just swaps texture bytes inside an existing material instance.

---

## 2. How custom map signs/billboards render literal RGB

When a Lethamyr map shows a literal logo on a wall, here's what actually happens:

1. The modder picks `TexturePaint_2Tex_Color` (or `LevelColorationLitMaterial` / `Unlit`) as the parent. This material's expression graph does **not** include the team-paint logic. It just samples a `Diffuse` texture parameter.
2. They create a MIC inside the level `.upk`, set `TextureParameterValues.Diffuse = <imported PNG/TGA>`. The texture is imported into the level `.upk` and cooked normally as a UE3 `Texture2D`.
3. They apply the MIC to a static mesh face.
4. When RL loads the level, it resolves `TexturePaint_2Tex_Color` from the already-cooked `Startup.upk`, instantiates it with the cooked overrides, and renders the sign as plain RGB.

> "Locate `LevelColorationLitMaterial` in Engine > Content > EngineDebugMaterials… Create New Material Instance (Constant). Modify the Color parameter."
> — rocketleaguemapmaking.com, `guide/udk/materials`

> "Workshop textures must be installed for certain materials to function in-game."
> — rocketleaguemapmaking.com, `guide/udk/custom_material`

`TexturePaint_2Tex_Color` is interesting because its name suggests a paint shader, but it's used by mapmakers as a generic two-texture diffuse blend — the **"Paint" in the name refers to UDK's runtime texture painting, not RL's team-color shader**. Important nuance: just because the base material has `Paint` in its name doesn't mean it runs the paint shader. RL has many materials in `Startup.upk`, only some of which read the team-color slot.

---

## 3. Cookbook: shipping a custom map upk to RL

Sourced from the modding wiki tips page and RLMM project_setup guide.

1. Install UDK 2015 + dummy classes (sticky walls, BoostPad_TA, PlayerStart_TA, etc.). Dummy classes are blank instances with the **correct Outer.Path name** of an RL asset — they survive cooking and at load time RL resolves the *real* asset of that name from `CookedPCConsole`.
2. Build your map. Save it as `<YourMapName>_P.upk` *inside the UDK content folder*. Import every custom texture into the **level package itself** (not a separate package) so cooking pulls the bytes in:
   > "you can (and should) import meshes, textures or create assets directly inside your level package. … This is the only thing that works with workshop and doesn't require dealing with extra files."
   > — modding wiki, `udk_tour/tips.html`
3. Reference each imported asset somewhere (drop into viewport or via a Kismet ObjectVar) — UDK strips unreferenced assets on Build All.
4. Cook the map with UDK's command-line cooker (or via Bmorr1123's setup gist).
5. Place the cooked `.upk` into `<RL>/TAGame/CookedPCConsole/mods/<MapName>/<MapName>.upk`. A map-loader tool (RLMapLoader, MapLoader, Lethamyr's loader) swaps it into the `Labs_Underpass_P.upk` slot at launch time.

For testing: the recommended slot is `Labs_Underpass_P.upk` to avoid competitive-ban triggers. Renaming to `Park_P.upk` works but is risky.

---

## 4. Why this DOES NOT cleanly apply to vehicle decals

| Aspect | Map asset path | Vehicle decal path |
| -------- | -------------- | ------------------- |
| **Container .upk** | Map `.upk` placed in `CookedPCConsole/mods/<name>/` | Body/decal `.upk` shipped inside `CookedPCConsole/Body_*.upk` (encrypted), referenced by ProductID |
| **Load trigger** | Engine map-load (uses dummy-class name resolution) | Garage equip — `OnlineCosmeticItemAttribute`, `Products.upk`, item-equip pipeline |
| **Material binding** | StaticMesh references a MIC inside the level upk | Body material slot is hard-wired in the body's cooked material, parameter names fixed by the body |
| **What the modder ships** | New MIC + new Texture2D bytes, by name reference | Either: (a) replace existing body upk, or (b) hijack texture bytes inside that upk |
| **Shader chosen by modder?** | Yes — picks any base material from `Startup.upk` | **No** — material is whatever the body was cooked with; nearly always paint-shader-aware on body, `TextureSkin` on decal |
| **Workshop visible to other players?** | Yes (workshop maps are official content) | No — purely client-side |

The constraint isn't at the *asset class* level (a `Texture2D` is a `Texture2D` whether it's on a map or a car). It's at the **runtime loading path**:

* For a **map**, the loader instantiates whatever MaterialInstance references your texture by name. You author the MaterialInstance.
* For a **vehicle decal**, the equip pipeline looks up `(EquipSlot, ProductID)` → fetches the body's pre-cooked material → injects the `Skin`/`Diffuse` textures **the body's material declares**. The modder does *not* get to swap the material — only the texture-parameter bytes.

So the question "can a vehicle use a non-paint shader?" reduces to: **is there a body whose decal material was cooked without a team-paint node?** Empirically — yes, when the AC pack ships a "Main Body Only" decal product, that product's underlying material on the body is *already* paint-shader-aware, but with all paint inputs zeroed out (our forensic finding in `alphaconsole_packs_forensics.md`). The trick is to **neutralize** the paint shader by feeding it a zero mask, not to swap shaders.

> Note: the same `slot=1 item=306` is used for Sandman, FullColor companions and almost every "Skin" pack on Octane — that one item's material is exactly the paint-shader-aware decal material we've been hijacking. By feeding it a black-opaque BC3 mask, the paint shader contributes nothing, and any literal RGB we deliver via a different texture parameter on the **upper-body color-mask** material shows through.

---

## 5. Question-by-question answers

### Q1. How do custom maps ship custom materials that don't use the team-paint shader?

They don't ship custom *shaders*. They ship a **Material Instance Constant** parented to an engine material that has **no team-paint expressions** in its graph — typically `LevelColorationLitMaterial`, `LevelColorationUnlitMaterial`, or `TexturePaint_2Tex_Color`. The MIC + a new `Texture2D` ride inside the level `.upk`; the base shader is resolved by name from `Startup.upk` at runtime.

### Q2. Can the same approach be used for vehicle decals? Why or why not?

**No, not as-is.** The decal/skin texture parameter slot on a body is bound to whatever material that body was cooked with — typically a paint-shader-aware `TextureSkin` material. There's no API to redirect a body's slot to a different base material at runtime. What you *can* do (and what AC does) is **feed neutralized texture data** to the paint-shader-aware material so it produces no paint output, then deliver the literal RGB via a *different* texture parameter on the same or a sibling material (the body's "upper body color-mask" texture).

Constraint location: it's enforced at the **runtime content-loading path**, not the asset class. `UTexture2D` and `UMaterialInstanceConstant` are unrestricted; the equip pipeline simply doesn't let modders pick the parent material.

### Q3. Cookbook for shipping a custom UE3 material to RL?

See section 3. Short form:
1. UDK 2015 → import textures into level upk.
2. Create MIC parented to an engine material that already exists in `CookedPCConsole`.
3. Set texture-parameter values.
4. Cook level → produce `<MapName>_P.upk`.
5. Drop into `TAGame/CookedPCConsole/mods/<MapName>/`, load via a swap-loader (Lethamyr/RLMapLoader/MapLoader).

### Q4. Does RL load assets from `CookedPCConsole/mods/` for both maps AND vehicle parts?

**Yes for both, but via different code paths and via a plugin for cars.**

* Maps: the **game itself** scans `CookedPCConsole/` for `.upk` files containing levels. Subfolders are walked. A swap-loader renames or aliases a custom map into the `Labs_Underpass_P` slot at runtime.
  > "the .upk file should be placed in the CookedPCConsole folder of your Rocket League installation… as long as the .upk files are somewhere inside the CookedPCConsole folder, they will be recognized"
  > — search consensus, multiple RLMapLoader docs

* Vehicle parts (cars/toppers via `CustomCar` BakkesMod plugin):
  > "Open the `CookedPCConsole` folder of your RL installation, and put the `.upk` file there"
  > `C:\Program Files\Epic Games\rocketleague\TAGame\CookedPCConsole\mods\CustomCars\MyCustomCar.upk`
  > "You can create subfolders to organize your `.upk` files if you want. As long as the `.upk` files are somewhere inside the `CookedPCConsole` folder"
  > — `smallest-cock/CustomCar` README
  But the plugin is what actually patches the equip pipeline to load that `.upk` when the user spawns; without BakkesMod + CustomCar, the file just sits there inert.

* For decals/skins specifically (textures, not meshes), **there is no equivalent first-class plugin path** — that's why AC swaps texture bytes inside an existing upk rather than dropping a new upk. Our hijack approach is in the same category.

### Q5. Custom Workshop maps using vehicle replacement?

No evidence found. The workshop map system is built around levels (game type, level streaming, kismet); the equip pipeline is independent of the loaded level. Bundling a `.upk` with vehicle assets into a map's mods folder won't make the equip pipeline find it — the equip system reads from its own asset registry initialized at game start. CustomCar/BakkesMod actively patch that registry. No one has shipped a map that overrides a vehicle skin via the level itself.

---

## 6. Sources

* RL Modding Wiki — How to use any in-game asset: <https://rocketleaguemoddingwiki.github.io/pages/tutorials/How_to_use_any_in-game_assets.html>
* RL Modding Wiki — Map files & packages tips: <https://rocketleaguemoddingwiki.github.io/pages/udk_tour/tips.html>
* RL Modding Wiki — Materials: <https://rocketleaguemoddingwiki.github.io/pages/udk_tour/materials.html>
* RLMM — Custom Materials: <https://rocketleaguemapmaking.com/guide/udk/custom_material>
* RLMM — Project setup: <https://rocketleaguemapmaking.com/essential/project_setup>
* RLMM — Dummy Assets: <https://rocketleaguemapmaking.com/guide/udk/dummy_assets>
* RLMM — Materials guide: rocketleaguemapmaking.com/guide/udk/materials (page accessible via that domain — extracted content captured)
* Epic Dev forum, "Creating custom Materials for Rocket League" (2016-17, unsolved): <https://forums.unrealengine.com/t/creating-custom-materials-for-rocket-league/68039>
* CustomCar (smallest-cock) — `.upk` + `.json` BakkesMod plugin: <https://github.com/smallest-cock/CustomCar> — README mirrored to `customcar_readme.md`
* RLMapLoader (drewbitt) — README: <https://github.com/drewbitt/RLMapLoader/blob/master/README.md>
* rl-map-loader (nikosleft) — README: <https://github.com/nikosleft/rl-map-loader/blob/main/README.md>
* Lethamyr maps + install FAQ: <https://lethamyr.com/maps>, <https://lethamyr.com/faq>
* MELOGRAPHICS — BakkesMod + AlphaConsole custom decals guide: <https://medium.com/madebymelo/custom-rocket-league-decals-graphics-the-ultimate-guide-5d943eb62e79>
