fn main() {
    // The window and exe icons are embedded at build time; rebuild when they change.
    println!("cargo:rerun-if-changed=icons");
    tauri_build::build()
}
