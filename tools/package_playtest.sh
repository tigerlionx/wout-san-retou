#!/bin/bash
# Make a folder + zip for playtesters: the release binary, assets, a launcher and the guide.
# Usage: tools/package_playtest.sh [--no-third-party]   (needs `cargo build --release` first)
# --no-third-party leaves out the gunshot/knife/music recordings in assets/audio/local (the game runs quieter, with our own effects only).
set -e
cd "$(dirname "$0")/.."
OUT=dist/wout-san-retou-playtest
rm -rf dist && mkdir -p "$OUT/assets/audio"
cp target/release/wout-san-retou "$OUT/"
cp -R assets/mixamo "$OUT/assets/"
cp -R assets/fonts "$OUT/assets/"
cp -R assets/veve "$OUT/assets/"
cp -R assets/audio/synth "$OUT/assets/audio/"
if [ "$1" != "--no-third-party" ]; then
  mkdir -p "$OUT/assets/audio/local"
  # not the untouched originals, and not the 29 MB listening arrangement
  for f in assets/audio/local/*.wav; do case "$f" in *suspense_showcase*) ;; *) cp "$f" "$OUT/assets/audio/local/";; esac; done
fi
cp design/playtest/TESTER_GUIDE.md "$OUT/"
cat > "$OUT/Play.command" <<'LAUNCH'
#!/bin/bash
cd "$(dirname "$0")"
xattr -dr com.apple.quarantine . 2>/dev/null
chmod +x ./wout-san-retou
./wout-san-retou
LAUNCH
chmod +x "$OUT/Play.command"
(cd dist && zip -qr wout-san-retou-playtest.zip wout-san-retou-playtest)
du -sh dist/wout-san-retou-playtest dist/wout-san-retou-playtest.zip
