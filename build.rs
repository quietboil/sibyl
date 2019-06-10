use build_helper::{host, windows, rustc, LibKind, SearchKind};
use std::env;

fn main() {
    let oracle_client_lib = if windows() { "oci" } else { "clntsh" };
    rustc::link_lib(Some(LibKind::DyLib), oracle_client_lib);
    
    if host().os() == "windows" {
        if let Some( path ) = env::var_os("PATH") {
            for dir in env::split_paths(&path) {
                if has_oci_dll(&dir) {
                    rustc::link_search(Some(SearchKind::Native), &dir);
                }
            }
        }
    }
}

fn has_oci_dll(dir: &std::path::PathBuf) -> bool {
    if let Ok( iter ) = dir.read_dir() {
        for entry in iter {
            if let Ok( file ) = entry {
                if let Some( name ) = file.file_name().to_str() {
                    if name.to_lowercase() == "oci.dll" {
                        return true;
                    }
                }
            }
        }
    }
    false
}
