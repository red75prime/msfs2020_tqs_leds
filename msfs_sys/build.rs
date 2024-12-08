use windows_registry::LOCAL_MACHINE;

fn main() {
    let folders = LOCAL_MACHINE.open(r#"SOFTWARE\Microsoft\Windows\CurrentVersion\Installer\Folders"#).expect("Cannot open");
    let (static_lib_path, _) =
        folders.values()
        .expect("Cannot read values")
        .find(|(name, _)| {
            name.ends_with(r#"SimConnect SDK\lib\static\"#)
        }).expect("SimConnect SDK not found");
    let path = std::path::PathBuf::from(static_lib_path);
    println!("cargo:rustc-link-search=native={}", path.display());
    println!("cargo:rustc-link-lib=SimConnect");
    println!("cargo:rustc-link-lib=Shell32");
    println!("cargo:rustc-link-lib=User32");
    println!("cargo:rustc-link-lib=Shlwapi");
}

