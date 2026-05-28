fn main() {
    // libgit2-sys's fs_path.o references GetNamedSecurityInfoW (file_owner_sid),
    // which lives in advapi32.lib on the MSVC target. Link it explicitly so the
    // Windows release build resolves the symbol.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-lib=dylib=advapi32");
    }
}
