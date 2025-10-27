fn main() {
    // Tell Cargo about the custom cfg conditions we'll be using
    println!("cargo:rustc-check-cfg=cfg(flash_size_64k)");
    println!("cargo:rustc-check-cfg=cfg(flash_size_128k)");
    println!("cargo:rustc-check-cfg=cfg(ram_size_8k)");
    println!("cargo:rustc-check-cfg=cfg(ram_size_16k)");
    // Determine which memory layout to use and provide chip information
    let (memory_file, chip_info) = if cfg!(feature = "ht32f52342") {
        ("memory_ht32f52342.x", "HT32F52342: 64KB Flash, 8KB RAM")
    } else if cfg!(feature = "ht32f52352") {
        ("memory_ht32f52352.x", "HT32F52352: 128KB Flash, 16KB RAM")
    } else {
        // Default to larger chip for safety
        ("memory_ht32f52352.x", "HT32F52352: 128KB Flash, 16KB RAM (default)")
    };

    // Tell user which chip configuration is being used
    println!("cargo:warning=Building for {}", chip_info);

    // Configure linker to use the selected memory layout directly
    println!("cargo:rustc-link-arg=-T{}", memory_file);

    // Tell cargo where to find the memory layout files
    println!("cargo:rustc-link-search=.");

    // Rebuild if memory layout files change
    println!("cargo:rerun-if-changed=memory_ht32f52342.x");
    println!("cargo:rerun-if-changed=memory_ht32f52352.x");

    // Emit detailed chip configuration for conditional compilation
    if cfg!(feature = "ht32f52342") {
        println!("cargo:rustc-cfg=chip=\"ht32f52342\"");
        println!("cargo:rustc-cfg=flash_size_64k");
        println!("cargo:rustc-cfg=ram_size_8k");
    } else {
        println!("cargo:rustc-cfg=chip=\"ht32f52352\"");
        println!("cargo:rustc-cfg=flash_size_128k");
        println!("cargo:rustc-cfg=ram_size_16k");
    }
}