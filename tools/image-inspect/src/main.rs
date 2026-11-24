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
    if image.len() > 16 * 1024 * 1024 {
        eprintln!("{path}: unexpectedly large image (over 16 MiB)");
        process::exit(1);
    }
    // `_start` begins with `mrs x0, MPIDR_EL1` (0xd53800a0, little-endian).
    if image[..4] != [0xa0, 0x00, 0x38, 0xd5] {
        eprintln!("{path}: reset vector does not begin with the expected MPIDR read");
        process::exit(1);
    }
    println!("{path}: {} bytes, non-empty AArch64 raw image", image.len());
}
