# Modded multiplayer (custom maps with friends)

The **Modded multi** page (`src/features/multi`, backend `src-tauri/src/multi`)
hosts or joins a custom-map game between friends. The game runs **without
Easy Anti-Cheat** (offline mode), so this never touches online play.

## How it works

| Piece | Role | Who installs it |
|---|---|---|
| Tailscale | Private network between the players (`100.x.y.z`), no port forwarding | each player |
| BakkesMod + Rocket Plugin | Rocket Plugin hosts (`open <arena>?…?Lan?Listen`, port 7777) and joins (`start ip:port/?Lan`) | each player |
| ALXS-RL-Mod | installs the map, starts the game (no EAC) and BakkesMod's injector, sends the Rocket Plugin command | each player |

Every player needs the **same map file** on the **same Labs arena**: the host
opens the arena's level name (`rp host Labs_Underpass_P`), and our map is
written over that arena (`maps::session`). The invite code carries the
arena, the map's SHA-256 and its bakkesplugins id (downloaded automatically
on the joiner's side when missing).

### Steps (`multi::session`)

1. Map: join resolves it in the library (bakkesplugins id, then name), else
   downloads it; then `maps::session::activate` (same write layer as Maps).
2. BakkesMod RCON (`multi::bakkes_cfg`), only while the game is closed:
   - `cfg/autoexec.cfg`: a marked block `rcon_enabled "1"` + a random
     `rcon_password`;
   - `data/rcon_commands.cfg` (whitelist, first word): `rp`.
   "Remove" in the checklist deletes both (the `rp` line only if we added it).
3. `RocketLeague.exe` (no EAC), then `BakkesMod.exe` if it is not running.
   The app never injects anything itself.
4. Waits (4 min max) for `ws://127.0.0.1:<rcon_port>` (default 9002), sends
   `rcon_password <pw>` then `rp host <arena>` / `rp join <ip> <port>`.
5. If RCON never answers (game started before the setup, BakkesMod not
   injected, protocol change…), the session ends in `manual` and the UI
   shows the command to type in the BakkesMod console (F6).

Invite code: `ALXS-MULTI-` + base64url(JSON `{v, host, port, hostName, map}`),
strictly validated (`multi::invite::decode`): the host is an IPv4, the arena
a known Labs file, the map id alphanumeric — nothing free-form reaches the
console command.

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
