use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::env;

fn main() {
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("../../memory_ht32f52352.x"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());

    // Only re-run the build script when memory.x is changed,
    // instead of when any part of the source code changes.
    println!("cargo:rerun-if-changed=../../memory_ht32f52352.x");

    println!("cargo:rustc-link-arg=-Tlink.x");
    // Include defmt.x for proper defmt support
    println!("cargo:rustc-link-arg=-Tdefmt.x");
}