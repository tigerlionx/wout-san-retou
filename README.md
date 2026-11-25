<p align="center"><img src="promo/wout-san-retou-github-social-1280x640.png" alt="Wout San Retou" width="720"></p>

# Wout San Retou: The Road of No Return

A stealth-horror game about leaving a gang. You play **Ro**, a SWAT officer fighting through a gang's compound, and you find out
that violence has a cost: every kill adds Weight, the dead rise at night, and the "Lost" kids are not shot, they are reached
by name. Seven levels, from the yard to the rooftop, with Kreyol dialogue, Haitian Vodou imagery, and a kingpin who is a fraud.

> **All characters, places and events are fictional.** The game is about gangs, with horror and gunfire (no slow-motion
> kills, no gore as a reward). It is meant for teenagers and older. Haitian Vodou is shown as a religion, never as the villain.
> The Kreyol text still needs review by a native speaker.

<p align="center"><img src="promo/wout-san-retou-poster-1200x1800.png" alt="Poster" width="420"></p>

## Play it

Rust and [Bevy 0.20](https://bevyengine.org/). You need a Rust toolchain.

```bash
cargo run --release        # the first build takes several minutes
```

| Key | Action | Key | Action |
|---|---|---|---|
| W A S D | move | E | use / wake someone by name |
| Shift | run | R | reload |
| C / Z | crouch / prone | G | grenade |
| Space | jump | F | knife |
| Mouse | look and aim | 1 to 4 | weapons |
| Left click | fire | V | first / third person |
| H | drink from a bottle | M | music on/off |
| F1 | hide the controls | F9 | leave a note for the developer |

### Missing files

The repository does **not** include the character models (`assets/mixamo`) or the third-party sound and music
(`assets/audio/local`), which are not ours to redistribute. See [THIRD_PARTY.md](THIRD_PARTY.md). Without them the characters
will be missing and the game will be silent apart from the generated sounds in `assets/audio/synth`.
`tools/make_characters.py` rebuilds the models from Mixamo downloads (Blender needed); `tools/build_audio.py` rebuilds the audio.

## Look at it without playing

`WSR_AUTOPLAY=1 cargo run --release` plays a scripted level offscreen and writes screenshots to `playtest/`
(`WSR_LEVEL2=1` to `WSR_LEVEL6=1` pick a level). `WSR_POSTER=1` renders the key art; see `tools/make_poster.py`.

## Folders

* `src/`: the game. `tools/`: Python and Blender scripts for models, audio, vever sigils and the poster.
* `design/`: the level plans and a Three.js preview of the compound. `promo/`: poster and social image.

## Licence

The code is under the [PolyForm Noncommercial License 1.0.0](LICENSE): you may read, run and change it for noncommercial
purposes. **Commercial use, including selling the game or anything built from it, needs the author's written permission.**
The story, characters, poster and promo images, music and sound recordings are not covered by that licence: all rights in
them are reserved. Fonts and other third-party material have their own terms, see [THIRD_PARTY.md](THIRD_PARTY.md).
