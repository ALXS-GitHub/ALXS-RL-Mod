fn main() {
    // Keep the development executable's Windows resource in sync with the
    // bundled icon when the artwork changes.
    println!("cargo:rerun-if-changed=icons/icon.ico");
    tauri_build::build()
}
