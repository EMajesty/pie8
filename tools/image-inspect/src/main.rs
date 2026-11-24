use std::{env, fs, process};

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: pie8-image-inspect <kernel8.img>");
        process::exit(2);
    });
    let image = fs::read(&path).unwrap_or_else(|error| {
        eprintln!("cannot read {path}: {error}");
        process::exit(1);
    });
    if image.len() < 4 || image.iter().all(|byte| *byte == 0) {
        eprintln!("{path}: invalid empty/zero image");
        process::exit(1);
    }
    println!("{path}: {} bytes, non-empty AArch64 raw image", image.len());
}
