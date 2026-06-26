fn main() {
    tauri_build::build();

    // tauri_build doesn't track icons/, but generate_context!() embeds them.
    // Re-run when icons change so the bundle icon stays current.
    println!("cargo:rerun-if-changed=icons");
}
