# Modded multiplayer (custom maps with friends)

The **Modded multi** page (`src/features/multi`, backend `src-tauri/src/multi`)
hosts or joins a custom-map game between friends. The game runs **without
Easy Anti-Cheat** (offline mode), so this never touches online play.

## How it works

| Piece | Role | Who installs it |
|---|---|---|
| Tailscale | Private network between the players (`100.x.y.z`), no port forwarding | each player |
| BakkesMod + Rocket Plugin | Rocket Plugin hosts (`open <arena>?…?Lan?Listen`, port 7777) and joins (`start ip:port/?Lan`) | each player; the checklist's Install button runs BakkesMod's `plugininstaller.exe bakkesmod://install/26` (bakkesplugins id 26) |
| ALXS-RL-Mod | installs the map, starts the game (no EAC) and BakkesMod's injector, sends the Rocket Plugin command | each player |

Every player needs the **same map file** on the **same Labs arena**: the host
picks the arena on the host card, opens its level name
(`rp host Labs_Underpass_P`), and our map is written over that arena
(`maps::session`). The invite code carries the arena (friends' apps use it
automatically), the map's SHA-256 and its bakkesplugins id. Nothing is
downloaded without a click: the join card (`multi_check`) shows whether the
map is already on that arena, in the library, downloadable from
bakkesplugins (Download button) or to be imported by hand, and Join stays
disabled until the map is in the library. A map already on the right arena
is never rewritten.

### Steps (`multi::session`)

1. Map: resolved in the library (bakkesplugins id, then name), then
   `maps::session::activate` over the chosen arena (same write layer as
   Maps), skipped when that map is already there. It is a normal,
   persistent install (never the "play offline" mode): the map stays in
   place and in the active mods panel after the game closes, until the user
   removes it.
2. BakkesMod RCON (`multi::bakkes_cfg`), only while the game is closed:
   - `cfg/autoexec.cfg`: a marked block `rcon_enabled "1"` + a random
     `rcon_password`;
   - `data/rcon_commands.cfg` (whitelist, first word): `rp`.
   "Remove" in the checklist deletes both (the `rp` line only if we added it).
3. The game without EAC: on Epic the app brings the Epic launcher forward
   and the session waits (`launchInEpic`, 10 min max) for the user to pick
   "Launch without Anti-Cheat" (see ARCHITECTURE.md, "Launch without EAC on
   Epic"); a game started with EAC ends the session with an explanation.
   Then `BakkesMod.exe` if it is not running. The app never injects
   anything itself.
4. Match options (`multi::rp_options`): game mode (`rp gamemode`), players
   (`rp players`), a Rocket Plugin preset (its `rp mutator` lines are read
   from `data/RocketPlugin/presets/*.cfg` and replayed, so the user's
   `rp_preset_path` cvar does not matter) and individual mutators on top.
   Every mutator category not chosen is sent as `Default`. Bot difficulty,
   host password and UPnP stay in Rocket Plugin's window. "Start the match
   myself in Rocket Plugin" sends nothing: the session ends in
   `hostInRocketPlugin` once the game and BakkesMod are up, and the invite
   code still carries the map and arena.
5. Waits (4 min max) for `ws://127.0.0.1:<rcon_port>` (default 9002), sends
   `rcon_password <pw>` then `rp host <arena>` / `rp join <ip> <port>`.
6. If RCON never answers (game started before the setup, BakkesMod not
   injected, protocol change…), the session ends in `manual` and the UI
   shows the command to type in the BakkesMod console (F6).

Invite code: `ALXS-MULTI-` + base64url(JSON `{v, host, port, hostName, map}`),
strictly validated (`multi::invite::decode`): the host is an IPv4, the arena
a known Labs file, the map id alphanumeric — nothing free-form reaches the
console command.

## Repaired maps

Map repair (`docs_rl/map_repair.md`) is deterministic: same original file +
same mapping version = same repaired file. The invite carries the host's
`repairVersion` (and, as before, the SHA-256 of the installed file). The
join card compares it with the friend's copy (`multi_check` →
`repairAction`: none / repair vN / undo / unavailable when the friend's app
lacks that mapping) and offers the action on click; Join stays disabled
(and `multi_join` refuses) until both sides match.

## Page layout

Three tabs: **Host** (saved setups bar, map picked from a thumbnail grid,
arena, match options with a categorised mutators dialog, Host button),
**Join** (invite code, map state with the explicit download, Join) and
**Infos** (the "Before playing" checklist, installation guide, Tailscale
policy). Saved setups ("host presets", `multi::presets`) store the map, the
arena replaced and the match options; the form itself is remembered in
`localStorage` between sessions.

## Unverified (test in game)

- BakkesMod's RCON handshake (`authyes` / `authno`) and whether
  `rcon_enabled` set from `autoexec.cfg` starts the server — sources only
  showed the cvars (`rcon_port`, `rcon_password`, the whitelist file).
- `rp host` uses Rocket Plugin's currently selected game mode (first entry
  by default) and 6 players; change them in Rocket Plugin's window.
- Rocket Plugin has no host password from the command line: the tailnet
  policy below is the access control.

## Securing Tailscale

The default tailnet policy lets every device reach every port of every other
device. Replace it (admin console → Access controls) with:

```jsonc
{
  "groups": { "group:rl": ["you@example.com", "friend@example.com"] },
  "acls": [
    { "action": "accept", "src": ["group:rl"], "proto": "udp", "dst": ["group:rl:7777"] }
  ],
  "ssh": []
}
```

Only UDP 7777 between players is allowed (replies are allowed by Tailscale's
stateful filter); SSH, SMB, RDP, BakkesMod's RCON port and everything else
are denied. Also: no `tailscale up --ssh`, no exit node / subnet routes, keep
device approval on, allow Rocket League on *Private* networks only in the
Windows firewall, remove a friend's device when you stop playing together.
The page shows the same policy with a copy button.
