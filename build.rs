//! Embeds every file under `assets/` into the binary and generates a lookup
//! function keyed by the asset's path relative to `assets/` (with forward
//! slashes), e.g. `windowsIcons/start.png`.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let assets = Path::new(&manifest_dir).join("assets");

    let mut keys: Vec<String> = Vec::new();
    walk(&assets, &assets, &mut keys);
    keys.sort();

    let mut code = String::new();
    code.push_str("/// Look up an embedded asset by its path relative to `assets/`.\n");
    code.push_str("pub fn asset_bytes(path: &str) -> Option<&'static [u8]> {\n");
    code.push_str("    match path {\n");
    for key in &keys {
        // Skip fonts; they are loaded directly via include_bytes! in fonts.rs.
        if key.starts_with("fonts/") {
            continue;
        }
        code.push_str(&format!(
            "        {key:?} => Some(include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/assets/{key}\"))),\n"
        ));
    }
    code.push_str("        _ => None,\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");
    code.push_str("/// Every embedded asset key, sorted.\n");
    code.push_str("pub const ASSET_KEYS: &[&str] = &[\n");
    for key in &keys {
        if key.starts_with("fonts/") {
            continue;
        }
        code.push_str(&format!("    {key:?},\n"));
    }
    code.push_str("];\n");

    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("assets_gen.rs");
    fs::write(out, code).unwrap();

    println!("cargo:rerun-if-changed=assets");
    println!("cargo:rerun-if-changed=build.rs");
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(read) = fs::read_dir(dir) else { return };
    for entry in read.flatten() {
        let path: PathBuf = entry.path();
        if path.is_dir() {
            walk(root, &path, out);
        } else if let Ok(rel) = path.strip_prefix(root) {
            let rel = rel.to_string_lossy().replace('\\', "/");
            out.push(rel);
        }
    }
}
