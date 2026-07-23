fn main() {
    // Embed the app icon + version resource into the Windows exe.
    // Uses windres (works with both MSVC and mingw cross builds).
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/black-hole-trash.ico");
        res.set("ProductName", "Black Hole Trash");
        res.set(
            "FileDescription",
            "Black Hole Trash - gravitational Windows recycle bin",
        );
        res.set(
            "LegalCopyright",
            "Copyright (c) 2026 GreenScreen410 and Black Hole Trash contributors",
        );
        res.set("OriginalFilename", "BlackHoleTrash.exe");
        res.set("CompanyName", "Black Hole Trash");
        if let Err(e) = res.compile() {
            // Don't fail the build over a missing windres - just skip the icon.
            println!("cargo:warning=icon resource skipped: {e}");
        }
    }
}
