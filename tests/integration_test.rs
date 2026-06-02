use cargo_metadata::MetadataCommand;
use cargo_uv::Packages;

fn fixture(relative: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(relative)
}

fn packages_from(manifest: &str) -> Packages {
    let metadata = MetadataCommand::new()
        .manifest_path(fixture(manifest))
        .exec()
        .unwrap();
    Packages::from(&metadata)
}

#[test]
fn simple_loads_one_package() {
    let packages = packages_from("simple/Cargo.toml");
    assert_eq!(packages.workspace_members().len(), 1);
    assert!(packages.root_package_name_unchecked().is_some());
    assert_eq!(packages.root_version().unwrap().to_string(), "0.1.11");
}

#[test]
fn mixed_ws_loads_three_packages() {
    let packages = packages_from("mixed_ws/Cargo.toml");
    assert_eq!(packages.workspace_members().len(), 3);
    assert!(packages.root_package_name_unchecked().is_some());
    assert_eq!(
        packages.root_package_name_unchecked().unwrap().as_str(),
        "b"
    );
    assert_eq!(packages.root_version().unwrap().to_string(), "0.1.2");
}

#[test]
fn pure_ws_loads_three_packages_no_root() {
    let packages = packages_from("pure_ws/Cargo.toml");
    assert_eq!(packages.workspace_members().len(), 3);
    assert!(packages.root_package_name_unchecked().is_none());
    // All packages share version 0.1.0, so root_version falls back to the unified value
    assert_eq!(packages.root_version().unwrap().to_string(), "0.1.0");
}

#[test]
fn ws_version_has_workspace_package() {
    let packages = packages_from("ws_version/Cargo.toml");
    assert_eq!(packages.workspace_members().len(), 3);
    assert!(packages.workspace_package().is_some());
    assert_eq!(
        packages.workspace_package().unwrap().version().to_string(),
        "0.2.5-rc.4"
    );
}

#[test]
fn ws_version_mixed_has_workspace_package() {
    let packages = packages_from("ws_version_mixed/Cargo.toml");
    assert_eq!(packages.workspace_members().len(), 3);
    assert!(packages.workspace_package().is_some());
}

#[test]
fn ws_version_nested_has_default_members() {
    let packages = packages_from("ws_version_nested/Cargo.toml");
    assert_eq!(packages.workspace_members().len(), 3);
    // default-members = ["a", "."] which is a and b (the root)
    assert_eq!(packages.workspace_default_members().len(), 2);
}

#[test]
fn display_tree_does_not_panic() {
    for manifest in [
        "simple/Cargo.toml",
        "mixed_ws/Cargo.toml",
        "pure_ws/Cargo.toml",
        "ws_version/Cargo.toml",
        "ws_version_mixed/Cargo.toml",
        "ws_version_nested/Cargo.toml",
    ] {
        let packages = packages_from(manifest);
        let tree = packages.display_tree();
        assert!(!tree.is_empty(), "display_tree empty for {manifest}");
    }
}
