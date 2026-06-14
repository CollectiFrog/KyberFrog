// SPDX-License-Identifier: AGPL-3.0-or-later

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // Icon lives in the sibling server crate's assets directory.
        let icon = std::path::Path::new(
            &std::env::var("CARGO_MANIFEST_DIR").unwrap(),
        )
        .parent()
        .unwrap()
        .join("server/assets/kyberfrog.ico");

        let mut res = winresource::WindowsResource::new();
        res.set_icon(&icon.to_string_lossy());
        if let Err(e) = res.compile() {
            eprintln!("cargo:warning=Could not embed icon: {e}");
        }
    }
}
