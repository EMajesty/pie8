fn main() {
    let id = std::env::var("PIE8_BUILD_ID").unwrap_or_else(|_| "development".into());
    println!("cargo:rustc-env=PIE8_BUILD_ID={id}");
    println!("cargo:rerun-if-env-changed=PIE8_BUILD_ID");
    println!("cargo:rerun-if-changed=src/boot.S");
    println!("cargo:rerun-if-changed=src/vectors.S");
    println!("cargo:rerun-if-changed=linker.ld");
}
