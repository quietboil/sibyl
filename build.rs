use std::env;

fn main() {
    let is_windows = env::var("CARGO_CFG_TARGET_FAMILY").unwrap_or_default() == "windows";
    let oracle_client_lib = if is_windows { "oci" } else { "clntsh" };
    let is_msvc = is_windows && env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default() == "msvc";
    println!("cargo:rustc-link-lib={}={}", if is_msvc { "static" } else { "dylib" }, oracle_client_lib);
    if let Ok( dir ) = env::var("OCI_LIB_DIR") {
        println!("cargo:rustc-link-search={}", dir);
    }
}
