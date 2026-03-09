fn main() {
    // Add the vendor directory to the library search path for the Vosk native lib.
    let vendor_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("vendor")
        .join("vosk-linux-x86_64-0.3.45");

    if vendor_dir.exists() {
        println!("cargo:rustc-link-search=native={}", vendor_dir.display());
    }
}
