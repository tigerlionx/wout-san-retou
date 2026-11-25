//! Packs the game's assets into the executable, so a built game is one file and the models and sounds are not loose files
//! lying in a folder. Whatever exists under `assets/` at build time is embedded (character models, music, sounds, fonts,
//! sigils). Files that are missing (a fresh clone has no Mixamo models or third-party audio) are simply not embedded, and
//! the game then falls back to the `assets/` folder at run time, as before.
//! Skipped on purpose: `assets/audio/local/orig` and the listening arrangement `suspense_showcase.wav` (not used in game).

use std::{env, fs, path::{Path, PathBuf}};

const SKIP: &[&str] = &["audio/local/orig", "audio/local/suspense_showcase.wav", "audio/README.md"];

fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let p = e.path();
        let rel = p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
        if SKIP.iter().any(|s| rel == *s) || rel.starts_with('.') || rel.ends_with(".DS_Store") {
            continue;
        }
        if p.is_dir() {
            walk(root, &p, out);
        } else {
            out.push((rel, p));
        }
    }
}

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let assets = manifest.join("assets");
    let mut files = Vec::new();
    walk(&assets, &assets, &mut files);

    let mut src = String::from("pub static EMBEDDED: &[(&str, &[u8])] = &[\n");
    for (rel, path) in &files {
        src += &format!("    ({:?}, include_bytes!({:?})),\n", rel, path.to_string_lossy());
    }
    src += "];\n";
    fs::write(PathBuf::from(env::var("OUT_DIR").unwrap()).join("embedded_assets.rs"), src).unwrap();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets");
    for (_, path) in &files {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    println!("cargo:warning=embedding {} asset files", files.len());
}
