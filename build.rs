//! Build script — embeds Windows icon into the .exe (if icon file exists)

fn main() {
    #[cfg(target_os = "windows")]
    {
        let ico_path = "assets/hardzip.ico";
        if std::path::Path::new(ico_path).exists() {
            let mut res = winresource::WindowsResource::new();
            res.set_icon(ico_path);
            res.set("ProductName", "HardZIP");
            res.set("FileDescription", "HardZIP - Multi-algorithm archiver");
            res.set("CompanyName", "Lukasz Dziubinski");
            res.set("LegalCopyright", "(c) 2024-2026 Lukasz Dziubinski");
            if let Err(e) = res.compile() {
                eprintln!("Warning: Failed to embed icon: {}", e);
            }
        }
    }
}
