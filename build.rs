use std::{env, fs, ffi::OsString, path::PathBuf, process::exit};

fn main() {
    let is_windows = env::var("CARGO_CFG_TARGET_FAMILY").unwrap_or_default() == "windows";
    if is_windows {
        let is_msvc = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default() == "msvc";

        if let Ok( dir ) = env::var("OCI_LIB_DIR") {
            println!("cargo:rustc-link-search={}", dir);
        } else if is_msvc {
            // It needs OCI.LIB, thus OCI_LIB_DIR is mandatory
            eprintln!("OCI_LIB_DIR must be specified for \"msvc\" targets");
            exit(1);
        } else if let Some( dir ) = env::var_os("PATH").and_then(find_dir_with_oci_dll) {            
            // Target is GNU. It needs OCI.DLL, which might be on the PATH
            println!("cargo:rustc-link-search={}", dir.display());
        } else {
            eprintln!("OCI_LIB_DIR must be specified for \"gnu\" targets when OCI.DLL is not on the PATH");
            exit(1);
        }
        println!("cargo:rustc-link-lib={}=oci", if is_msvc { "static" } else { "dylib" });
    } else {
        println!("cargo:rustc-link-lib=dylib=clntsh");
    }
}

fn find_dir_with_oci_dll(path: OsString) -> Option<PathBuf> {
    env::split_paths(&path).find(|dir| has_oci_dll(dir))
}

fn has_oci_dll(dir: &PathBuf) -> bool {
    dir.read_dir().ok().and_then(check_dir_for_oci_dll).unwrap_or_default()
}

fn check_dir_for_oci_dll(mut files: fs::ReadDir) -> Option<bool> {
    files.find_map(|res| 
        res.ok().map(|entry| 
            entry.file_name().eq_ignore_ascii_case("OCI.DLL")
        )
    )
}