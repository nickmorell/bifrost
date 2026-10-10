//! Gives the Windows program BiFrost's icon and name, so Explorer, the taskbar and Task
//! Manager show them. Other systems take the icon from the app bundle (macOS) or the
//! window (Linux) instead; see docs/brand.md.

fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/brand/bifrost.ico");
    windows_resources()
}

/// Compiles the icon and name into the program when building for Windows on Windows.
#[cfg(windows)]
fn windows_resources() -> std::io::Result<()> {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/brand/bifrost.ico")
            .set("ProductName", "BiFrost")
            .set("FileDescription", "BiFrost")
            .compile()?;
    }
    Ok(())
}

#[cfg(not(windows))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "returns what the Windows version returns"
)]
fn windows_resources() -> std::io::Result<()> {
    Ok(())
}
