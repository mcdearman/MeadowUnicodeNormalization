//! Finds the `unicode-normalization` source that Cargo fetched: its
//! algorithms, which `main.rs` fingerprints since `src/` ports them by hand,
//! and the size of its composition table, which `main.rs` checks the
//! composites it finds against.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let mut sources = String::new();
    let dir = upstream_dir("unicode-normalization");
    println!("cargo:rustc-env=UPSTREAM_DIR={}", dir.display());
    for file in [
        "src/decompose.rs",
        "src/lib.rs",
        "src/lookups.rs",
        "src/normalize.rs",
        "src/quick_check.rs",
        "src/recompose.rs",
        "src/replace.rs",
        "src/stream_safe.rs",
    ] {
        let path = dir.join(file);
        sources.push_str(
            &std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("could not read {}: {e}", path.display())),
        );
        println!("cargo:rerun-if-changed={}", path.display());
    }

    // `COMPOSITION_TABLE_KV` has one `(0x…, '\u{…}')` entry per line, and
    // `composition_table_astral` one `(0x…, 0x…) => Some(…)` arm per line.
    let tables = std::fs::read_to_string(dir.join("src/tables.rs")).unwrap();
    let kv_start = tables.find("COMPOSITION_TABLE_KV").unwrap();
    let kv = &tables[kv_start..kv_start + tables[kv_start..].find("];").unwrap()];
    let astral_start = tables.find("fn composition_table_astral").unwrap();
    let astral = &tables[astral_start..astral_start + tables[astral_start..].find("\n}").unwrap()];
    let count = kv.matches("(0x").count() + astral.matches("=> Some(").count();
    std::fs::write(out.join("composition_count.txt"), count.to_string()).unwrap();
    std::fs::write(out.join("sources.rs.txt"), sources).unwrap();
    println!("cargo:rerun-if-changed=Cargo.toml");
}

/// Where Cargo put the package `name` this build depends on.
fn upstream_dir(name: &str) -> PathBuf {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let manifest = Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("Cargo.toml");
    let out = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(&manifest)
        .output()
        .expect("could not run `cargo metadata`");
    assert!(out.status.success(), "`cargo metadata` failed");
    let meta: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let pkg = meta["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == name)
        .unwrap_or_else(|| panic!("{name} is not among the dependencies"));
    Path::new(pkg["manifest_path"].as_str().unwrap())
        .parent()
        .unwrap()
        .to_path_buf()
}
