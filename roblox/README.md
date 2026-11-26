# Wout San Retou: Roblox edition

A small, non-violent Roblox version of the game. It is **a new game inspired by the Rust one, not a port**: Roblox does not
take the Rust build, and the Rust game's guns, knife and kills would not suit a Roblox audience. Here you never hurt anyone.

**You are Ro, at night, inside a gang's compound.** Cut the power at three generators, which darkens the yard and opens the
cellblock. Sneak past six guards, call each of the three Lost kids by name (hold E), and lead them out by the road.

| Key | Action |
|---|---|
| W A S D, mouse | move, look |
| Shift (hold) | run (fast, but loud and easy to see) |
| C | sneak on or off (slow, quiet, hard to see) |
| F | flash: blinds guards in front of you for 6 s |
| E (hold) | cut a generator, or call a kid by name |

* **Guards** see in a cone, hear you only while you move, grow suspicious, investigate, chase, and give up. If one catches you,
  you are dragged back to the road and the kids you were leading return to their cells. Nobody is hurt.
* **Weight.** The flash does no damage, but each time it blinds someone you gain 1 Weight. Weight slows you slightly and calls an
  **Echo**, a faint figure that walks toward you; touching one slows you for 3 s. Calling a kid by name lowers Weight by 1.
  The ending text depends on how much Weight you carry out.
* **Up to four players** can play together. Weight is per player; the generators, the door and the kids are shared.

## Open it in Studio

1. Open `WoutSanRetou.rbxlx` (double click) in Roblox Studio. The map is built by the scripts when the game starts, so the file
   itself holds only scripts.
2. Press **Play** (or Play Here). Studio needs internet so the guards, kids and echoes can be built as avatars (if it cannot, they
   appear as plain block people and still work).

Prefer a file-based workflow? `default.project.json` is a [Rojo](https://rojo.space) project: `rojo serve` and connect from
Studio. After editing anything in `src/`, rebuild the Studio file with `python3 tools/build_rbxlx.py`.

## Tuning

Every speed, range, cooldown and the kids' names are in `src/ReplicatedStorage/WSRConfig.lua`.

## What is not in it, on purpose

* **No weapons, kills, blood or gore.** Roblox allows some violence but bans extreme gore and realistic modern firearms, and
  moderates realistic or excessive depictions whatever the maturity label says.
* **No Haitian Vodou imagery and no Kreyol dialogue.** Both need a native or cultural reviewer first, and the Kreyol text in the
  Rust game is still unreviewed. The kids' lines are in English.
* **None of the Rust game's models, music or sounds.** The Mixamo models and the third-party audio are for the Rust build only.
  This version uses Roblox avatars made by the engine and no sound at all, so there are no audio rights to clear.
* **No links, no mention of VowCraft, itch.io, GitHub or payments.** Roblox treats off-platform links and outside promotion as
  advertising. Selling anything inside Roblox would be done with Robux through Roblox's own tools, not the crypto checkout.

## Not tested yet

I could not run Roblox Studio in this environment. The ten Luau files parse cleanly with a plain Lua parser (Luau's own analyzer was not
available, so Roblox API names and types were not checked), and the place file is well-formed XML. **Play it once in Studio and tell me what breaks**: the first things to look at are guard pathfinding around the
crates, the kids keeping up with you, and how the night lighting looks.

## Before you publish it yourself

Publishing needs your Roblox account, so that part is yours.

1. In Studio: File, Publish to Roblox. Name it, set it **Private** first and test it with a friend.
2. Fill in the **Maturity and Compliance questionnaire** honestly: stylised, non-graphic conflict (guards chase and detain you),
   a flashlight-style blinding effect, and crime and gang themes in the setting. Expect a Mild or Moderate style label, and let
   the questionnaire decide.
3. Write the description without links. Keep the poster out of it unless you check the current
   [experience guidelines](https://create.roblox.com/docs/en-us/production/promotion/experience-guidelines) and
   [content maturity](https://create.roblox.com/docs/production/promotion/content-maturity) pages first.
4. Roblox's rules change; read the current [Community Standards](https://about.roblox.com/community-standards) before you go public.
