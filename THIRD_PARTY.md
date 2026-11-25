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

## Left out on purpose (git-ignored)

| Material | Where it goes | Why it is not here |
|---|---|---|
| Character models and animations from Mixamo | `assets/mixamo/` | Mixamo content is licensed by Adobe for use in your own projects; check their terms before redistributing, and do not publish the raw files. |
| Gunshot and knife recordings, a gunfight bed | `assets/audio/local/` | Recordings from third-party sound libraries. Check each licence. |
| Suspense music remixed from a Ti Roro track | `assets/audio/local/` | Needs the rights holder's permission before any release. |
| Introduction music remixed from a ritual recording by Jericho 666 | `assets/audio/local/` | Needs the rights holder's permission before any release. |

## Images

The poster and social image in `promo/` are renders of Mixamo-based characters. Treat them as promotional art for this
game only, not as freely reusable assets.
