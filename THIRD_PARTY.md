# Third-party material

What is in the repository, and what is deliberately left out.

## Included

| Material | Where | Terms |
|---|---|---|
| Fira Sans (Bold, Italic, Regular) | `assets/fonts/` | SIL Open Font License 1.1, see `assets/fonts/FiraSans-License.txt` |
| Three.js (design preview) | `design/3d/vendor/` | MIT, see `design/3d/vendor/LICENSE-three` |
| Bevy and other Rust crates | `Cargo.toml` | MIT / Apache-2.0 (see each crate) |
| Synthesised sounds | `assets/audio/synth/` | made for this game by `tools/build_audio.py` |
| Vever sigils | `assets/veve/` | drawn for this game by `tools/make_veve.py` (traditional symbols, redrawn) |

## Packed into the built game, not in the repository (git-ignored)

`build.rs` embeds these into the executable, so they ship inside the game and are not loose, reusable files. They are not in git.

| Material | Where it goes | Why it is not here |
|---|---|---|
| Character models and animations from Mixamo | `assets/mixamo/` | Free from Mixamo (Adobe account needed) and allowed in games. Adobe's rule, as reported on its community forum, is to ship them inside the project and not as loose files, so they are embedded in the executable and the raw files are never committed. Check Adobe's current Mixamo terms before a release. |
| Gunshot and knife recordings, a gunfight bed | `assets/audio/local/` | Recordings from third-party sound libraries. The project owner confirms the right to use these in the game. |
| Suspense music remixed from a Ti Roro track | `assets/audio/local/` | The project owner confirms the right to use this audio in the game. |
| Introduction music remixed from a ritual recording by Jericho 666 | `assets/audio/local/` | The project owner confirms the right to use this audio in the game. |

## Images

The poster and social image in `promo/` are renders of Mixamo-based characters. Treat them as promotional art for this
game only, not as freely reusable assets.
