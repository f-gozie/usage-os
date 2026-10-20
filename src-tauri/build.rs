use std::path::Path;

fn main() {
    let mut attributes = tauri_build::Attributes::new();

    // Tauri embeds its Windows manifest (common controls v6) in the app binary only. Test
    // binaries then load comctl32 v5, which has no TaskDialogIndirect, and exit before any
    // test runs with STATUS_ENTRYPOINT_NOT_FOUND. Link the same manifest into every target.
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_os == "windows" && target_env == "msvc" {
        attributes = attributes
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("windows-app-manifest.xml");
        println!("cargo:rerun-if-changed=windows-app-manifest.xml");
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    }

    tauri_build::try_build(attributes).expect("failed to run tauri-build");
}
