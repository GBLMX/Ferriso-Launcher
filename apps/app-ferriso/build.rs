fn main() {
    println!("cargo:rerun-if-env-changed=FERRISO_VERSION");
    println!("cargo:rerun-if-changed=packaging/ferriso-launcher.rc");
    println!("cargo:rerun-if-changed=packaging/ferriso-launcher.ico");

    #[cfg(windows)]
    {
        embed_resource::compile(
            "packaging/ferriso-launcher.rc",
            embed_resource::NONE,
        );
    }
}
