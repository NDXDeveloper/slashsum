fn main() {
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rerun-if-changed=icon.ico");
        
        let mut res = winresource::WindowsResource::new();
        res.set_icon("icon.ico");
        res.set("ProductName", "Slashsum");
        res.set("CompanyName", "NDXDeveloper");
        res.set("LegalCopyright", "© NDXDeveloper");
        res.set("FileDescription", "Calculate multiple checksums simultaneously");
        res.set("ProductVersion", env!("CARGO_PKG_VERSION"));
        res.set("FileVersion", env!("CARGO_PKG_VERSION"));
        
        if let Err(e) = res.compile() {
            println!("cargo:warning=Failed to compile resources: {}", e);
        }
    }
}
