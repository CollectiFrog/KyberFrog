// SPDX-License-Identifier: AGPL-3.0-or-later

use std::process::Command;

fn main() {
    emit_version();
    embed_icon();
    tauri_shell();
}

/// tauri-build: validates `tauri.conf.json` and embeds the Windows app manifest
/// (WebView2 shell, see src/shell/). The `tauri` crate itself is a Windows-only
/// dependency, so this must not run for other targets — where the shell falls
/// back to the headless stub anyway. Its resource file needs an icon: reuse the
/// one embed_icon() already ships instead of the `icons/icon.ico` default.
fn tauri_shell() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let attributes = tauri_build::Attributes::new().windows_attributes(
            tauri_build::WindowsAttributes::new().window_icon_path("assets/kyberfrog.ico"),
        );
        tauri_build::try_build(attributes).expect("tauri-build failed");
    }
}

/// Single source of truth for the displayed/installer version, resolved at build
/// time and exposed as the `KYBERFROG_VERSION` compile env (read with `env!`):
///   1. an explicit `KYBERFROG_VERSION` env — CI / build-installer.sh set it from
///      the git tag; authoritative for releases.
///   2. else `git describe --tags --always --dirty` — the exact tag, or
///      `<tag>-<n>-g<sha>` between tags, for local/dev builds.
///   3. else `v<CARGO_PKG_VERSION>` — last resort (no git, no env).
/// Nothing else stores the version; Cargo.toml's version is only fallback (3).
fn emit_version() {
    println!("cargo:rerun-if-env-changed=KYBERFROG_VERSION");
    // Re-run when HEAD/refs move so dev builds track new commits and tags.
    for p in ["../.git/HEAD", "../.git/packed-refs"] {
        if std::path::Path::new(p).exists() {
            println!("cargo:rerun-if-changed={p}");
        }
    }

    let version = std::env::var("KYBERFROG_VERSION")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(git_describe)
        .unwrap_or_else(|| {
            format!(
                "v{}",
                std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".into())
            )
        });

    println!("cargo:rustc-env=KYBERFROG_VERSION={version}");
}

fn git_describe() -> Option<String> {
    let out = Command::new("git")
        .args(["describe", "--tags", "--always", "--dirty"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let v = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (!v.is_empty()).then_some(v)
}

fn embed_icon() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let icon = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("assets/kyberfrog.ico");

        let mut res = winresource::WindowsResource::new();
        res.set_icon(&icon.to_string_lossy());
        if let Err(e) = res.compile() {
            eprintln!("cargo:warning=Could not embed icon: {e}");
        }
    }
}
