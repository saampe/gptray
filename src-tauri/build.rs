fn main() {
    tauri_build::build();
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        // tauri-winres/embed-resource links Common Controls v6 into binaries,
        // not Cargo examples. The native probe needs the same manifest to load
        // TaskDialogIndirect; reuse the exact resource already built by Tauri.
        let resource = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"))
            .join("resource.lib");
        assert!(resource.is_file(), "Tauri Windows resource missing");
        println!("cargo:rustc-link-arg-examples={}", resource.display());
    }
}
