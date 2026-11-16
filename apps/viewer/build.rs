/*
 * Copyright 2026 PyPNMApps contributors
 * SPDX-License-Identifier: GPL-3.0-only
 */
//! Generate shared product constants from the checked-in identity manifest.
use std::{env, fs, path::PathBuf};

/// Validates product identity and writes Rust constants for desktop consumers.
///
/// # Panics
/// Stops compilation if the identity manifest is missing, invalid or incomplete.
fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let source = manifest.join("../../product.json");
    println!("cargo:rerun-if-changed={}", source.display());
    let value: serde_json::Value = serde_json::from_slice(&fs::read(source).unwrap()).unwrap();
    let mut output = String::new();
    for key in [
        "display_name",
        "organization",
        "repository",
        "target_repository",
        "desktop_binary",
        "linux_package",
        "desktop_id",
        "bundle_id",
    ] {
        let text = value[key].as_str().expect("Product field must be a string");
        assert!(
            !text.is_empty() && !text.contains(['\n', '\r']),
            "Invalid product field"
        );
        output.push_str(&format!(
            "/// Product identity from product.json.\npub const {}: &str = {:?};\n",
            key.to_uppercase(),
            text
        ));
    }
    let repository = value["repository"].as_str().unwrap();
    let segments: Vec<_> = repository.split('/').collect();
    assert!(
        segments.len() == 2
            && segments.iter().all(|part| !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))),
        "Invalid GitHub repository"
    );
    for (name, url) in [
        ("PROJECT_URL", format!("https://github.com/{repository}")),
        (
            "RELEASES_URL",
            format!("https://github.com/{repository}/releases"),
        ),
        (
            "RELEASE_API_URL",
            format!("https://api.github.com/repos/{repository}/releases/latest"),
        ),
    ] {
        output.push_str(&format!(
            "/// Canonical URL derived from product.json.\npub const {name}: &str = {url:?};\n"
        ));
    }
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("product_identity.rs"),
        output,
    )
    .unwrap();
}
