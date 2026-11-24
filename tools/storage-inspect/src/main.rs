use pie8_core::{block::host::ImageBlockDevice, fat32::Fat32, partition::first_fat32, wav::Wave};
use std::{env, fs, process::ExitCode};

fn main() -> ExitCode {
    let args: Vec<_> = env::args().collect();
    let result = match args.as_slice() {
        [_, command, path] if command == "image" => inspect_image(path, None),
        [_, command, path, file] if command == "image" => inspect_image(path, Some(file)),
        [_, command, path] if command == "wav" => inspect_wav(path),
        _ => Err("usage: pie8-storage-inspect image DISK.img [PATH] | wav SAMPLE.wav".into()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
fn inspect_image(path: &str, selected: Option<&String>) -> Result<(), String> {
    let device = ImageBlockDevice::open(path).map_err(|e| e.to_string())?;
    let partition = first_fat32(&device).map_err(|e| format!("{e:?}"))?;
    let fs = Fat32::mount(&device, partition).map_err(|e| format!("{e:?}"))?;
    println!(
        "FAT32 LBA {} sectors {} clusters {}",
        partition.first_lba,
        partition.sectors,
        fs.cluster_count()
    );
    fs.enumerate(None, |entry| {
        println!(
            "{} {} bytes{}",
            entry.name(),
            entry.size,
            if entry.is_directory() {
                " directory"
            } else {
                ""
            }
        );
        true
    })
    .map_err(|e| format!("{e:?}"))?;
    if let Some(path) = selected {
        let entry = fs.find_path(path).map_err(|e| format!("{e:?}"))?;
        println!("selected {} {} bytes", entry.name(), entry.size);
    }
    Ok(())
}
fn inspect_wav(path: &str) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let wave = Wave::parse(&bytes).map_err(|e| format!("{e:?}"))?;
    println!(
        "{} Hz {} channel(s) {}-bit {} frames {} ms",
        wave.sample_rate,
        wave.channels,
        wave.bits_per_sample,
        wave.frame_count,
        wave.duration_ms()
    );
    Ok(())
}
