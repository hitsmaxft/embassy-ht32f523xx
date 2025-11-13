//! Build script for USB Task 4.2 Test
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    // Put the linker script somewhere the linker can find it
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory_ht32f52352.x"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());

    // Re-run if any of these files change
    println!("cargo:rerun-if-changed=memory_ht32f52352.x");
    // Set the linker script to the one provided by cortex-m-rt.
    println!("cargo:rustc-link-arg=-Tlink.x");

    println!("cargo:rustc-link-arg=-Tdefmt.x");
}