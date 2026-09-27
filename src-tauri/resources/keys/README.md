# AES keys for cooked Rocket League packages

Rocket League encrypts the name / import / export tables of most cooked
`.upk` packages with AES-256-ECB. The app needs the public community key
list (`keys.txt`, one base64 key per line) to rename packages (item swaps)
and to read textures (thumbnails, custom decals).

`keys.txt` is **not committed** (see `.gitignore`). The app looks for it, in
order:

1. `%LOCALAPPDATA%\ALXS-RL-Mod\keys.txt` (user drop-in, wins),
2. `src-tauri/resources/keys/keys.txt` (dev builds, read from disk).

Without keys the app still works for maps, palette, stats and presets; item
swaps and decals report `KeysMissing` in the UI.
