fn main() {
    if std::env::var_os("CARGO_FEATURE_EMBED_WEB").is_none() {
        return;
    }

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let web_dir = std::path::Path::new(&manifest_dir).join("../web");

    let status = std::process::Command::new("trunk")
        .args(["build", "--release"])
        .current_dir(&web_dir)
        .status()
        .expect("failed to run `trunk build --release` — is trunk installed?");

    assert!(status.success(), "trunk build --release failed");
}
