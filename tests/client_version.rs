use sibyl::*;

#[test]
fn check_client_version() -> Result<()> {
    let client_version = sibyl::client_version();
    if let Ok(path) = std::env::var("LD_LIBRARY_PATH").or_else(|_| std::env::var("LIBRARY_PATH")) {
        let items : Vec<&str> = path.split('_').collect();
        if items.len() >= 2 {
            let release : i32 = items[items.len() - 2].parse().expect("client release number");
            let update  : i32 = items[items.len() - 1].parse().expect("client release update");
            assert_eq!(release, client_version.0);
            assert_eq!(update, client_version.1);
        }
    } else {
        println!("client version = {client_version:?}");
    }
    Ok(())
}
