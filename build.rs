fn main() {
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-arg-bins=/SUBSYSTEM:WINDOWS");
        println!("cargo:rustc-link-arg-bins=/ENTRY:mainCRTStartup");

        // path کامل به assets folder
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let rc_path = format!("{}\\assets\\kalam.rc", manifest_dir);
        
        embed_resource::compile(&rc_path, embed_resource::NONE);
    }
}