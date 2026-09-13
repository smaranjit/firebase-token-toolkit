//! Embeds the icon and version metadata into the Windows executable.
//!
//! No-op for every other target. Gating is on CARGO_CFG_TARGET_OS rather than
//! `cfg!(windows)` because build scripts are compiled for the host, so the
//! latter reports the wrong answer whenever this is cross-compiled.

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=build.rs");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/icon.ico");
    res.set("ProductName", "Firebase Token Toolkit");
    res.set("FileDescription", "Firebase Token Toolkit");
    res.set("LegalCopyright", "Copyright (c) 2026 Smaranjit Maiti");

    // A missing resource compiler is normal when cross-compiling from Linux, and
    // it only costs us the icon — so warn loudly instead of failing the build.
    if let Err(err) = res.compile() {
        println!(
            "cargo:warning=could not embed Windows resources ({err}); the .exe will have no icon"
        );
    }
}
